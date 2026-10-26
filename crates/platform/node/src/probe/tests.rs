use super::common::CommonHardware;
use crate::{AcceleratorKind, AcceleratorProfile};

#[test]
fn gpu_degradation_preserves_common_hardware_facts() {
    let profile = CommonHardware {
        cpu: "Example CPU".to_owned(),
        memory_bytes: 8192,
        disk_available_bytes: 4096,
    }
    .with_accelerators(Vec::new());

    assert_eq!(profile.cpu, "Example CPU");
    assert_eq!(profile.memory_bytes, 8192);
    assert_eq!(profile.disk_available_bytes, 4096);
    assert!(profile.accelerators.is_empty());
}

#[test]
fn partial_gpu_results_are_preserved() {
    let accelerator = AcceleratorProfile {
        kind: AcceleratorKind::Other,
        name: "Detected GPU".to_owned(),
        memory_bytes: None,
    };
    let profile = CommonHardware {
        cpu: "Example CPU".to_owned(),
        memory_bytes: 8192,
        disk_available_bytes: 4096,
    }
    .with_accelerators(vec![accelerator.clone()]);

    assert_eq!(profile.accelerators, vec![accelerator]);
}
