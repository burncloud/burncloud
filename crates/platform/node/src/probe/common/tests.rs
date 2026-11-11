use super::disk_available_for_path;
use crate::HardwareProbeError;
use std::path::Path;

#[test]
fn disk_snapshot_uses_the_deepest_matching_mount() {
    let disks = [
        (Path::new("volume/workspace/models"), 128),
        (Path::new("other-volume"), 8192),
        (Path::new("volume"), 4096),
        (Path::new("volume/workspace"), 512),
    ];

    let result = disk_available_for_path(
        Path::new("volume/workspace/models/cache"),
        disks.into_iter(),
    );
    assert!(matches!(result, Ok(128)), "{result:?}");
}

#[test]
fn disk_snapshot_matches_path_components_instead_of_string_prefixes() {
    let disks = [
        (Path::new("volume"), 4096),
        (Path::new("volume/workspace"), 512),
    ];

    let result = disk_available_for_path(
        Path::new("volume/workspace-other/models"),
        disks.into_iter(),
    );
    assert!(matches!(result, Ok(4096)), "{result:?}");
}

#[test]
fn disk_snapshot_reports_a_missing_mount() {
    let current_dir = Path::new("volume/workspace/models");
    let disks = [(Path::new("other-volume"), 8192)];

    assert!(matches!(
        disk_available_for_path(current_dir, disks.into_iter()),
        Err(HardwareProbeError::DiskError(message))
            if message.contains(&current_dir.display().to_string())
    ));
}
