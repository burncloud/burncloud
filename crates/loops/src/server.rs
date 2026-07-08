use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::paths::burncloud_bin;
use crate::process::cleanup_stale_e2e_processes;

pub struct E2eServer {
    child: Child,
    pub base_url: String,
}

impl E2eServer {
    pub fn start(root: &Path, port: u16) -> anyhow::Result<Self> {
        let bin = burncloud_bin(root);
        if !bin.exists() {
            eprintln!("Building burncloud (one-time)...");
            let status = Command::new("cargo")
                .args(["build", "--bin", "burncloud"])
                .current_dir(root)
                .status()?;
            if !status.success() {
                anyhow::bail!("cargo build --bin burncloud failed");
            }
        }

        // Best-effort cleanup of stale processes from prior runs.
        cleanup_stale_e2e_processes();
        thread::sleep(Duration::from_millis(500));

        let base_url = format!("http://127.0.0.1:{port}");
        eprintln!("Starting loop E2E server at {base_url} (persistent across iterations)...");

        let child = Command::new(&bin)
            .args(["server", "start"])
            .current_dir(root)
            .env("PORT", port.to_string())
            .env("RUST_LOG", "burncloud=warn")
            .env("NO_PROXY", "*")
            .env(
                "MASTER_KEY",
                "a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8",
            )
            .env("PRICE_SYNC_INTERVAL_SECS", "999999")
            .env("SKIP_INITIAL_PRICE_SYNC", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        wait_health(&base_url)?;
        eprintln!("  E2E server ready: {base_url}");
        Ok(Self { child, base_url })
    }

    pub fn apply_env(&self) {
        std::env::set_var("E2E_BASE_URL", &self.base_url);
        std::env::set_var("JOBS_LOOP_E2E_URL", &self.base_url);
        std::env::set_var("NO_PROXY", "*");
    }
}

impl Drop for E2eServer {
    fn drop(&mut self) {
        eprintln!("Stopping loop E2E server (pid {})...", self.child.id());
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::env::remove_var("JOBS_LOOP_E2E_URL");
    }
}

fn wait_health(base_url: &str) -> anyhow::Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    let url = format!("{base_url}/health");
    for attempt in 0..120 {
        if let Ok(resp) = client.get(&url).send() {
            if resp.status().is_success() {
                return Ok(());
            }
        }
        if attempt > 0 && attempt % 10 == 0 {
            eprintln!("  Waiting for burncloud at {base_url} ({attempt}/120)...");
        }
        thread::sleep(Duration::from_millis(500));
    }
    anyhow::bail!("Server failed to start at {base_url}");
}
