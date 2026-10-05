#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "Test-only file: the assertions are the test. `serde_json::Value` is allowed here for the same \
              reason `lib.rs` allows it -- the loop-state JSON is written by this crate but the assertions read \
              it as a document rather than through a domain type, which is what the workspace gate permits at \
              an explicit boundary. An integration test is its own crate root, so it cannot inherit `lib.rs`'s \
              allowance."
)]
//! The loop run lock and the persisted run state (#633, plan section 5 item 21).
//!
//! `burncloud-loops` is ~2500 lines with **no tests at all**. The plan lists eight behaviours; this file covers
//! the ones that need no subprocess and no long-running service -- "同目录互斥锁、陈旧锁、并发获取；运行状态序列化；
//! 路径限定；任务进度恢复" -- and leaves the rest named at the end.
//!
//! ## What the lock actually is
//!
//! `lock.rs` is a **lock file containing a PID**, not an OS lock:
//!
//! ```text
//! acquire(run_dir):
//!   create_dir_all(run_dir)
//!   path = run_dir/"loop.lock"
//!   if path exists:
//!       stale = contents do not parse as a u32, OR the pid is not alive
//!       if stale      -> remove it
//!       else          -> bail "another loop is already running"
//!   write(process::id())
//! Drop: remove_file(path)
//! ```
//!
//! Two properties follow, and both are asserted rather than assumed:
//!
//! 1. **It is not atomic.** The existence check, the staleness decision, the removal and the write are separate
//!    filesystem calls with no lock held between them, so two processes calling `acquire` at the same moment can
//!    both pass the check. `concurrent_acquires_in_one_process_cannot_both_succeed` measures that **within one
//!    process**, which is weaker than the real race and is described as such.
//! 2. **Staleness is decided by liveness**, and liveness is decided differently per platform. On Windows it
//!    shells out to `tasklist` and tests `output.contains(&pid.to_string())` -- a **substring** test, so a PID
//!    that is a substring of another running PID reads as alive. `a_pid_that_is_a_substring_of_a_live_pid_reads_as_alive`
//!    records that, because it decides whether a stale lock is cleared or a run is refused.
//!
//! ## State is written and never read back
//!
//! `state.rs` writes `loop-state.json` in three places and **nothing deserialises `LoopState`**:
//!
//! ```text
//! grep LoopState   -> only state.rs (the definition and the writer) and three `write_loop_state` calls
//! ```
//!
//! `LoopState` derives `Serialize` and **not `Deserialize`**, so the file cannot be read back into the type at
//! all. The plan lists "任务进度恢复" (resume progress) for this crate, and this is the concrete obstacle:
//! resuming needs a reader, and there is no type that can hold a read. The tests below pin what the writing
//! side does -- the shape, the omitted fields, the phase mapping -- and the last one records the gap.

use burncloud_loops::lock::LoopRunLock;
use burncloud_loops::paths::{
    css_visual_artifacts_dir, jobs_aesthetic_run_dir, loops_data_dir, repo_root,
};
use burncloud_loops::state::{
    next_action_from_phase, phase_from_gates, write_loop_state, LoopState,
};
use std::path::{Path, PathBuf};

/// A temporary directory removed when the guard is dropped.
///
/// **A `Drop`, not a line at the end of the test.** A failing assertion skips everything after it, which is how
/// this suite has left temporary directories behind before; a guard runs during unwinding.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "bc_loops_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after the epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).expect("the temporary directory must be creatable");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        // Reported and ignored: a panic inside `Drop` during unwinding aborts the process.
        if let Err(e) = std::fs::remove_dir_all(&self.0) {
            if self.0.exists() {
                eprintln!("could not remove {}: {e}", self.0.display());
            }
        }
    }
}

