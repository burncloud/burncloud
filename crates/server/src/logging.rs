use std::{env, fs};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    filter::{EnvFilter, Targets},
    layer::SubscriberExt,
    Layer,
};

/// Initialize the tracing-based logging system.
///
/// - Stdout layer controlled by `RUST_LOG` env var
/// - Per-module file layers (server, service, database, router)
/// - Daily log rotation with retention via `LOG_MAX_FILES` (default: 7)
/// - Log directory via `LOG_DIR` env var (default: `./logs`)
/// - `tracing-log` bridge so existing `log::*!` calls route to tracing
///
/// Returns `WorkerGuard`s that must be held for the program's lifetime.
pub fn init_logging() -> Vec<WorkerGuard> {
    let log_dir = env::var("LOG_DIR").unwrap_or_else(|_| "./logs".to_string());
    let max_files = env::var("LOG_MAX_FILES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(7);

    fs::create_dir_all(&log_dir).ok();
    tracing_log::LogTracer::init().ok();

    let env_filter = EnvFilter::from_default_env();
    let mut guards = Vec::new();

    let (server_nb, g) = file_appender(&log_dir, "server", max_files);
    guards.push(g);
    let (service_nb, g) = file_appender(&log_dir, "service", max_files);
    guards.push(g);
    let (database_nb, g) = file_appender(&log_dir, "database", max_files);
    guards.push(g);
    let (router_nb, g) = file_appender(&log_dir, "router", max_files);
    guards.push(g);

    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(server_nb)
                .with_ansi(false)
                .with_filter(module_filter("burncloud_server")),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(service_nb)
                .with_ansi(false)
                .with_filter(module_filter("burncloud_service")),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(database_nb)
                .with_ansi(false)
                .with_filter(module_filter("burncloud_database")),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(router_nb)
                .with_ansi(false)
                .with_filter(module_filter("burncloud_router")),
        );

    tracing::subscriber::set_global_default(subscriber).ok();

    guards
}

fn module_filter(target: &'static str) -> Targets {
    Targets::new().with_target(target, tracing::Level::INFO)
}

fn file_appender(
    log_dir: &str,
    prefix: &str,
    max_files: usize,
) -> (tracing_appender::non_blocking::NonBlocking, WorkerGuard) {
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(prefix)
        .filename_suffix("log")
        .max_log_files(max_files)
        .build(log_dir)
        .unwrap_or_else(|e| panic!("failed to create {prefix} log appender: {e}"));
    tracing_appender::non_blocking(appender)
}
