//! Static defaults for the administrator platform settings page.

pub const BASE_REV_SHARE: &str = "75.0";
pub const L4_REV_SHARE: &str = "85.0";
pub const TARGET_HEADROOM: &str = "25.0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminSettingsModel {
    pub base_rev_share: &'static str,
    pub l4_rev_share: &'static str,
    pub target_headroom: &'static str,
}

impl Default for AdminSettingsModel {
    fn default() -> Self {
        Self {
            base_rev_share: BASE_REV_SHARE,
            l4_rev_share: L4_REV_SHARE,
            target_headroom: TARGET_HEADROOM,
        }
    }
}
