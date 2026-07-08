use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use chrono::Utc;

use crate::gates::{run_css_naming, run_css_visual, GateCategory};
use crate::log::LoopLogger;
use crate::paths::{acceptance_dir, css_optimize_run_dir, css_visual_artifacts_dir, repo_root};
use crate::server::E2eServer;

pub struct CssOptimizeOptions {
    pub max_iterations: u32,
    pub delay_seconds: u64,
    pub skip_visual: bool,
    pub e2e_port: u16,
}

pub fn run(opts: CssOptimizeOptions) -> anyhow::Result<i32> {
    let root = repo_root();
    let run_dir = css_optimize_run_dir(&root);
    std::fs::create_dir_all(&run_dir)?;

    let acceptance = acceptance_dir().join("css-optimize-acceptance.md");
    let agent_rules = acceptance_dir().join("css-optimize-agent-prompt.md");
    if !acceptance.exists() || !agent_rules.exists() {
        anyhow::bail!("missing acceptance docs under crates/loops/acceptance/");
    }

    let agent = default_agent_cmd();
    if !agent.exists() {
        anyhow::bail!(
            "Cursor agent CLI not found at {}. Install cursor-agent.",
            agent.display()
        );
    }

    let visual_dir = css_visual_artifacts_dir(&root);
    let naming_prompt = format!(
        "Read crates/loops/acceptance/css-optimize-acceptance.md and css-optimize-agent-prompt.md. \
         Fix STATIC naming violations (gate css-naming). One iteration, max 8 files. Work in {}.",
        root.display()
    );
    let visual_prompt = format!(
        "Read crates/loops/acceptance/css-optimize-acceptance.md section V and css-optimize-agent-prompt.md section 3. \
         Fix VISUAL/layout issues using screenshots in {}/. One iteration, max 8 files. Work in {}.",
        visual_dir.display(),
        root.display()
    );

    let _server = if opts.skip_visual {
        None
    } else {
        Some(E2eServer::start(&root, opts.e2e_port)?)
    };
    if let Some(ref server) = _server {
        server.apply_env();
    }

    println!("CSS optimize loop (burncloud-loop)");
    println!("  Acceptance : {}", acceptance.display());
    println!("  Artifacts  : {}", run_dir.display());
    println!("  Visual dir : {}", visual_dir.display());
    if opts.skip_visual {
        println!("  Visual     : SKIPPED");
    }
    println!("  Max rounds : {}", opts.max_iterations);
    println!();

    for i in 1..=opts.max_iterations {
        println!("========== CSS iteration {i} / {} ==========", opts.max_iterations);
        let check_log = run_dir.join(format!("loop-check-{i}.log"));
        let mut logger = LoopLogger::for_iteration(i, Some(&check_log))?;

        let (naming_ok, _, _) = logger.timed_gate(GateCategory::CssNaming, || run_css_naming(&root));

        let visual_ok = if opts.skip_visual {
            true
        } else if naming_ok {
            let (ok, _, _) = logger.timed_gate(GateCategory::CssVisual, || run_css_visual(&root));
            ok
        } else {
            println!("Skipping visual check (naming failed).");
            false
        };

        if naming_ok && visual_ok {
            println!();
            println!("Done: all CSS acceptance checks passed (naming + visual).");
            return Ok(0);
        }

        let agent_prompt = if !naming_ok {
            println!("Naming violations; invoking agent...");
            naming_prompt.clone()
        } else {
            println!("Visual violations; invoking agent...");
            visual_prompt.clone()
        };

        let status = Command::new(&agent)
            .args(["-p", "-f", "--trust", "--workspace"])
            .arg(&root)
            .arg(&agent_prompt)
            .status()?;
        if !status.success() {
            eprintln!("Warning: agent exited with {status}");
        }

        if i < opts.max_iterations {
            println!("Waiting {}s before re-check...", opts.delay_seconds);
            thread::sleep(Duration::from_secs(opts.delay_seconds));
        }
    }

    let state_path = run_dir.join("loop-state.json");
    let state = serde_json::json!({
        "loop": "css-optimize",
        "phase": "max-iterations",
        "updated_at": Utc::now().to_rfc3339(),
    });
    std::fs::write(state_path, serde_json::to_string_pretty(&state)?)?;

    println!();
    println!("Stopped: max iterations ({}).", opts.max_iterations);
    Ok(1)
}

fn default_agent_cmd() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(|appdata| PathBuf::from(appdata).join("cursor-agent").join("agent.cmd"))
        .unwrap_or_else(|_| PathBuf::from("agent.cmd"))
}
