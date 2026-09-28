use super::{actions::validate_top_up_amount, model::BillingModel};

pub const LOW_BALANCE_THRESHOLD: f64 = 20.0;
pub const MIN_TOP_UP_AMOUNT: f64 = 10.0;
pub const PRESET_TOP_UP_AMOUNTS: [f64; 5] = [50.0, 100.0, 250.0, 500.0, 1000.0];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BalanceState {
    Healthy,
    Low,
    Critical,
    Unavailable,
}

impl BalanceState {
    pub fn from_balance(balance: Option<f64>) -> Self {
        match balance {
            None => Self::Unavailable,
            Some(value) if !value.is_finite() => Self::Unavailable,
            Some(value) if value <= 0.0 => Self::Critical,
            Some(value) if value < LOW_BALANCE_THRESHOLD => Self::Low,
            Some(_) => Self::Healthy,
        }
    }

    pub const fn needs_attention(self) -> bool {
        matches!(self, Self::Low | Self::Critical)
    }

    pub const fn is_alert(self) -> bool {
        matches!(self, Self::Low | Self::Critical | Self::Unavailable)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopUpError {
    Unavailable,
    InvalidAmount,
    BelowMinimum,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BillingState {
    pub balance: Option<f64>,
    pub today_spend: f64,
    pub selected_amount: f64,
    pub custom_amount: String,
    pub modal_open: bool,
    pub auto_recharge_enabled: bool,
    pub success_message: Option<String>,
    pub success_message_version: u64,
    pub validation_error: Option<TopUpError>,
}

impl Default for BillingState {
    fn default() -> Self {
        let model = BillingModel::reference();
        Self {
            balance: model.prepaid_balance,
            today_spend: model.today_spend,
            selected_amount: model.auto_top_up_amount,
            custom_amount: String::new(),
            modal_open: false,
            auto_recharge_enabled: true,
            success_message: None,
            success_message_version: 0,
            validation_error: None,
        }
    }
}

impl BillingState {
    pub fn balance_state(&self) -> BalanceState {
        BalanceState::from_balance(self.balance)
    }

    pub fn open_modal(&mut self) {
        self.modal_open = true;
        self.validation_error = None;
    }

    pub fn close_modal(&mut self) {
        self.modal_open = false;
        self.custom_amount.clear();
        self.validation_error = None;
    }

    pub fn select_preset(&mut self, amount: f64) {
        self.selected_amount = amount;
        self.custom_amount.clear();
        self.validation_error = None;
    }

    pub fn set_custom_amount(&mut self, value: String) {
        self.custom_amount = value;
        self.validation_error = None;
    }

    pub fn toggle_auto_recharge(&mut self) {
        self.auto_recharge_enabled = !self.auto_recharge_enabled;
    }

    pub fn amount_for_display(&self) -> f64 {
        if self.custom_amount.trim().is_empty() {
            self.selected_amount
        } else {
            self.custom_amount.trim().parse::<f64>().unwrap_or(f64::NAN)
        }
    }

    pub fn submit_top_up(&mut self) -> Result<f64, TopUpError> {
        let amount = self.amount_for_display();
        if let Err(error) = validate_top_up_amount(amount) {
            self.validation_error = Some(error);
            return Err(error);
        }
        let Some(balance) = self.balance.filter(|value| value.is_finite()) else {
            self.validation_error = Some(TopUpError::Unavailable);
            return Err(TopUpError::Unavailable);
        };

        self.balance = Some(balance + amount);
        self.modal_open = false;
        self.custom_amount.clear();
        self.validation_error = None;
        Ok(amount)
    }

    pub fn set_success_message(&mut self, message: String) {
        self.success_message_version = self.success_message_version.wrapping_add(1);
        self.success_message = Some(message);
    }

    pub fn clear_success_message(&mut self) {
        self.success_message = None;
    }

    pub fn clear_success_message_if(&mut self, expected_version: u64) {
        if self.success_message_version == expected_version {
            self.clear_success_message();
        }
    }
}
