use super::{parse_byte_count, parse_displays};
use crate::AcceleratorKind;
use serde_json::json;

#[test]
fn displays_use_unified_memory_and_preserve_unknown_vendors() {
    let displays = json!({"SPDisplaysDataType": [
        {"spdisplays_vendor": "Apple", "sppci_model": "Apple M2"},
        {"spdisplays_vendor": "AMD (0x1002)", "sppci_model": "Radeon", "spdisplays_vram": "8 GB"},
        {"spdisplays_vendor": "Example", "sppci_model": "Unknown GPU"}
    ]});
    let devices = parse_displays(&displays, 16 << 30);
    assert_eq!(devices.len(), 3);
    assert_eq!(devices[0].kind, AcceleratorKind::Apple);
    assert_eq!(devices[0].memory_bytes, Some(16 << 30));
    assert_eq!(devices[1].kind, AcceleratorKind::Amd);
    assert_eq!(devices[1].memory_bytes, Some(8 << 30));
    assert_eq!(devices[2].kind, AcceleratorKind::Other);
    assert_eq!(devices[2].memory_bytes, None);
    assert!(parse_displays(&json!({}), 1024).is_empty());
}

#[test]
fn vram_parser_accepts_reported_units_and_rejects_invalid_values() {
    assert_eq!(parse_byte_count(&json!(4096)), Some(4096));
    assert_eq!(parse_byte_count(&json!("1,024 MB")), Some(1 << 30));
    assert_eq!(parse_byte_count(&json!("1.5 GiB")), Some(3 << 29));
    for value in [
        json!("unknown"),
        json!("-1 GB"),
        json!("NaN GB"),
        json!("8 widgets"),
        json!(null),
    ] {
        assert_eq!(parse_byte_count(&value), None);
    }
}
