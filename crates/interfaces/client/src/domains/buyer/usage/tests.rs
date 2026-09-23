use super::{
    actions::progress_width,
    model::{UsageBarTone, UsageMetricKind, UsageModel},
    state::{UsagePeriod, UsageState},
};
use crate::app::router::routes::Route;

#[test]
fn default_state_selects_the_reference_seven_day_period() {
    assert_eq!(UsageState::default().period, UsagePeriod::SevenDays);
}

#[test]
fn every_time_range_can_be_selected_without_changing_the_data_model() {
    let mut state = UsageState::default();

    for period in UsagePeriod::ALL {
        state.set_period(period);
        assert_eq!(state.period, period);
    }

    let usage = UsageModel::reference();
    assert_eq!(usage.metrics[0].value, "14.82M");
    assert_eq!(usage.rows[0].requests, "114,200");
}

#[test]
fn fixed_reference_data_matches_the_usage_page() {
    let usage = UsageModel::reference();

    assert_eq!(usage.metrics.len(), 4);
    assert_eq!(usage.metrics[0].kind, UsageMetricKind::Tokens);
    assert_eq!(usage.metrics[3].kind, UsageMetricKind::Latency);
    assert_eq!(usage.breakdown.len(), 4);
    assert_eq!(usage.breakdown[0].percent, 57);
    assert_eq!(usage.breakdown[1].tone, UsageBarTone::Indigo);
    assert_eq!(usage.breakdown[3].cost, "$10.80");
    assert_eq!(usage.rows.len(), 3);
    assert_eq!(usage.rows[2].model, "Qwen 2.5 72B (Standard)");
}

#[test]
fn progress_width_is_clamped_for_safe_inline_styles() {
    assert_eq!(progress_width(57), "width: 57%");
    assert_eq!(progress_width(100), "width: 100%");
}

#[test]
fn usage_route_parses_to_the_dedicated_page() {
    assert!(matches!(
        "/buyer/usage".parse::<Route>(),
        Ok(Route::Usage {})
    ));
}
