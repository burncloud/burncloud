//! Monitor status CLI commands
//!
//! This module provides CLI commands for system monitoring:
//! - status: Show system status and metrics
//! - server: Monitor server process status and logs

use anyhow::Result;
use burncloud_database::{ph, sqlx, Database};
use burncloud_database_channel::ChannelProviderModel;
use clap::ArgMatches;
use serde::Serialize;
use std::process::Command as StdCommand;
use std::time::{SystemTime, UNIX_EPOCH};

/// System status output for JSON format
#[derive(Debug, Clone, Serialize)]
pub struct SystemStatus {
    pub total_channels: usize,
    pub active_channels: usize,
    pub today_requests: i64,
    pub today_tokens: i64,
    /// Revenue in dollars (converted from nanodollars)
    pub today_revenue_usd: f64,
}

/// Server status output
#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub process_running: bool,
    pub pid: Option<u32>,
    pub tmux_session: Option<String>,
    pub port_3000_in_use: bool,
    pub last_log_time: Option<String>,
    pub uptime_seconds: Option<u64>,
    pub recent_errors: Vec<String>,
}

/// Handle monitor status command
pub async fn cmd_monitor_status(db: &Database, matches: &ArgMatches) -> Result<()> {
    let format = matches
        .get_one::<String>("format")
        .map(|s| s.as_str())
        .unwrap_or("table");

    // Get channel counts
    let channels = ChannelProviderModel::list(db, 10000, 0).await?;
    let total_channels = channels.len();
    let active_channels = channels.iter().filter(|c| c.status == 1).count();

    // Get today's statistics from router_logs
    let (today_requests, today_tokens, today_revenue_nano) = get_today_stats(db).await?;

    // Convert nanodollars to dollars
    let today_revenue_usd = today_revenue_nano as f64 / 1_000_000_000.0;

    let status = SystemStatus {
        total_channels,
        active_channels,
        today_requests,
        today_tokens,
        today_revenue_usd,
    };

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&status)?;
            println!("{}", json);
        }
        _ => {
            // Table format
            println!("System Status");
            println!("{}", "=".repeat(50));
            println!();
            println!("  Channels:");
            println!("    Total:      {}", status.total_channels);
            println!("    Active:     {}", status.active_channels);
            println!(
                "    Inactive:   {}",
                status.total_channels - status.active_channels
            );
            println!();
            println!("  Today's Statistics:");
            println!("    Requests:   {}", status.today_requests);
            println!("    Tokens:     {}", status.today_tokens);
            println!("    Revenue:    ${:.6}", status.today_revenue_usd);
            println!();
        }
    }

    Ok(())
}

/// Handle monitor server command
pub async fn cmd_monitor_server(matches: &ArgMatches) -> Result<()> {
    let show_logs = matches.get_flag("logs");
    let tail_lines = matches.get_one::<usize>("tail").copied().unwrap_or(50);

    let status = check_server_status()?;

    // Print server status
    println!("Server Status");
    println!("{}", "=".repeat(50));
    println!();

    if status.process_running {
        println!("  Process:     Running (PID: {})", status.pid.unwrap_or(0));
    } else {
        println!("  Process:     NOT RUNNING");
    }

    if let Some(session) = &status.tmux_session {
        println!("  Tmux:        Session '{}' active", session);
    } else {
        println!("  Tmux:        No active session");
    }

    println!("  Port 3000:   {}", if status.port_3000_in_use { "In use" } else { "Not in use" });

    if let Some(time) = &status.last_log_time {
        println!("  Last Log:    {}", time);
    }

    if let Some(uptime) = status.uptime_seconds {
        let hours = uptime / 3600;
        let mins = (uptime % 3600) / 60;
        println!("  Uptime:      {}h {}m", hours, mins);
    }

    // Check for recent errors
    if !status.recent_errors.is_empty() {
        println!();
        println!("  Recent Errors:");
        for err in &status.recent_errors {
            println!("    - {}", err);
        }
    }

    // Show logs if requested
    if show_logs {
        println!();
        println!("Recent Server Logs (last {} lines):", tail_lines);
        println!("{}", "-".repeat(50));
        show_recent_logs(tail_lines)?;
    }

    // Show tmux session logs if available
    if status.tmux_session.is_some() {
        println!();
        println!("Tmux Session Output:");
        println!("{}", "-".repeat(50));
        show_tmux_output()?;
    }

    Ok(())
}