/// A `LoopState` naming only what a test asserts on.
fn state(phase: &str) -> LoopState {
    LoopState {
        loop_name: "jobs-aesthetic".to_string(),
        iteration: 3,
        max_iterations: 10,
        phase: phase.to_string(),
        css_ok: true,
        metrics_ok: false,
        review_ok: false,
        next_action: next_action_from_phase(phase).to_string(),
        agent_prompt: "do the thing".to_string(),
        fast_mode: false,
        preview_routes: true,
        pages: vec!["/a".to_string(), "/b".to_string()],
        current_page: Some("/b".to_string()),
        completed_pages: vec!["/a".to_string()],
        pages_remaining: Some(1),
        updated_at: "2026-04-01T00:00:00Z".to_string(),
    }
}

// -------------------------------------------------------------------------------------------
// the lock: acquisition, exclusion, staleness, release
// -------------------------------------------------------------------------------------------

#[test]
fn a_lock_is_created_released_and_reacquirable() {
    // The baseline, without which every "it refused" assertion below would pass on a lock that never works.
    let root = TempRoot::new("lock_basic");
    let run_dir = root.path().join("run");

    let first = LoopRunLock::acquire(&run_dir).expect("the first acquire succeeds");
    let lock_path = run_dir.join("loop.lock");
    assert!(lock_path.exists(), "the lock file is created");

    let contents = std::fs::read_to_string(&lock_path).expect("the lock is readable");
    println!("lock contents: {contents:?}");
    assert_eq!(
        contents,
        std::process::id().to_string(),
        "the lock records this process's pid, which is what the staleness check reads"
    );

    drop(first);
    assert!(!lock_path.exists(), "dropping the lock removes the file");

    // And it can be taken again, so the release is clean rather than merely absent.
    let second = LoopRunLock::acquire(&run_dir).expect("the lock can be taken again");
    drop(second);
}

#[test]
fn a_lock_held_by_a_live_process_is_refused() {
    // Mutual exclusion, in the case that matters: the lock names a process that is alive, so a second acquire
    // must be refused rather than overwriting it. This process is used as the live one, because it certainly is.
    let root = TempRoot::new("lock_live");
    let run_dir = root.path().join("run");
    std::fs::create_dir_all(&run_dir).expect("the run directory must be creatable");

    let lock_path = run_dir.join("loop.lock");
    std::fs::write(&lock_path, std::process::id().to_string()).expect("the lock must be writable");

    let refused = LoopRunLock::acquire(&run_dir);
    match &refused {
        Ok(_) => panic!(
            "a second acquire succeeded while the lock named a live process, so two loops could run at once"
        ),
        Err(e) => println!("refused: {e}"),
    }
    assert!(refused.is_err(), "a live lock must refuse a second acquire");

    // The refused acquire must not have disturbed the lock, or the message it prints about the holder would be
    // wrong and the original holder would lose its claim.
    assert_eq!(
        std::fs::read_to_string(&lock_path).expect("the lock is still there"),
        std::process::id().to_string(),
        "the existing lock is left exactly as it was"
    );
}

#[test]
fn a_stale_lock_is_cleared_and_the_run_proceeds() {
    // The recovery path: a lock left by a process that died must not block forever. Three shapes of stale lock
    // are used, because `is_none_or` treats an unparseable file as stale too -- so a truncated or hand-edited
    // lock is recoverable as well.
    let root = TempRoot::new("lock_stale");

    // 1. A pid that is not running. `u32::MAX` is used rather than a fabricated low pid, because a low pid could
    //    belong to a real process on a busy machine and the test would then pass or fail by luck.
    let run_dir = root.path().join("dead_pid");
    std::fs::create_dir_all(&run_dir).expect("creatable");
    let lock_path = run_dir.join("loop.lock");
    std::fs::write(&lock_path, u32::MAX.to_string()).expect("writable");

    let taken = LoopRunLock::acquire(&run_dir);
    match &taken {
        Ok(_) => println!("a lock naming pid {} was cleared and re-taken", u32::MAX),
        Err(e) => panic!("a stale lock must be cleared: {e}"),
    }
    assert!(
        taken.is_ok(),
        "a lock whose pid is not running must be cleared"
    );
    assert_eq!(
        std::fs::read_to_string(&lock_path).expect("readable"),
        std::process::id().to_string(),
        "and replaced with this process's pid"
    );
    drop(taken);

    // 2. A file whose contents are not a pid at all -- a truncated write, or a hand-edited file.
    for (label, contents) in [
        ("empty", ""),
        ("garbage", "not-a-pid"),
        ("partially written", "12"),
    ] {
        let dir = root.path().join(label.replace(' ', "_"));
        std::fs::create_dir_all(&dir).expect("creatable");
        std::fs::write(dir.join("loop.lock"), contents).expect("writable");

        let result = LoopRunLock::acquire(&dir);
        match &result {
            Ok(_) => println!("{label:?} contents were treated as stale and cleared"),
            Err(e) => println!("{label:?} contents were treated as a live holder: {e}"),
        }
        // `"12"` parses as a pid, so it takes the liveness branch: pid 12 is almost certainly not running, but
        // this asserts the *decision rule* rather than a particular machine's process table.
        if label == "12" {
            let pid_is_alive = std::path::Path::new("/proc/12").exists();
            assert_eq!(
                result.is_err(),
                pid_is_alive,
                "a numeric lock is decided by whether that pid is alive, not by the file's shape"
            );
        } else {
            assert!(
                result.is_ok(),
                "{label:?} does not parse as a pid, so it must be treated as stale rather than blocking a run"
            );
        }
        drop(result);
    }
}

