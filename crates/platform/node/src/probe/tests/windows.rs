use super::{parse_adapter_ram, parse_cim_devices};
use crate::AcceleratorKind;
use serde_json::json;

#[test]
fn cim_accepts_a_single_controller_and_vendor_fallback() {
    let devices = parse_cim_devices(&json!({
        "Name": "  Integrated Graphics  ",
        "AdapterCompatibility": "Intel Corporation",
        "AdapterRAM": 2147483648_u64
    }));
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].kind, AcceleratorKind::Intel);
    assert_eq!(devices[0].name, "Integrated Graphics");
    assert_eq!(devices[0].memory_bytes, Some(2147483648));
}

#[test]
fn cim_skips_duplicates_and_invalid_records_but_keeps_unknown_devices() {
    let devices = parse_cim_devices(&json!([
        {"Name": "NVIDIA RTX", "AdapterRAM": 1024},
        {"Name": "   "},
        {"AdapterRAM": 1024},
        {"Name": "AMD Radeon", "AdapterRAM": 0},
        {"Name": "Example GPU", "AdapterRAM": 4294967295_u64}
    ]));
    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].kind, AcceleratorKind::Amd);
    assert_eq!(devices[0].memory_bytes, None);
    assert_eq!(devices[1].kind, AcceleratorKind::Other);
    assert_eq!(devices[1].name, "Example GPU");
    assert_eq!(devices[1].memory_bytes, None);
    assert!(parse_cim_devices(&json!(null)).is_empty());
}

#[test]
fn adapter_ram_rejects_missing_zero_and_overflowed_capacity() {
    assert_eq!(parse_adapter_ram(&json!(1073741824)), Some(1073741824));
    for value in [
        json!(null),
        json!(0),
        json!(-1),
        json!(4294967295_u64),
        json!(4294967296_u64),
    ] {
        assert_eq!(parse_adapter_ram(&value), None);
    }
}
