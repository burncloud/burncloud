use clap::{Parser, Subcommand};

use crate::gates::{parse_gate_name, parse_gate_suite, run_gate_suite, run_single_gate, GateCategory};
use crate::log::LoopLogger;
use crate::loops::css_optimize::{run as run_css_optimize, CssOptimizeOptions};
use crate::loops::jobs_aesthetic::{run as run_jobs_aesthetic, JobsAestheticOptions};
use crate::paths::{jobs_aesthetic_run_dir, repo_root};

#[derive(Parser)]
#[command(
    name = "burncloud-loop",
    about = "Agent-driven UI optimization loops with categorized gates and structured logs"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Jobs aesthetic loop (J1 + J3 + J4)
    JobsAesthetic {
        #[arg(long, default_value_t = 30)]
        max_iterations: u32,
        #[arg(long, default_value_t = 2)]
        delay_seconds: u64,
        #[arg(long)]
        check_only: bool,
        /// Include 11-page css_visual each round (slow)
        #[arg(long)]
        full_css_gate: bool,
        #[arg(long, default_value_t = 3099)]
        e2e_port: u16,
        /// Reset page-progress.json and start from the first page in scope
        #[arg(long)]
        reset_page_progress: bool,
        /// Only optimize these pages (repeatable). Example: --only-page aesthetic-home
        #[arg(long = "only-page", value_name = "PAGE_KEY")]
        only_pages: Vec<String>,
    },
    /// CSS naming + visual loop
    CssOptimize {
        #[arg(long, default_value_t = 30)]
        max_iterations: u32,
        #[arg(long, default_value_t = 10)]
        delay_seconds: u64,
        #[arg(long)]
        skip_visual: bool,
        #[arg(long, default_value_t = 3099)]
        e2e_port: u16,
    },
    /// Run a single gate category (debug / CI slice)
    Gate {
        /// css-naming | css-all | css-visual | aesthetic-metrics | aesthetic-review
        name: String,
        #[arg(long)]
        verbose: bool,
    },
    /// Run a predefined gate suite
    Gates {
        /// jobs-fast | jobs-full | css-full | aesthetic-full
        suite: String,
        #[arg(long)]
        verbose: bool,
    },
    /// List gate categories
    ListGates,
}

pub fn run(cli: Cli) -> anyhow::Result<i32> {
    match cli.command {
        Commands::JobsAesthetic {
            max_iterations,
            delay_seconds,
            check_only,
            full_css_gate,
            e2e_port,
            reset_page_progress,
            only_pages,
        } => run_jobs_aesthetic(JobsAestheticOptions {
            max_iterations,
            delay_seconds,
            check_only,
            full_css_gate,
            e2e_port,
            reset_page_progress,
            only_pages,
        }),
        Commands::CssOptimize {
            max_iterations,
            delay_seconds,
            skip_visual,
            e2e_port,
        } => run_css_optimize(CssOptimizeOptions {
            max_iterations,
            delay_seconds,
            skip_visual,
            e2e_port,
        }),
        Commands::ListGates => {
            for gate in [
                GateCategory::CssNaming,
                GateCategory::CssAll,
                GateCategory::CssVisual,
                GateCategory::AestheticMetrics,
                GateCategory::AestheticReview,
            ] {
                println!("{} — {}", gate.log_label(), gate.description());
            }
            Ok(0)
        }
        Commands::Gate { name, verbose } => {
            let root = repo_root();
            let gate = parse_gate_name(&name)?;
            let log_path = if verbose {
                Some(
                    jobs_aesthetic_run_dir(&root).join(format!("gate-{name}.log")),
                )
            } else {
                None
            };
            let mut logger = LoopLogger::for_iteration(0, log_path.as_deref())?;
            let (passed, _, _) = logger.timed_gate(gate, || run_single_gate(&root, gate));
            Ok(if passed { 0 } else { 1 })
        }
        Commands::Gates { suite, verbose } => {
            let root = repo_root();
            let suite = parse_gate_suite(&suite)?;
            let log_path = if verbose {
                Some(jobs_aesthetic_run_dir(&root).join(format!("suite-{suite:?}.log")))
            } else {
                None
            };
            let mut logger = LoopLogger::for_iteration(0, log_path.as_deref())?;
            let passed = run_gate_suite(&root, suite, &mut logger);
            Ok(if passed { 0 } else { 1 })
        }
    }
}
