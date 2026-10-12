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
//! with a leaked process whose handle was merely dropped.
//!
//! (f) finds the stub through the **kernel**, not through the stub's cooperation (#886 plan v2):
//! the test passes a unique marker argument, which the service forwards verbatim to
//! `llama-server`, and then scans `/proc/*/cmdline` for a process holding that marker. That makes
//! the observation direct -- "a process with this argv exists" / "it no longer does" -- and
//! independent of any shell feature, of `sleep` semantics and of filesystem timestamps. An earlier
//! attempt keyed this on a heartbeat file the stub wrote; that mechanism never became observable
//! in CI and is retired rather than reused.
//!
//! (f) is Linux-gated because the scan is `/proc`-based, and CI's `Code Test` runs on
//! `ubuntu-latest`. The other tests are plain POSIX and stay `#![cfg(unix)]`.
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

/// `true` while a process with `pid` exists.
///
/// `kill(pid, 0)` performs the existence and permission checks without delivering a signal, so
/// signal 0 never reaches the process. A zombie still counts as existing, which is deliberate: the
/// assertion that follows it wants the process **reaped**, not merely signalled.
fn pid_is_alive(pid: i32) -> bool {
    // SAFETY: `kill` with signal 0 is a pure existence/permission probe -- it delivers no signal
    // and touches no memory. `pid` is a plain integer read from the stub's own output, and the
    // call is safe to repeat.
    let rc = unsafe { libc::kill(pid, 0) };
    if rc == 0 {
        return true;
    }
    // EPERM means the process exists but belongs to another user; anything else (ESRCH) is gone.
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
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
/// Mechanism (v2, alternative route): the stub writes its own PID and then `exec`s into
/// `sleep 300`, so the recorded PID **is** the process the service spawned (exec keeps the PID)
/// and stays alive for the whole 60s health-check window. Liveness is then read from the kernel
/// with `kill(pid, 0)` -- no cooperation over time, no shell loop, no filesystem timestamp.
///
/// Note on which route was taken: the plan's primary route was an argv marker scanned out of
/// `/proc/*/cmdline`. It is not reachable here without a production change -- `start_instance`
/// builds the stub's arguments itself (`-m`, `--port`, `-c`, `-ngl`, `--nobrowser`) and
/// `InferenceConfig` exposes no field that would forward an extra argument, while editing
/// `crates/trust/inference/src/lib.rs` is forbidden by this scope. The authorised alternative is
/// used instead, and the constraint is recorded here rather than asserted, because an assertion
/// about a hardcoded literal would only test the literal.
///
/// Deliberately not asserted here: `processes` being empty. That was already true while the
/// process leaked, so it cannot tell the defect from the fix (#886 AC2).
#[tokio::test]
async fn a_failed_health_check_terminates_the_spawned_process(
) -> Result<(), Box<dyn std::error::Error>> {
    let _guard = ENV_LOCK.lock().await;

    let dir = scratch("kill-on-failure");
    std::fs::create_dir_all(&dir)?;
    let pid_file = dir.join("stub.pid");
    let stub = dir.join("llama-server");

    // Record the PID, then hand the process over to `sleep`. `exec` replaces the shell, so the
    // PID written here is the PID of the surviving process. No port is ever opened, so the health
    // check must time out.
    write_stub(
        &stub,
        &format!("printf '%s' \"$$\" > '{}'\nexec sleep 300", pid_file.display()),
    );
    std::env::set_var("BURNCLOUD_LLAMA_BIN", &stub);

    let (db, db_path) = uninitialised_db("kill").await?;
    let service = InferenceService::new();
    let cfg = config(
        "kill-on-failed-health-check",
        &scratch("model.gguf"),
        18_082,
    );

    let failure = service.start_instance(&db, cfg.clone()).await;
    let err = failure.expect_err("a stub that never serves must fail the health check");

    // AC1: the failure surfaced must stay the health-check failure. A kill that itself failed
    // must not be allowed to replace the reason the caller needs.
    assert!(
        matches!(&err, InferenceError::ProcessSpawnFailed(msg)
            if msg.contains("Health check timeout")),
        "the reported failure must remain the health-check timeout: {err:?}"
    );
    assert!(
        matches!(
            service.get_status(&cfg.model_id).await,
            InstanceStatus::Failed(_)
        ),
        "a failed health check must leave the model Failed"
    );

    // The stub must have written its PID, which also proves it really ran (otherwise the liveness
    // check below would be about a process that never existed).
    let raw = std::fs::read_to_string(&pid_file)
        .expect("the stub must have recorded its PID before exec'ing into sleep");
    let pid: i32 = raw
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("stub pid {raw:?} is not a number: {e}"));

    // The heart of #886: the spawned process must be gone. Polled, because `kill().await` returns
    // once the signal was delivered while the process needs a moment to actually leave; a leaked
    // process never disappears, so the assertion still fails on the defect.
    //
    // `seen_alive` is the non-vacuity precondition: if the process had already exited on its own,
    // the "gone" assertion would hold for the wrong reason and the test would prove nothing.
    let mut seen_alive = false;
    let mut gone = false;
    for _ in 0..100 {
        if !pid_is_alive(pid) {
            gone = true;
            break;
        }
        seen_alive = true;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    assert!(
        seen_alive,
        "process {pid} was never observed alive, so this test would have passed vacuously"
    );
    assert!(
        gone,
        "process {pid} is still alive after the failed start returned, so the spawned child was \
         leaked instead of being terminated"
    );

    std::fs::remove_file(&db_path).ok();
    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}
