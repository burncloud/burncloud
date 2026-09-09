use super::model::OverviewModel;

pub const LOW_BALANCE_THRESHOLD: f64 = 20.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BalanceState {
    Healthy,
    Low,
    Critical,
    Unknown,
}

impl BalanceState {
    pub fn from_balance(balance: Option<f64>) -> Self {
        match balance {
            None => Self::Unknown,
            Some(value) if value <= 0.0 => Self::Critical,
            Some(value) if value < LOW_BALANCE_THRESHOLD => Self::Low,
            Some(_) => Self::Healthy,
        }
    }

    pub fn needs_attention(self) -> bool {
        matches!(self, Self::Low | Self::Critical)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OverviewState {
    pub model: OverviewModel,
    pub balance_state: BalanceState,
    pub search: String,
}

impl Default for OverviewState {
    fn default() -> Self {
        let model = OverviewModel::mock();
        let balance_state = BalanceState::from_balance(model.prepaid_balance);
        Self {
            model,
            balance_state,
            search: String::new(),
        }
    }
}
