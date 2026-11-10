//! Interactive state for the administrator platform settings page.

use super::model::AdminSettingsModel;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminSettingsState {
    pub base_rev_share: String,
    pub l4_rev_share: String,
    pub target_headroom: String,
    pub saved: bool,
}

impl Default for AdminSettingsState {
    fn default() -> Self {
        let model = AdminSettingsModel::default();
        Self {
            base_rev_share: model.base_rev_share.to_string(),
            l4_rev_share: model.l4_rev_share.to_string(),
            target_headroom: model.target_headroom.to_string(),
            saved: false,
        }
    }
}

impl AdminSettingsState {
    pub fn set_base_rev_share(&mut self, value: String) {
        self.base_rev_share = value;
    }

    pub fn set_l4_rev_share(&mut self, value: String) {
        self.l4_rev_share = value;
    }

    pub fn set_target_headroom(&mut self, value: String) {
        self.target_headroom = value;
    }

    pub fn mark_saved(&mut self) {
        self.saved = true;
    }

    pub fn clear_saved(&mut self) {
        self.saved = false;
    }
}
