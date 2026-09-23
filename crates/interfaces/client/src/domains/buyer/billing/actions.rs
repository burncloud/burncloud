//! Pure billing helpers used by the page and its tests.

use super::state::{TopUpError, MIN_TOP_UP_AMOUNT};

pub fn validate_top_up_amount(amount: f64) -> Result<(), TopUpError> {
    if !amount.is_finite() || amount <= 0.0 {
        return Err(TopUpError::InvalidAmount);
    }
    if amount < MIN_TOP_UP_AMOUNT {
        return Err(TopUpError::BelowMinimum);
    }
    Ok(())
}

pub fn format_amount(amount: f64) -> String {
    format!("${amount:.2}")
}

pub fn remaining_days(balance: Option<f64>, today_spend: f64) -> Option<u64> {
    let balance = balance?;
    if !balance.is_finite() || !today_spend.is_finite() {
        return None;
    }
    let balance = balance.max(0.0);
    let burn_rate = today_spend.max(0.1);
    Some((balance / burn_rate).floor() as u64)
}
