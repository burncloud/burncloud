use std::process::{Command, Stdio};

/// Stop orphaned `burncloud` / `api_tests-*` / `agent-browser` processes that block E2E.
pub fn cleanup_stale_e2e_processes() {
    #[cfg(windows)]
    {
        // Best effort: the process may already be gone, and a missing PowerShell is not fatal.
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-Process -ErrorAction SilentlyContinue | Where-Object { \
                 $_.ProcessName -eq 'burncloud' -or \
                 $_.ProcessName -like 'api_tests-*' -or \
                 $_.ProcessName -like 'agent-browser*' } | \
                 Stop-Process -Force -ErrorAction SilentlyContinue",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok();
    }
    #[cfg(not(windows))]
    {
        // Best effort: nothing to kill when pkill is unavailable or no process matches.
        Command::new("pkill")
            .args(["-f", "api_tests-"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok();
        Command::new("pkill")
            .args(["-f", "burncloud"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok();
        Command::new("pkill")
            .args(["-f", "agent-browser"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok();
    }
}
