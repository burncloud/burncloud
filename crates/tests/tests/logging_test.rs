#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
use std::sync::OnceLock;

static LOG_DIR: OnceLock<std::path::PathBuf> = OnceLock::new();

fn ensure_logging() -> &'static std::path::Path {
    LOG_DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join("burncloud_logging_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("failed to create test log dir");
        std::env::set_var("LOG_DIR", &dir);
        std::env::set_var("RUST_LOG", "info");
        let guards = burncloud_server::logging::init_logging();
        Box::leak(Box::new(guards));
        dir
    })
}

fn wait_for_file_content(dir: &std::path::Path, needle: &str) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if content.contains(needle) {
                        return true;
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    false
}

#[test]
fn test_tracing_init() {
    ensure_logging();
}

#[test]
fn test_tracing_output() {
    let dir = ensure_logging();
    tracing::info!(target: "burncloud_server", "test_tracing_output_marker");
    assert!(
        wait_for_file_content(dir, "test_tracing_output_marker"),
        "tracing output not found in log files"
    );
}

#[test]
fn test_log_bridge_output() {
    let dir = ensure_logging();
    log::info!(target: "burncloud_server", "test_log_bridge_marker");
    assert!(
        wait_for_file_content(dir, "test_log_bridge_marker"),
        "log bridge output not found in log files"
    );
}

#[test]
fn test_log_file_creation() {
    let dir = ensure_logging();
    tracing::info!(target: "burncloud_server", "file_creation_server");
    tracing::info!(target: "burncloud_service", "file_creation_service");
    tracing::info!(target: "burncloud_database", "file_creation_database");
    tracing::info!(target: "burncloud_router", "file_creation_router");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let entries: Vec<String> = std::fs::read_dir(dir)
            .expect("failed to read log dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        if entries.len() >= 4 {
            break;
        }

        if std::time::Instant::now() > deadline {
            panic!(
                "expected 4 log files, found {}: {:?}",
                entries.len(),
                entries
            );
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let valid_prefixes = ["server", "service", "database", "router"];
    for entry in std::fs::read_dir(dir)
        .expect("failed to read log dir")
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        let valid = valid_prefixes
            .iter()
            .any(|p| name.starts_with(p) && name.ends_with(".log"));
        assert!(valid, "unexpected log file name: {}", name);
    }
}
