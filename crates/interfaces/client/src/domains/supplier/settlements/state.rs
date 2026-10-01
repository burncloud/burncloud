#[derive(Clone, Debug, Default, PartialEq)]
pub struct SupplierSettlementsState {
    pub payout_modal_open: bool,
    pub success_message: Option<String>,
}

impl SupplierSettlementsState {
    pub fn open_payout_modal(&mut self) {
        self.payout_modal_open = true;
        self.success_message = None;
    }

    pub fn close_payout_modal(&mut self) {
        self.payout_modal_open = false;
    }

    pub fn confirm_payout(&mut self, message: impl Into<String>) {
        self.payout_modal_open = false;
        self.success_message = Some(message.into());
    }

    pub fn clear_success_message(&mut self) {
        self.success_message = None;
    }
}
