use burncloud_node_runtime::{
    current_arch, current_os, detect_platform, AcceleratorKind, AcceleratorProfile, HardwareProbe,
    HardwareProbeError, HardwareProfile, Platform, SystemHardwareProbe,
};
use sysinfo::Disks;

#[test]
fn platform_helpers_match_the_compilation_target() {
    assert_eq!(current_os(), std::env::consts::OS);
    assert_eq!(current_arch(), std::env::consts::ARCH);

    let expected = match std::env::consts::OS {
        "linux" => Platform::Linux,
        "macos" => Platform::MacOS,
        "windows" => Platform::Windows,
        _ => Platform::Other,
    };
    assert_eq!(detect_platform(), expected);
}

#[test]
fn hardware_contract_round_trips_through_serde() {
    let profile = HardwareProfile {
        cpu: "Example CPU".to_owned(),
        memory_bytes: u64::MAX,
        disk_available_bytes: 0,
        accelerators: vec![AcceleratorProfile {
            kind: AcceleratorKind::Intel,
            name: "Example GPU".to_owned(),
            memory_bytes: None,
        }],
    };
    let encoded = serde_json::to_string(&profile).expect("profile should serialize");
    let decoded: HardwareProfile =
        serde_json::from_str(&encoded).expect("profile should deserialize");
    assert_eq!(decoded, profile);
    assert!(encoded.contains("\"cpu\""));
    assert!(!encoded.contains("cpu_threads"));
}

#[test]
fn public_error_retains_diagnostic_text() {
    let error = HardwareProbeError::DiskError("unavailable".to_owned());
    assert!(error.to_string().contains("unavailable"));
}

#[tokio::test]
async fn system_probe_returns_hardware_facts_on_supported_platforms() {
    let result = SystemHardwareProbe::new().inspect().await;
    if detect_platform() == Platform::Other {
        assert!(matches!(
            result,
            Err(HardwareProbeError::UnsupportedPlatform(_))
        ));
        return;
    }

    let profile = result.expect("supported platforms should return common hardware facts");
    assert!(!profile.cpu.trim().is_empty());
    assert!(profile
        .accelerators
        .iter()
        .all(|accelerator| !accelerator.name.trim().is_empty()));
}

#[tokio::test]
async fn probe_reports_only_the_current_directory_filesystem() {
    let current_dir = std::env::current_dir().expect("test working directory should be readable");
    let disks = Disks::new_with_refreshed_list();
    let expected = disks
        .list()
        .iter()
        .filter(|disk| current_dir.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().components().count())
        .map(|disk| disk.available_space())
        .expect("test directory should map to a visible filesystem");

    let profile = SystemHardwareProbe::new()
        .inspect()
        .await
        .expect("supported platforms should report disk availability");
    assert_eq!(profile.disk_available_bytes, expected);
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn apple_accelerators_report_unified_system_memory() {
    let profile = SystemHardwareProbe::new()
        .inspect()
        .await
        .expect("macOS hardware probe should succeed");
    for accelerator in profile
        .accelerators
        .iter()
        .filter(|accelerator| accelerator.kind == AcceleratorKind::Apple)
    {
        assert_eq!(accelerator.memory_bytes, Some(profile.memory_bytes));
    }
}
