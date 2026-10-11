#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic_in_result_fn,
    reason = "Test-only integration file. A failed assertion is the intended failure signal for a \
              process-lifecycle contract, and the assertions below compare real return values, \
              statuses and captured side effects rather than trusting the function under test."
)]
//! Process-lifecycle failure paths for the local inference service (#635, plan v2).
//!
//! `InferenceService::start_instance` owns process spawning, health checking and the
//! `channel_providers` registration that follows a healthy start, but none of that had a test.
//! These tests drive it with **stub executables** instead of a real `llama-server`: the crate
//! must never download a model, start a real inference process or reach the network.
//!
//! Covered here:
//!
//! | Plan item | Test |
//! | --- | --- |
//! | (a) spawn failure | `a_missing_binary_is_reported_as_a_process_error_and_marks_the_model_failed` |
//! | (b) health-check failure | `a_stub_that_never_serves_fails_the_health_check_and_marks_the_model_failed` |
//! | (c) status defaults and restart | both tests (unknown model is `Stopped`; a second start on a `Failed` model must not panic) |
//! | (d) argv contract | `the_spawn_arguments_match_the_documented_contract` |
//! | (e) no database side effect on failure | both tests run against an **uninitialised** database |
//!
//! `BURNCLOUD_LLAMA_BIN` is process-global, so every test that sets it takes the same lock;
//! `cargo test` runs the tests of one binary in parallel and the environment is shared.

use burncloud_database::{create_database_with_url, Database};
use burncloud_service_inference::{
    InferenceConfig, InferenceError, InferenceService, InstanceStatus,
};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Serialises the tests that mutate `BURNCLOUD_LLAMA_BIN`.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// A path under the temporary directory that is unique per process and call.
fn scratch(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "bc_inference_{}_{}_{}",
        tag,
        std::process::id(),
        nanos
    ))
}

/// Write a POSIX shell stub and mark it executable.
///
/// The stub is the only "llama-server" these tests ever start: it records what it was called
/// with, or exits immediately, so the health check fails without a port ever being opened.
fn write_stub(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("writing the stub must succeed");
    let mut perms = std::fs::metadata(path)
        .expect("the stub exists")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("marking the stub executable must succeed");
}

/// A temporary SQLite database that was **never initialised**.
///
/// The schema is deliberately absent: if a failed start ever wrote to the database before the
/// process was up, the call would report a database error instead of the process error these
/// tests assert, which is how (e) becomes observable.
async fn uninitialised_db(tag: &str) -> Result<(Database, PathBuf), Box<dyn std::error::Error>> {
    let path = scratch(&format!("{tag}.db"));
    let _ = std::fs::remove_file(&path);
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}?mode=rwc", normalized);
    let db = create_database_with_url(&url).await?;
    Ok((db, path))
}

fn config(model_id: &str, file_path: &Path, port: u16) -> InferenceConfig {
    InferenceConfig {
        model_id: model_id.to_string(),
        file_path: file_path.to_string_lossy().to_string(),
        port,
        context_size: 2048,
        gpu_layers: -1,
    }
}

/// (a) + (c) + (e): a binary that cannot be spawned is a process error, the model is left
/// `Failed`, an unseen model reports `Stopped`, and a second start does not panic.
#[tokio::test]
async fn a_missing_binary_is_reported_as_a_process_error_and_marks_the_model_failed(
) -> Result<(), Box<dyn std::error::Error>> {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let missing = scratch("no-such-binary");
    std::env::set_var("BURNCLOUD_LLAMA_BIN", &missing);

    let (db, db_path) = uninitialised_db("spawn").await?;
    let service = InferenceService::new();
    let cfg = config("missing-binary-model", &scratch("model.gguf"), 18_080);

    // (c) An unknown model is Stopped — not a fabricated state and not a panic.
    assert_eq!(service.get_status(&cfg.model_id).await, InstanceStatus::Stopped);

    let err = service
        .start_instance(&db, cfg.clone())
        .await
        .expect_err("spawning a nonexistent binary must fail");
    assert!(
        matches!(err, InferenceError::ProcessSpawnFailed(_)),
        "the failure must be the process error, not a database one: {err:?}"
    );
    assert!(
        matches!(service.get_status(&cfg.model_id).await, InstanceStatus::Failed(_)),
        "a failed start must leave the model Failed: {:?}",
        service.get_status(&cfg.model_id).await
    );

    // (c) Starting again on a Failed model must report a failure, not panic.
    let second = service.start_instance(&db, cfg.clone()).await;
    assert!(
        second.is_err(),
        "a second start on a Failed model must fail again: {second:?}"
    );

    let _ = std::fs::remove_file(&db_path);
    Ok(())
}

/// (b) + (d): a stub that records its arguments and exits without ever listening fails the health
/// check, leaves the model `Failed`, and shows the exact argv llama-server is started with.
#[tokio::test]
async fn the_spawn_arguments_match_the_documented_contract(
) -> Result<(), Box<dyn std::error::Error>> {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let dir = scratch("stub");
    std::fs::create_dir_all(&dir)?;
    let log = dir.join("argv.log");
    let stub = dir.join("llama-server");
    write_stub(
        &stub,
        &format!("printf '%s\\n' \"$@\" > '{}'\nexit 1", log.display()),
    );
    std::env::set_var("BURNCLOUD_LLAMA_BIN", &stub);

    let (db, db_path) = uninitialised_db("argv").await?;
    let service = InferenceService::new();
    let file_path = scratch("documented-model.gguf");
    let cfg = config("documented-argv-model", &file_path, 18_081);

    let failure = service.start_instance(&db, cfg.clone()).await;
    assert!(
        failure.is_err(),
        "a stub that never serves must fail the health check: {failure:?}"
    );
    assert!(
        matches!(service.get_status(&cfg.model_id).await, InstanceStatus::Failed(_)),
        "a failed health check must leave the model Failed: {:?}",
        service.get_status(&cfg.model_id).await
    );

    let captured =
        std::fs::read_to_string(&log).expect("the stub must have recorded its arguments");
    let args: Vec<&str> = captured.lines().collect();
    let expected = vec![
        "-m",
        cfg.file_path.as_str(),
        "--port",
        "18081",
        "-c",
        "2048",
        "-ngl",
        "-1",
        "--nobrowser",
    ];
    assert_eq!(args, expected, "llama-server must be started with the documented arguments");

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