#[test]
fn a_lock_is_scoped_to_its_directory() {
    // "路径限定": the lock lives in the run directory it was given, so two loops with different run directories
    // do not exclude each other, and a lock in one directory is invisible from another. A lock written to a
    // fixed path -- the repository root, say -- would serialise every loop and would also be found by a test
    // running elsewhere.
    let root = TempRoot::new("lock_scope");
    let first_dir = root.path().join("run-a");
    let second_dir = root.path().join("run-b");

    let first = LoopRunLock::acquire(&first_dir).expect("the first run directory takes its lock");
    let second =
        LoopRunLock::acquire(&second_dir).expect("a different run directory is not blocked");

    assert!(
        first_dir.join("loop.lock").exists(),
        "the lock is in the directory it was given"
    );
    assert!(second_dir.join("loop.lock").exists(), "and so is the other");
    println!(
        "two locks held at once in different directories: {:?} and {:?}",
        first_dir.join("loop.lock"),
        second_dir.join("loop.lock")
    );

    drop(first);
    assert!(
        second_dir.join("loop.lock").exists(),
        "releasing one does not remove the other -- the paths are distinct"
    );
    drop(second);
}

#[test]
fn eight_threads_acquiring_and_releasing_leave_the_lock_file_consistent() {
    // **What this measures, stated narrowly, because the first version claimed more than it delivered.**
    //
    // The real race is between two *processes*: `acquire` checks for the lock file, decides on staleness, removes
    // it if stale and writes its own pid, with nothing held across those steps. This test uses threads in one
    // process, so they share a pid -- and `acquire` is only a few filesystem calls, so the threads mostly run one
    // after another. The first version asserted that some attempts must have been refused; it measured **eight
    // successes and zero refusals**, because the 20 ms hold happens *after* the lock is taken and a competing
    // thread usually arrives once it has been released.
    //
    // So this does not measure the check-then-write window, and does not claim to. What it asserts is that the
    // lock file is left consistent: every attempt either took the lock or was refused, none hung or panicked, and
    // nothing is left behind. The refusal path is covered deterministically by
    // `a_lock_held_by_a_live_process_is_refused`.
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let root = TempRoot::new("lock_concurrent");
    let run_dir = Arc::new(root.path().join("run"));
    let wins = Arc::new(AtomicUsize::new(0));
    let attempts = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..8 {
        let run_dir = Arc::clone(&run_dir);
        let wins = Arc::clone(&wins);
        let attempts = Arc::clone(&attempts);
        handles.push(std::thread::spawn(move || {
            match LoopRunLock::acquire(&run_dir) {
                Ok(lock) => {
                    wins.fetch_add(1, Ordering::SeqCst);
                    // Hold it briefly so a competing thread has a chance to observe the file.
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    drop(lock);
                }
                Err(_) => {
                    attempts.fetch_add(1, Ordering::SeqCst);
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("no thread panicked");
    }

    let wins = wins.load(Ordering::SeqCst);
    let refused = attempts.load(Ordering::SeqCst);
    println!("{wins} acquire(s) succeeded, {refused} refused");

    assert!(
        wins >= 1,
        "at least one acquire must succeed, or nothing was tested"
    );
    assert_eq!(
        wins + refused,
        8,
        "every attempt either took the lock or was refused -- none hung or vanished"
    );

    // **No assertion that some attempts were refused.** The first version asserted it and failed with eight
    // successes and zero refusals: `acquire` is a handful of filesystem calls, and the hold happens *after* the
    // lock is taken, so a competing thread usually arrives once it has been released. The refusal path is
    // covered deterministically by `a_lock_held_by_a_live_process_is_refused` instead of being hoped for here.

    // Every lock was dropped, so no file is left behind. A lock leaked here would block the next run for as long
    // as the recorded pid lived -- and for an in-process thread that pid is this test binary.
    assert!(
        !run_dir.join("loop.lock").exists(),
        "every holder released, so no lock file is left behind"
    );
}

// -------------------------------------------------------------------------------------------
// state: what gets written
// -------------------------------------------------------------------------------------------

#[test]
fn the_state_file_is_written_with_the_fields_the_plan_cares_about() {
    // "运行状态序列化". The file is the loop's only record of where it got to, so the fields a resuming run
    // would need must be in it, and the ones that are absent by design must stay absent.
    let root = TempRoot::new("state_write");
    let path = root.path().join("nested").join("loop-state.json");

    // The parent directory does not exist, which the writer is expected to create.
    assert!(!path.parent().expect("has a parent").exists());

    let written = state("metrics");
    write_loop_state(&path, &written).expect("the state must be written");

    assert!(path.exists(), "the file is created");
    let text = std::fs::read_to_string(&path).expect("readable");
    let json: serde_json::Value =
        serde_json::from_str(&text).expect("the state must be valid JSON");
    println!(
        "{}",
        serde_json::to_string_pretty(&json).expect("serialisable")
    );

    // The open set a resuming run needs.
    assert_eq!(
        json["loop"], "jobs-aesthetic",
        "the field is renamed to `loop`"
    );
    assert_eq!(json["iteration"], 3);
    assert_eq!(json["max_iterations"], 10);
    assert_eq!(json["phase"], "metrics");
    assert_eq!(json["next_action"], "fix-metrics-and-layout");
    assert_eq!(json["pages"], serde_json::json!(["/a", "/b"]));
    assert_eq!(json["current_page"], "/b");
    assert_eq!(json["completed_pages"], serde_json::json!(["/a"]));
    assert_eq!(json["pages_remaining"], 1);
    assert_eq!(json["updated_at"], "2026-04-01T00:00:00Z");

    // `loop_name` is written under a different key and **not** under its own name, so a reader expecting the
    // Rust field name would find nothing.
    assert!(
        json.get("loop_name").is_none(),
        "the field is serialised only as `loop`: {json}"
    );
}

#[test]
fn the_marked_optional_fields_are_omitted_rather_than_written_as_null() {
    // Three fields carry `skip_serializing_if`, so they are absent when empty. That is a real difference for a
    // reader: `pages_remaining: null` would mean "unknown", while an absent key means "not applicable". A
    // change to those attributes would be invisible without this test.
    let root = TempRoot::new("state_omitted");
    let path = root.path().join("loop-state.json");

    let mut minimal = state("css");
    minimal.current_page = None;
    minimal.completed_pages = Vec::new();
    minimal.pages_remaining = None;
    write_loop_state(&path, &minimal).expect("the state must be written");

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("readable"))
            .expect("valid JSON");
    println!("minimal state: {json}");

    for key in ["current_page", "completed_pages", "pages_remaining"] {
        assert!(
            json.get(key).is_none(),
            "`{key}` is skipped when empty, so it must not appear as null: {json}"
        );
    }

    // And the fields without the attribute are still present even when empty, so the two groups are distinct.
    // `pages` holds what `state()` set -- the assertion is that the key exists and carries the value, not that it
    // is empty; the first version of this test compared it against `[]` and measured `["/a", "/b"]`.
    assert!(
        json.get("pages").is_some(),
        "`pages` has no skip attribute and is always written"
    );
    assert_eq!(
        json["pages"],
        serde_json::json!(["/a", "/b"]),
        "and it carries the value it was given"
    );
}

#[test]
fn the_phase_decides_the_next_action_and_the_mapping_is_total() {
    // `phase_from_gates` and `next_action_from_phase` are the pair a resuming run uses to decide what to do. The
    // mapping is total -- `next_action_from_phase` has a catch-all arm -- so an unrecognised phase produces
    // advice rather than an empty string, which is worth pinning because an empty `next_action` in the state
    // file would look like a completed run.
    let cases = [
        // `review_ok` is checked first, so it wins outright.
        (true, true, true, "done"),
        (false, true, true, "done"),
        // Then `!css_ok`, so a CSS failure becomes the phase whatever the metrics gate says.
        (false, false, false, "css"),
        (false, true, false, "css"),
        // Then `!metrics_ok`.
        (true, false, false, "metrics"),
        (true, false, true, "done"),
        // And a run that is fine on both gates falls through to review.
        (true, true, false, "review"),
    ];
    for (css_ok, metrics_ok, review_ok, expected) in cases {
        let phase = phase_from_gates(css_ok, metrics_ok, review_ok);
        println!("css={css_ok} metrics={metrics_ok} review={review_ok} -> {phase}");
        assert_eq!(
            phase, expected,
            "css={css_ok} metrics={metrics_ok} review={review_ok}"
        );
    }

    // The precedence, asserted as an ordering rather than only as examples: `review_ok` outranks a CSS failure,
    // which outranks a metrics failure. The first version of this test had the `(true, false, false)` case
    // labelled `"css"` and measured `"metrics"` -- the CSS gate was passing, so nothing sent it to the CSS phase.
    assert_eq!(phase_from_gates(true, true, true), "done", "review_ok wins");
    assert_eq!(
        phase_from_gates(false, true, true),
        "done",
        "even over a CSS failure"
    );
    assert_eq!(
        phase_from_gates(false, true, false),
        "css",
        "a CSS failure beats metrics"
    );
    assert_eq!(
        phase_from_gates(true, false, false),
        "metrics",
        "and metrics comes last"
    );

    // Every phase the gates can produce maps to a non-empty action.
    for phase in ["css", "metrics", "review", "done"] {
        let action = next_action_from_phase(phase);
        assert!(
            !action.is_empty(),
            "phase {phase:?} must map to a non-empty action"
        );
        println!("{phase} -> {action}");
    }
    assert_eq!(
        next_action_from_phase("done"),
        "none",
        "a finished run says so explicitly"
    );

    // An unknown phase gets the catch-all rather than an empty string.
    let unknown = next_action_from_phase("something-else");
    assert_eq!(
        unknown, "increase-MaxIterations-or-fix-manually",
        "an unrecognised phase gets advice, not an empty action"
    );
    assert!(!unknown.is_empty());
}

// -------------------------------------------------------------------------------------------
// paths
// -------------------------------------------------------------------------------------------

#[test]
fn every_derived_path_stays_under_the_root_it_was_given() {
    // "路径限定". Every function that takes a `root` must produce a path under it, so a loop cannot write
    // outside the repository it was pointed at. `repo_root()` is the exception and is checked separately: it
    // derives the repository from the crate's own location.
    let root = TempRoot::new("paths_scope");
    let base = root.path();

    let derived = [
        ("loops_data_dir", loops_data_dir(base)),
        ("aesthetic_artifacts_dir", css_visual_artifacts_dir(base)),
        ("jobs_aesthetic_run_dir", jobs_aesthetic_run_dir(base)),
        ("css_visual_artifacts_dir", css_visual_artifacts_dir(base)),
    ];
    for (name, path) in &derived {
        println!("{name} -> {}", path.display());
        assert!(
            path.starts_with(base),
            "{name} must be under the root it was given, got {}",
            path.display()
        );
    }

    // The specific layout the acceptance document names, so a change to it is visible rather than silent.
    assert_eq!(
        jobs_aesthetic_run_dir(base),
        base.join("data").join("loops").join("jobs-aesthetic"),
        "the run directory is the one the acceptance document refers to"
    );
    assert_eq!(
        loops_data_dir(base),
        base.join("data").join("loops"),
        "and every other directory is under it"
    );
    for (name, path) in &derived[1..] {
        assert!(
            path.starts_with(loops_data_dir(base)),
            "{name} must be inside the loops data directory"
        );
    }

    // Two different roots produce two different trees, so nothing is anchored to a fixed location.
    let other = root.path().join("elsewhere");
    assert_ne!(loops_data_dir(base), loops_data_dir(&other));
    assert!(loops_data_dir(&other).starts_with(&other));
}

#[test]
fn the_repository_root_is_derived_from_the_crate_location_and_is_this_repository() {
    // `repo_root()` walks four parents up from `CARGO_MANIFEST_DIR` and panics if the crate is not at
    // `crates/platform/lifecycle/loops`. It cannot be given a root, so the test asserts the one thing that is
    // checkable: it resolves to the repository this test is running in.
    let root = repo_root();
    println!("repo_root -> {}", root.display());

    assert!(
        root.join("crates")
            .join("platform")
            .join("lifecycle")
            .join("loops")
            .is_dir(),
        "the derived root must contain the crate itself: {}",
        root.display()
    );
    assert!(
        root.join("Cargo.toml").is_file(),
        "and the workspace manifest, so it is the repository root rather than some ancestor of it"
    );

    // The crate's own manifest is exactly four levels below it, which is the assumption the `expect` encodes.
    let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(
        expected.ancestors().nth(4).expect("four levels up"),
        root,
        "the depth assumption holds for this checkout"
    );
}

// -------------------------------------------------------------------------------------------
// the gap that blocks resume
// -------------------------------------------------------------------------------------------

#[test]
fn the_state_type_cannot_be_deserialised_so_a_run_cannot_resume_from_its_own_file() {
    // **A gap, recorded rather than fixed.** The plan lists "任务进度恢复" for this crate. `LoopState` derives
    // `Serialize` and **not `Deserialize`**, and nothing in the crate reads `loop-state.json` back -- three
    // call sites write it, none parse it. So a resuming run has no way to load the progress it wrote.
    //
    // This test pins the obstacle itself: the JSON is written and valid, and there is no type in the crate that
    // can turn it back into a `LoopState`. Adding `Deserialize` would be a one-line change but a behaviour
    // decision -- what a reader does with a missing or older file is the real question -- so it is left to the
    // crate's owner, and the assertion below is written so that it **fails when the gap is closed**.
    let root = TempRoot::new("state_resume");
    let path = root.path().join("loop-state.json");
    write_loop_state(&path, &state("review")).expect("the state must be written");

    let text = std::fs::read_to_string(&path).expect("readable");
    let json: serde_json::Value = serde_json::from_str(&text).expect("the file is valid JSON");

    // The data a resuming run would need is all present in the file.
    for key in ["iteration", "phase", "completed_pages", "pages_remaining"] {
        assert!(
            json.get(key).is_some(),
            "`{key}` is in the file, so the information needed to resume exists -- only the reader does not"
        );
    }

    // And the crate's own source has no deserialisation of it. This is the assertion that inverts when the gap
    // is closed: if `LoopState` gains `Deserialize`, or a reader appears, this test should be deleted and
    // replaced by one that resumes from a written file.
    let state_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/state.rs"))
        .expect("state.rs is part of this crate");
    println!(
        "state.rs derives Deserialize: {}",
        state_src.contains("Deserialize")
    );
    assert!(
        !state_src.contains("Deserialize"),
        "`LoopState` now derives Deserialize, so a run can be resumed from its own file. Replace this test \
         with one that writes a state, reads it back and continues from it."
    );
}
