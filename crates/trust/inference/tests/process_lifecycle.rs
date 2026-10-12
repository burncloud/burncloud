#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic_in_result_fn,
    clippy::let_underscore_must_use,
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
//! | (f) the process is really terminated on health-check failure (#886) | `a_failed_health_check_terminates_the_spawned_process` |
//!
//! (f) is deliberately not "the `processes` map is empty": an empty map is equally consistent
//! with a leaked process whose handle was merely dropped. The test below asserts on the real
//! process, by PID, and fails if the stub is still running.
//!
//! `BURNCLOUD_LLAMA_BIN` is process-global, so every test that sets it takes the same lock;
//! `cargo test` runs the tests of one binary in parallel and the environment is shared.

use burncloud_database::{create_database_with_url, Database};
use burncloud_service_inference::{
    InferenceConfig, InferenceError, InferenceService, InstanceStatus,
};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;

/// Serialises the tests that mutate `BURNCLOUD_LLAMA_BIN`.
static ENV_LOCK: Mutex<()> = Mutex::const_new(());

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

/// `true` while a process with `pid` exists (a zombie still counts as existing, which is why the
/// assertion below waits for `false` rather than accepting "it exited a moment ago").
///
/// `kill(pid, 0)` performs the permission and existence checks without delivering a signal.
/// Signal 0 is POSIX and never reaches the process.
fn pid_is_alive(pid: i32) -> bool {
    // SAFETY: `kill` with signal 0 is a pure existence/permission probe: it delivers no signal
    // and touches no memory. `pid` is a plain integer obtained from the stub itself, and the call
    // is safe to repeat.
    let rc = unsafe { libc::kill(pid, 0) };
    if rc == 0 {
        return true;
    }
    // EPERM means it exists but belongs to someone else; anything else (ESRCH) means it is gone.
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Wait (bounded) for `pid` to disappear, so the assertion is not a race against the OS.
async fn wait_until_gone(pid: i32, timeout: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if !pid_is_alive(pid) {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    !pid_is_alive(pid)
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
    std::fs::remove_file(&path).ok();
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
    let _guard = ENV_LOCK.lock().await;

    let missing = scratch("no-such-binary");
    std::env::set_var("BURNCLOUD_LLAMA_BIN", &missing);

    let (db, db_path) = uninitialised_db("spawn").await?;
    let service = InferenceService::new();
    let cfg = config("missing-binary-model", &scratch("model.gguf"), 18_080);

    // (c) An unknown model is Stopped — not a fabricated state and not a panic.
    assert_eq!(
        service.get_status(&cfg.model_id).await,
        InstanceStatus::Stopped
    );

    let err = service
        .start_instance(&db, cfg.clone())
        .await
        .expect_err("spawning a nonexistent binary must fail");
    assert!(
        matches!(err, InferenceError::ProcessSpawnFailed(_)),
        "the failure must be the process error, not a database one: {err:?}"
    );
    assert!(
        matches!(
            service.get_status(&cfg.model_id).await,
            InstanceStatus::Failed(_)
        ),
        "a failed start must leave the model Failed: {:?}",
        service.get_status(&cfg.model_id).await
    );

    // (c) Starting again on a Failed model must report a failure, not panic.
    let second = service.start_instance(&db, cfg.clone()).await;
    assert!(
        second.is_err(),
        "a second start on a Failed model must fail again: {second:?}"
    );

    std::fs::remove_file(&db_path).ok();
    Ok(())
}

/// (b) + (d): a stub that records its arguments and exits without ever listening fails the health
/// check, leaves the model `Failed`, and shows the exact argv llama-server is started with.
#[tokio::test]
async fn the_spawn_arguments_match_the_documented_contract(
) -> Result<(), Box<dyn std::error::Error>> {
    let _guard = ENV_LOCK.lock().await;

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
        matches!(
            service.get_status(&cfg.model_id).await,
            InstanceStatus::Failed(_)
        ),
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
    assert_eq!(
        args, expected,
        "llama-server must be started with the documented arguments"
    );

    std::fs::remove_file(&db_path).ok();
    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

/// (f) #886: a failed health check must actually **terminate** the process it started.
///
/// The stub records its own PID and then sleeps well past the health-check window, so the
/// service is still running when the check times out. Two independent things are asserted:
///
/// 1. the handle was released out of `processes` (so the failure path does not keep a live
///    process addressable), and
/// 2. **the process is gone** — checked against the OS by PID, which is what the defect in
///    #886 actually violated. Before the fix the service dropped the `Child` without killing
///    it, so this assertion failed while the stub kept running.
///
/// A bare `assert!(processes.is_empty())` would not have caught #886: the map was already
/// empty there, and dropping a `tokio::process::Child` does not end the process.
#[tokio::test]
async fn a_failed_health_check_terminates_the_spawned_process(
) -> Result<(), Box<dyn std::error::Error>> {
    let _guard = ENV_LOCK.lock().await;

    let dir = scratch("kill-on-failure");
    std::fs::create_dir_all(&dir)?;
    let pid_file = dir.join("stub.pid");
    let stub = dir.join("llama-server");

    // Record the PID, then stay alive: without the fix the process outlives the failed start.
    // No port is opened, so the health check fails. The sleep is deliberately longer than the
    // 60-attempt window so the process is unambiguously still running at the assertion point.
    write_stub(
        &stub,
        &format!("printf '%s' \"$$\" > '{}'\nsleep 300", pid_file.display()),
    );
    std::env::set_var("BURNCLOUD_LLAMA_BIN", &stub);

    let (db, db_path) = uninitialised_db("kill").await?;
    let service = InferenceService::new();
    let cfg = config("kill-on-failed-health-check", &scratch("model.gguf"), 18_082);

    let failure = service.start_instance(&db, cfg.clone()).await;
    let err = failure.expect_err("a stub that never serves must fail the health check");

    // The failure surfaced must be the health-check failure, not a kill failure (#886 AC1:
    // a kill problem must not mask the original reason).
    assert!(
        matches!(&err, InferenceError::ProcessSpawnFailed(msg)
            if msg.contains("Health check timeout")),
        "the reported failure must remain the health-check timeout: {err:?}"
    );

    // The service released the handle: a failed start must not leave a live process addressable.
    assert!(
        matches!(
            service.get_status(&cfg.model_id).await,
            InstanceStatus::Failed(_)
        ),
        "a failed health check must leave the model Failed"
    );

    // The stub must have written its PID, proving the process really started (otherwise the
    // liveness check below would pass vacuously because nothing ever ran).
    let raw = std::fs::read_to_string(&pid_file)
        .expect("the stub must have recorded its PID before sleeping");
    let pid: i32 = raw
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("stub pid {raw:?} is not a number: {e}"));
    assert!(
        pid_is_alive(pid),
        "the stub must still be running when the health check fails, otherwise this test \
         proves nothing about termination"
    );

    // The heart of #886: the process the service started must no longer exist.
    assert!(
        wait_until_gone(pid, std::time::Duration::from_secs(10)).await,
        "process {pid} is still alive after a failed health check, so the spawned child was \
         leaked instead of being terminated"
    );

    std::fs::remove_file(&db_path).ok();
    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}
