//! Exclusive lock so only one jobs-aesthetic loop runs at a time.

use std::path::{Path, PathBuf};

pub struct LoopRunLock {
    path: PathBuf,
}

impl LoopRunLock {
    pub fn acquire(run_dir: &Path) -> anyhow::Result<Self> {
        std::fs::create_dir_all(run_dir)?;
        let path = run_dir.join("loop.lock");
        if path.exists() {
            let stale = std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
                .is_none_or(|pid| !is_process_alive(pid));
            if stale {
                let _ = std::fs::remove_file(&path);
            } else if let Ok(text) = std::fs::read_to_string(&path) {
                let pid = text.trim();
                anyhow::bail!(
                    "Another jobs-aesthetic loop is already running (pid {pid}, lock: {}). \
                     Wait for it to finish before starting a new run.",
                    path.display()
                );
            }
        }
        std::fs::write(&path, std::process::id().to_string())?;
        Ok(Self { path })
    }
}

impl Drop for LoopRunLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|o| {
                let out = String::from_utf8_lossy(&o.stdout);
                out.contains(&pid.to_string()) && !out.contains("No tasks")
            })
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    }
}
