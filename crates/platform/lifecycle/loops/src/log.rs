use chrono::Utc;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use crate::gates::GateCategory;

/// Writes the same lines to stdout and an optional per-iteration log file.
pub struct LoopLogger {
    file: Option<File>,
    iteration: u32,
}

impl LoopLogger {
    pub fn for_iteration(iteration: u32, log_path: Option<&Path>) -> anyhow::Result<Self> {
        let file = if let Some(path) = log_path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            Some(
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(path)?,
            )
        } else {
            None
        };
        Ok(Self { file, iteration })
    }

    fn stamp(&self, category: &str, level: &str) -> String {
        format!(
            "[{}] [iter {}] [{category}] {level}",
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
            self.iteration
        )
    }

    pub fn line(&mut self, category: &str, message: &str) {
        let line = format!("{} {}", self.stamp(category, "INFO"), message);
        let _ = writeln!(io::stdout(), "{line}");
        if let Some(file) = self.file.as_mut() {
            let _ = writeln!(file, "{line}");
        }
    }

    pub fn gate_start(&mut self, gate: GateCategory) {
        self.line(&gate.log_label(), "START");
    }

    pub fn gate_end(&mut self, gate: GateCategory, passed: bool, elapsed_secs: f64) {
        let status = if passed { "PASS" } else { "FAIL" };
        self.line(
            &gate.log_label(),
            &format!("{status} ({elapsed_secs:.1}s)"),
        );
    }

    pub fn gate_output(&mut self, _gate: GateCategory, lines: &[String]) {
        for line in lines {
            let indented = format!("  | {line}");
            let _ = writeln!(io::stdout(), "{indented}");
            if let Some(file) = self.file.as_mut() {
                let _ = writeln!(file, "{indented}");
            }
        }
    }

    pub fn timed_gate<F>(&mut self, gate: GateCategory, run: F) -> (bool, Vec<String>, f64)
    where
        F: FnOnce() -> anyhow::Result<(bool, Vec<String>)>,
    {
        self.gate_start(gate);
        let started = Instant::now();
        let (passed, lines) = match run() {
            Ok(v) => v,
            Err(err) => {
                let msg = format!("ERROR: {err:#}");
                let lines = vec![msg];
                (false, lines)
            }
        };
        let elapsed = started.elapsed().as_secs_f64();
        self.gate_output(gate, &lines);
        self.gate_end(gate, passed, elapsed);
        (passed, lines, elapsed)
    }
}
