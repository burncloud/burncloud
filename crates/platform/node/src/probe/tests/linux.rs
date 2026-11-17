use super::{classify_vendor, is_drm_card};
use crate::AcceleratorKind;

#[test]
fn drm_card_names_exclude_connectors_and_render_nodes() {
    for name in ["card0", "card12"] {
        assert!(is_drm_card(name), "{name}");
    }
    for name in ["card", "card0-HDMI-A-1", "renderD128", "carda", "other0"] {
        assert!(!is_drm_card(name), "{name}");
    }
}

#[test]
fn pci_vendor_ids_preserve_unknown_devices() {
    assert_eq!(classify_vendor("0x1002"), AcceleratorKind::Amd);
    assert_eq!(classify_vendor("0x8086"), AcceleratorKind::Intel);
    assert_eq!(classify_vendor("0xffff"), AcceleratorKind::Other);
}
