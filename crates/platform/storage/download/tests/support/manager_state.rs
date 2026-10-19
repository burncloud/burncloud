use super::{DownloadStatus, MonitorState};
use std::path::PathBuf;

fn state(status: DownloadStatus) -> MonitorState {
    MonitorState {
        db_id: 1,
        gid: "gid-1".to_string(),
        url: "https://huggingface.co/org/repo/resolve/main/file".to_string(),
        dir: PathBuf::from(r"C:\project\models"),
        filename: "repo".to_string(),
        status,
        retry_count: 0,
        max_retries: 1,
        total_length: 0,
        completed_length: 0,
        download_speed: 0,
        waiters: Vec::new(),
    }
}

#[test]
fn maps_aria2_statuses_and_rejects_unknown_values() {
    let cases = [
        ("pending", DownloadStatus::Pending),
        ("active", DownloadStatus::Active),
        ("waiting", DownloadStatus::Waiting),
        ("paused", DownloadStatus::Paused),
        ("complete", DownloadStatus::Completed),
        ("completed", DownloadStatus::Completed),
        ("error", DownloadStatus::Failed),
        ("failed", DownloadStatus::Failed),
        ("removed", DownloadStatus::Cancelled),
        ("cancelled", DownloadStatus::Cancelled),
    ];

    for (value, expected) in cases {
        assert_eq!(DownloadStatus::from_aria2(value), Some(expected), "{value}");
    }
    assert_eq!(DownloadStatus::from_aria2("unknown"), None);
}

#[test]
fn recognizes_only_terminal_statuses() {
    for status in [
        DownloadStatus::Completed,
        DownloadStatus::Failed,
        DownloadStatus::Cancelled,
    ] {
        assert!(status.is_terminal());
    }
    for status in [
        DownloadStatus::Pending,
        DownloadStatus::Active,
        DownloadStatus::Waiting,
        DownloadStatus::Paused,
    ] {
        assert!(!status.is_terminal());
    }
}

#[test]
fn matches_only_same_key_for_non_terminal_tasks() {
    let active = state(DownloadStatus::Active);
    assert!(active.has_key(
        "https://huggingface.co/org/repo/resolve/main/file",
        r"C:\project\models",
        "repo"
    ));
    assert!(!active.has_key(
        "https://huggingface.co/other/repo/resolve/main/file",
        r"C:\project\models",
        "repo"
    ));
    assert!(!active.has_key(
        "https://huggingface.co/org/repo/resolve/main/file",
        r"C:\project\other",
        "repo"
    ));
    assert!(!active.has_key(
        "https://huggingface.co/org/repo/resolve/main/file",
        r"C:\project\models",
        "other"
    ));
    assert!(!state(DownloadStatus::Completed).has_key(
        "https://huggingface.co/org/repo/resolve/main/file",
        r"C:\project\models",
        "repo"
    ));
}

#[test]
fn takes_waiters_and_leaves_the_state_empty() {
    let (first_sender, first_receiver) = tokio::sync::oneshot::channel();
    let (second_sender, second_receiver) = tokio::sync::oneshot::channel();
    let mut task = state(DownloadStatus::Active);
    task.waiters = vec![first_sender, second_sender];

    let waiters = task.take_waiters();

    assert_eq!(waiters.len(), 2);
    assert!(task.waiters.is_empty());
    drop(waiters);
    drop(first_receiver);
    drop(second_receiver);
}
