use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use chrono::Utc;

use crate::gates::{
    ensure_api_tests_built, run_jobs_fast_gates, timings_from_log_lines, ReviewScope,
};
use crate::lock::LoopRunLock;
use crate::log::LoopLogger;
use crate::page_progress::PageProgress;
use crate::paths::{acceptance_dir, jobs_aesthetic_run_dir, repo_root};
use crate::process::cleanup_stale_e2e_processes;
use crate::prompt::jobs_aesthetic::{build_jobs_aesthetic_prompt, PromptInput};
use crate::server::E2eServer;
use crate::state::{next_action_from_phase, phase_from_gates, write_loop_state, LoopState};

pub struct JobsAestheticOptions {
    pub max_iterations: u32,
    pub delay_seconds: u64,
    pub check_only: bool,
    pub full_css_gate: bool,
    pub e2e_port: u16,
    pub reset_page_progress: bool,
    /// Limit loop to these page keys (e.g. aesthetic-home only).
    pub only_pages: Vec<String>,
}

pub fn run(opts: JobsAestheticOptions) -> anyhow::Result<i32> {
    let root = repo_root();
    let run_dir = jobs_aesthetic_run_dir(&root);
    std::fs::create_dir_all(&run_dir)?;
    let state_path = run_dir.join("loop-state.json");
    let progress_path = PageProgress::progress_path(&run_dir);

    for required in [
        acceptance_dir().join("aesthetic-optimize-acceptance.md"),
        acceptance_dir().join("jobs-aesthetic-agent-prompt.md"),
    ] {
        if !required.exists() {
            anyhow::bail!("missing required file: {}", required.display());
        }
    }

    if !opts.check_only {
        let agent = default_agent_cmd();
        if !agent.exists() {
            anyhow::bail!(
                "Cursor agent CLI not found at {}. Use --check-only or install cursor-agent.",
                agent.display()
            );
        }
    }

    std::env::set_var("E2E_USE_PREVIEW", "1");

    let _run_lock = LoopRunLock::acquire(&run_dir)?;

    let scope = if opts.only_pages.is_empty() {
        None
    } else {
        Some(opts.only_pages.clone())
    };

    let mut progress = if opts.reset_page_progress {
        PageProgress::with_scope(scope.clone())
    } else {
        PageProgress::load(&progress_path)
    };
    if let Some(ref pages) = scope {
        progress.apply_scope(pages.clone())?;
    }
    progress.save(&progress_path)?;

    cleanup_stale_e2e_processes();
    eprintln!("Pre-building api_tests (one-time per loop session)...");
    let prebuild_start = std::time::Instant::now();
    ensure_api_tests_built(&root)?;
    eprintln!(
        "  api_tests ready in {:.1}s",
        prebuild_start.elapsed().as_secs_f64()
    );

    let server = E2eServer::start(&root, opts.e2e_port)?;
    server.apply_env();

    println!("Jobs aesthetic layout loop (burncloud-loop)");
    println!("  Goal       : one page at a time → J1 + J3 (ge 4) + final J4");
    println!(
        "  Acceptance : {}",
        acceptance_dir()
            .join("aesthetic-optimize-acceptance.md")
            .display()
    );
    println!("  Run data   : {}", run_dir.display());
    println!("  Max rounds : {}", opts.max_iterations);
    println!(
        "  Fast mode  : {} (naming + preview aesthetic, no 11-page css_visual)",
        !opts.full_css_gate
    );
    println!("  E2E server : {} (persistent)", server.base_url);
    if let Some(ref pages) = progress.active_pages {
        println!("  Scope      : {} (pilot — one page at a time)", pages.join(", "));
    }
    println!(
        "  Page queue : {} done, current={}, {} remaining",
        progress.completed_pages.len(),
        progress.current_page,
        progress.remaining_count()
    );
    if !progress.completed_pages.is_empty() {
        println!(
            "  Completed  : {}",
            progress.completed_pages.join(", ")
        );
    }
    if opts.check_only {
        println!("  Mode       : check only (no agent)");
    }
    println!();

    if progress.all_complete() {
        println!("All pages already marked complete in page-progress.json.");
        return Ok(0);
    }

    for i in 1..=opts.max_iterations {
        if progress.all_complete() {
            println!("All pages complete.");
            return Ok(0);
        }

        let review_scope = ReviewScope {
            current_page: progress.current_page.clone(),
            completed_pages: progress.completed_pages.clone(),
            require_global_j4: progress.is_last_page(),
        };

        let iter_start = std::time::Instant::now();
        println!(
            "========== Jobs layout iteration {i} / {} (page: {}) ==========",
            opts.max_iterations, progress.current_page
        );

        let check_log = run_dir.join(format!("loop-check-{i}.log"));
        let mut logger = LoopLogger::for_iteration(i, Some(&check_log))?;

        let gates = run_jobs_fast_gates(&root, opts.full_css_gate, Some(&review_scope), &mut logger);
        let phase = phase_from_gates(gates.css_ok, gates.metrics_ok, gates.review_ok);
        let elapsed = iter_start.elapsed().as_secs();
        println!(
            "  Iteration {i} checks finished in {elapsed}s (css={} metrics={} review={})",
            gates.css_ok, gates.metrics_ok, gates.review_ok
        );
        for r in &gates.results {
            println!(
                "    {} {} {:.1}s",
                if r.passed { "PASS" } else { "FAIL" },
                r.category.log_label(),
                r.elapsed_secs
            );
        }

        if check_log.exists() {
            let log_text = std::fs::read_to_string(&check_log).unwrap_or_default();
            let log_lines: Vec<String> = log_text.lines().map(|s| s.to_string()).collect();
            let timings = timings_from_log_lines(&log_lines);
            let timings_path = run_dir.join(format!("timings-{i}.json"));
            let payload = serde_json::json!({
                "iteration": i,
                "focus_page": progress.current_page,
                "completed_pages": progress.completed_pages,
                "wall_secs": elapsed,
                "gates": gates.results.iter().map(|r| serde_json::json!({
                    "gate": r.category.log_label(),
                    "passed": r.passed,
                    "secs": r.elapsed_secs,
                })).collect::<Vec<_>>(),
                "e2e": timings,
            });
            let _ = std::fs::write(&timings_path, serde_json::to_string_pretty(&payload).unwrap_or_default());
            println!("  Timings log : {}", timings_path.display());
        }

        let agent_prompt_path = build_jobs_aesthetic_prompt(&PromptInput {
            iteration: i,
            phase: phase.to_string(),
            css_ok: gates.css_ok,
            metrics_ok: gates.metrics_ok,
            review_ok: gates.review_ok,
            focus_page: progress.current_page.clone(),
            completed_pages: progress.completed_pages.clone(),
            check_log: Some(check_log.clone()),
            root: root.clone(),
        })?;

        write_loop_state(
            &state_path,
            &LoopState {
                loop_name: "jobs-aesthetic".to_string(),
                iteration: i,
                max_iterations: opts.max_iterations,
                phase: if gates.css_ok && gates.metrics_ok && gates.review_ok {
                    if progress.all_complete() {
                        "done".to_string()
                    } else {
                        "page-done".to_string()
                    }
                } else {
                    phase.to_string()
                },
                css_ok: gates.css_ok,
                metrics_ok: gates.metrics_ok,
                review_ok: gates.review_ok,
                next_action: next_action_from_phase(phase).to_string(),
                agent_prompt: agent_prompt_path.display().to_string(),
                fast_mode: !opts.full_css_gate,
                preview_routes: true,
                pages: progress.page_order().iter().map(|s| s.to_string()).collect(),
                current_page: Some(progress.current_page.clone()),
                completed_pages: progress.completed_pages.clone(),
                pages_remaining: Some(progress.remaining_count()),
                updated_at: Utc::now().to_rfc3339(),
            },
        )?;

        if gates.css_ok && gates.metrics_ok && gates.review_ok {
            if let Some(done) = progress.complete_current() {
                progress.save(&progress_path)?;
                println!();
                println!("  Page '{done}' passed. Observers can preview it in /preview/* routes.");
                if progress.all_complete() {
                    println!("Done: all pages passed Jobs aesthetic layout.");
                    return Ok(0);
                }
                println!(
                    "  Advanced to next page: {} ({} remaining). Skipping agent this round.",
                    progress.current_page,
                    progress.remaining_count()
                );
                if i < opts.max_iterations {
                    println!("Waiting {}s before next page check...", opts.delay_seconds);
                    thread::sleep(Duration::from_secs(opts.delay_seconds));
                }
                continue;
            }
        }

        if opts.check_only {
            println!();
            println!("CheckOnly: stopping after diagnostics (phase={phase}, page={}).", progress.current_page);
            println!("  Prompt preview: {}", agent_prompt_path.display());
            println!("  Gate log: {}", check_log.display());
            return Ok(1);
        }

        println!();
        match phase {
            "css" => println!("Phase: CSS baseline — agent fixes shared CSS only..."),
            "metrics" => println!(
                "Phase: J1 metrics — agent fixes {} only...",
                progress.current_page
            ),
            _ => println!(
                "Phase: J3 review — agent polishes {} only...",
                progress.current_page
            ),
        }

        println!("  Agent prompt: {}", agent_prompt_path.display());
        let agent = default_agent_cmd();
        let status = Command::new(&agent)
            .args(["-p", "-f", "--trust", "--workspace"])
            .arg(&root)
            .arg(&agent_prompt_path)
            .status()?;
        if !status.success() {
            eprintln!("Warning: agent exited with {status}");
        }

        if i < opts.max_iterations {
            println!("Waiting {}s before re-check...", opts.delay_seconds);
            thread::sleep(Duration::from_secs(opts.delay_seconds));
        }
    }

    write_loop_state(
        &state_path,
        &LoopState {
            loop_name: "jobs-aesthetic".to_string(),
            iteration: opts.max_iterations,
            max_iterations: opts.max_iterations,
            phase: "max-iterations".to_string(),
            css_ok: false,
            metrics_ok: false,
            review_ok: false,
            next_action: "increase-MaxIterations-or-fix-manually".to_string(),
            agent_prompt: run_dir.join("agent-prompt.md").display().to_string(),
            fast_mode: !opts.full_css_gate,
            preview_routes: true,
            pages: progress.page_order().iter().map(|s| s.to_string()).collect(),
            current_page: Some(progress.current_page.clone()),
            completed_pages: progress.completed_pages.clone(),
            pages_remaining: Some(progress.remaining_count()),
            updated_at: Utc::now().to_rfc3339(),
        },
    )?;
    println!();
    println!(
        "Stopped: max iterations ({}). Current page: {}. See {}",
        opts.max_iterations,
        progress.current_page,
        state_path.display()
    );
    Ok(1)
}

fn default_agent_cmd() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(|appdata| PathBuf::from(appdata).join("cursor-agent").join("agent.cmd"))
        .unwrap_or_else(|_| PathBuf::from("agent.cmd"))
}