/// Check server process status
fn check_server_status() -> Result<ServerStatus> {
    // Check if burncloud process is running
    let process_running = check_process_running()?;
    let pid = get_process_pid()?;

    // Check tmux session
    let tmux_session = check_tmux_session()?;

    // Check port 3000
    let port_3000_in_use = check_port_in_use(3000)?;

    // Get last log time
    let last_log_time = get_last_log_time()?;

    // Calculate uptime if process is running
    let uptime_seconds = if pid.is_some() {
        get_process_uptime(pid.unwrap())?
    } else {
        None
    };

    // Check for recent errors in logs
    let recent_errors = get_recent_errors()?;

    Ok(ServerStatus {
        process_running,
        pid,
        tmux_session,
        port_3000_in_use,
        last_log_time,
        uptime_seconds,
        recent_errors,
    })
}

/// Check if burncloud process is running
fn check_process_running() -> Result<bool> {
    let output = StdCommand::new("pgrep")
        .arg("-x")
        .arg("burncloud")
        .output();

    match output {
        Ok(o) => Ok(!o.stdout.is_empty()),
        Err(_) => {
            // Fallback: check with ps
            let output = StdCommand::new("ps")
                .arg("aux")
                .output()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.contains("burncloud") && !stdout.contains("grep"))
        }
    }
}

/// Get burncloud process PID
fn get_process_pid() -> Result<Option<u32>> {
    let output = StdCommand::new("pgrep")
        .arg("-x")
        .arg("burncloud")
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.is_empty() {
                Ok(None)
            } else {
                // pgrep may return multiple PIDs (one per line), take the first one
                let first_pid = stdout.lines().next().unwrap_or("").trim();
                if first_pid.is_empty() {
                    Ok(None)
                } else {
                    match first_pid.parse::<u32>() {
                        Ok(pid) => Ok(Some(pid)),
                        Err(_) => Ok(None),
                    }
                }
            }
        }
        Err(_) => Ok(None),
    }
}

/// Check if tmux session 'burncloud' exists
fn check_tmux_session() -> Result<Option<String>> {
    let output = StdCommand::new("tmux")
        .arg("list-sessions")
        .arg("-F")
        .arg("#{session_name}")
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.contains("burncloud") {
                Ok(Some("burncloud".to_string()))
            } else {
                Ok(None)
            }
        }
        Err(_) => Ok(None),
    }
}

/// Check if a port is in use
fn check_port_in_use(port: u16) -> Result<bool> {
    let output = StdCommand::new("ss")
        .arg("-tlnp")
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            Ok(stdout.contains(&format!(":{}", port)))
        }
        Err(_) => {
            // Fallback: try netstat
            let output = StdCommand::new("netstat")
                .arg("-tlnp")
                .output();
            match output {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    Ok(stdout.contains(&format!(":{}", port)))
                }
                Err(_) => Ok(false),
            }
        }
    }
}

/// Get last log entry time from router logs
fn get_last_log_time() -> Result<Option<String>> {
    // Find the most recent router log file
    let log_dir = std::path::Path::new("logs");
    if !log_dir.exists() {
        return Ok(None);
    }

    let router_logs: Vec<_> = std::fs::read_dir(log_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("router."))
        .collect();

    if router_logs.is_empty() {
        return Ok(None);
    }

    // Get the most recent file
    let latest_file = router_logs
        .into_iter()
        .max_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()).unwrap_or(SystemTime::UNIX_EPOCH));

    if let Some(file) = latest_file {
        let content = std::fs::read_to_string(file.path())?;
        let last_line = content.lines().last();
        if let Some(line) = last_line {
            // Extract timestamp from log line (format: 2026-05-17T14:42:47...)
            if let Some(ts) = line.split_whitespace().next() {
                return Ok(Some(ts.to_string()));
            }
        }
    }

    Ok(None)
}

