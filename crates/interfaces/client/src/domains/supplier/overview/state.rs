use super::model::SupplierOverviewModel;

#[derive(Clone, Debug, PartialEq)]
pub struct SupplierOverviewState {
    pub model: SupplierOverviewModel,
    pub install_modal_open: bool,
    pub command_copied: bool,
}

impl Default for SupplierOverviewState {
    fn default() -> Self {
        Self {
            model: SupplierOverviewModel::mock(),
            install_modal_open: false,
            command_copied: false,
        }
    }
}

impl SupplierOverviewState {
    pub fn open_install_modal(&mut self) {
        self.install_modal_open = true;
        self.command_copied = false;
    }

    pub fn close_install_modal(&mut self) {
        self.install_modal_open = false;
        self.command_copied = false;
    }

    pub fn mark_command_copied(&mut self) {
        self.command_copied = true;
    }
}