/// Get process uptime in seconds
fn get_process_uptime(pid: u32) -> Result<Option<u64>> {
    // Read /proc/<pid>/stat to get process start time
    let stat_path = std::path::Path::new("/proc").join(pid.to_string()).join("stat");
    if !stat_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(stat_path)?;
    // The 22nd field (starttime) is in clock ticks since boot
    // Format: pid (comm) state ppid pgrp session tty_nr tpgid flags ...
    // We need to handle the comm field which may contain spaces and parentheses
    let content = content.trim();

    // Find the closing parenthesis to skip the comm field
    let paren_end = match content.find(')') {
        Some(pos) => pos + 1,
        None => return Ok(None),
    };

    // Split the rest of the content after the comm field
    let after_comm = &content[paren_end..];
    let fields: Vec<&str> = after_comm.split_whitespace().collect();

    // starttime is the 22nd field overall, which is field index 21 (0-indexed)
    // After the comm field, we have: state ppid pgrp session tty_nr tpgid flags minflt cminflt majflt cmajflt utime stime cutime cstime priority nice num_threads itrealvalue starttime
    // That's 20 fields after comm, so starttime is at index 19 (after comm)
    if fields.len() >= 20 {
        let starttime_ticks: u64 = match fields[19].parse() {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        // Get system boot time from /proc/stat
        let btime_content = match std::fs::read_to_string("/proc/stat") {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        let btime_line = btime_content.lines().find(|l| l.starts_with("btime"));
        if let Some(line) = btime_line {
            let btime_str = match line.split_whitespace().nth(1) {
                Some(s) => s,
                None => return Ok(None),
            };
            let btime: u64 = match btime_str.parse() {
                Ok(v) => v,
                Err(_) => return Ok(None),
            };
            // Convert ticks to seconds (usually 100 ticks per second)
            let clk_tck: u64 = 100; // sysconf(_SC_CLK_TCK) is usually 100 on Linux
            let start_seconds = btime + starttime_ticks / clk_tck;
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            let uptime = now.saturating_sub(start_seconds);
            return Ok(Some(uptime));
        }
    }

    Ok(None)
}

/// Get recent errors from logs
fn get_recent_errors() -> Result<Vec<String>> {
    let log_dir = std::path::Path::new("logs");
    if !log_dir.exists() {
        return Ok(Vec::new());
    }

    let mut errors = Vec::new();

    // Check all log files for recent errors
    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let filename = entry.file_name().to_string_lossy().to_string();
        if filename.ends_with(".log") {
            let content = std::fs::read_to_string(entry.path())?;
            for line in content.lines().rev().take(100) {
                if line.contains("error") || line.contains("Error") || line.contains("ERROR")
                    || line.contains("panic") || line.contains("Panic")
                    || line.contains("fatal") || line.contains("Fatal") {
                    // Extract relevant part
                    if let Some(ts) = line.split_whitespace().next() {
                        errors.push(format!("{}: {}", ts, line.split_whitespace().skip(1).take(5).collect::<Vec<_>>().join(" ")));
                    }
                    if errors.len() >= 5 {
                        break;
                    }
                }
            }
        }
        if errors.len() >= 5 {
            break;
        }
    }

    Ok(errors)
}

/// Show recent logs from router log file
fn show_recent_logs(n: usize) -> Result<()> {
    let log_dir = std::path::Path::new("logs");
    if !log_dir.exists() {
        println!("No logs directory found");
        return Ok(());
    }

    // Find the most recent router log file
    let router_logs: Vec<_> = std::fs::read_dir(log_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("router."))
        .collect();

    if router_logs.is_empty() {
        println!("No router logs found");
        return Ok(());
    }

    let latest_file = router_logs
        .into_iter()
        .max_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()).unwrap_or(SystemTime::UNIX_EPOCH));

    if let Some(file) = latest_file {
        let content = std::fs::read_to_string(file.path())?;
        let lines: Vec<&str> = content.lines().rev().take(n).collect();
        for line in lines.iter().rev() {
            println!("{}", line);
        }
    }

    Ok(())
}

/// Show tmux session output
fn show_tmux_output() -> Result<()> {
    let output = StdCommand::new("tmux")
        .arg("capture-pane")
        .arg("-t")
        .arg("burncloud")
        .arg("-p")
        .arg("-S")
        .arg("-50") // Last 50 lines
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.is_empty() {
                println!("No tmux output available");
            } else {
                println!("{}", stdout);
            }
        }
        Err(e) => {
            println!("Could not capture tmux output: {}", e);
        }
    }

    Ok(())
}

/// Get today's statistics from router_logs
///
/// Returns (requests, tokens, revenue_nano)
async fn get_today_stats(db: &Database) -> Result<(i64, i64, i64)> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Calculate today's start timestamp (midnight UTC)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Time error: {}", e))?
        .as_secs() as i64;

    // Get start of today (midnight UTC)
    let today_start = now - (now % 86400);

    let time_filter = if is_postgres {
        format!("EXTRACT(EPOCH FROM created_at)::BIGINT >= {}", ph(is_postgres, 1))
    } else {
        "strftime('%s', created_at) >= CAST(? AS TEXT)".to_string()
    };

    let sql = format!(
        r#"
        SELECT
            COUNT(*) as requests,
            COALESCE(SUM(prompt_tokens + completion_tokens), 0) as tokens,
            COALESCE(SUM(cost), 0) as revenue
        FROM router_logs
        WHERE created_at IS NOT NULL AND {}
        "#,
        time_filter
    );

    let row: (Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as(&sql)
        .bind(today_start)
        .fetch_one(conn.pool())
        .await?;

    Ok((row.0.unwrap_or(0), row.1.unwrap_or(0), row.2.unwrap_or(0)))
}

/// Route monitor commands
pub async fn handle_monitor_command(db: &Database, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("status", sub_m)) => {
            cmd_monitor_status(db, sub_m).await?;
        }
        Some(("server", sub_m)) => {
            cmd_monitor_server(sub_m).await?;
        }
        _ => {
            println!("Usage: burncloud monitor <status|server>");
            println!("Run 'burncloud monitor --help' for more information.");
        }
    }

    Ok(())
}
