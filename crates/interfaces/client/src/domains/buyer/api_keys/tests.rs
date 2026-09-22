use super::{
    actions::{
        create_api_key, generate_secret, mask_secret, spend_progress_percent,
        validate_create_input, CreateKeyError, CreateKeyInput, MAX_MONTHLY_SPEND_CAP,
        MAX_RATE_LIMIT_RPM, MIN_MONTHLY_SPEND_CAP, MIN_RATE_LIMIT_RPM, SECRET_PREFIX,
        SECRET_RANDOM_LENGTH,
    },
    model::{ApiKeyStatus, BuyerApiKey},
    state::ApiKeysState,
};
use crate::app::router::routes::Route;

const FIXED_SECRET: &str = "bc_live_abcdefghijklmnopqrstuvwxyz";

#[test]
fn mock_keys_match_the_reference_page() {
    let keys = BuyerApiKey::mock_keys();

    assert_eq!(keys.len(), 3);
    assert_eq!(
        keys.iter().map(|key| key.name.as_str()).collect::<Vec<_>>(),
        vec![
            "Production Kubernetes Cluster (US-West)",
            "Dev Agent Sandbox (CI/CD)",
            "Model Benchmark & Automated Evaluation",
        ]
    );
    assert_eq!(
        keys.iter()
            .map(|key| key.rate_limit_rpm)
            .collect::<Vec<_>>(),
        vec![1_200, 300, 600]
    );
    assert_eq!(
        keys.iter()
            .map(|key| key.monthly_spend_cap)
            .collect::<Vec<_>>(),
        vec![2_500.0, 500.0, 300.0]
    );
    assert_eq!(
        keys.iter()
            .map(|key| key.spend_this_month)
            .collect::<Vec<_>>(),
        vec![842.10, 128.40, 42.15]
    );
    assert!(keys.iter().all(|key| key.status == ApiKeyStatus::Active));
}

#[test]
fn default_state_matches_the_reference_form_and_modal_state() {
    let state = ApiKeysState::default();

    assert_eq!(state.active_count(), 3);
    assert!(!state.create_modal_open);
    assert!(!state.result_modal_open);
    assert_eq!(state.new_key_name, "");
    assert_eq!(state.new_key_rate_limit, 600);
    assert_eq!(state.new_key_cap, 500.0);
    assert!(state.created_secret.is_none());
    assert!(!state.copied);
    assert!(state.validation_error.is_none());
}

#[test]
fn validation_rejects_blank_names_and_out_of_range_limits() {
    let valid = CreateKeyInput {
        name: "Production".to_string(),
        rate_limit_rpm: MIN_RATE_LIMIT_RPM,
        monthly_spend_cap: MIN_MONTHLY_SPEND_CAP,
    };
    assert_eq!(validate_create_input(&valid), Ok(()));
    assert_eq!(
        validate_create_input(&CreateKeyInput {
            name: " \t".to_string(),
            ..valid.clone()
        }),
        Err(CreateKeyError::NameRequired)
    );
    assert_eq!(
        validate_create_input(&CreateKeyInput {
            rate_limit_rpm: MIN_RATE_LIMIT_RPM - 1,
            ..valid.clone()
        }),
        Err(CreateKeyError::RateLimitOutOfRange)
    );
    assert_eq!(
        validate_create_input(&CreateKeyInput {
            rate_limit_rpm: MAX_RATE_LIMIT_RPM + 1,
            ..valid.clone()
        }),
        Err(CreateKeyError::RateLimitOutOfRange)
    );
    assert_eq!(
        validate_create_input(&CreateKeyInput {
            monthly_spend_cap: MIN_MONTHLY_SPEND_CAP - 0.01,
            ..valid.clone()
        }),
        Err(CreateKeyError::MonthlySpendCapOutOfRange)
    );
    assert_eq!(
        validate_create_input(&CreateKeyInput {
            monthly_spend_cap: MAX_MONTHLY_SPEND_CAP + 0.01,
            ..valid
        }),
        Err(CreateKeyError::MonthlySpendCapOutOfRange)
    );
}

#[test]
fn validation_accepts_inclusive_upper_limits() {
    let input = CreateKeyInput {
        name: "At limits".to_string(),
        rate_limit_rpm: MAX_RATE_LIMIT_RPM,
        monthly_spend_cap: MAX_MONTHLY_SPEND_CAP,
    };
    assert_eq!(validate_create_input(&input), Ok(()));
}

#[test]
fn masking_matches_the_reference_format_without_exposing_the_secret() {
    let masked = mask_secret(FIXED_SECRET);

    assert_eq!(masked, "bc_live_abc\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}wxyz");
    assert!(!masked.contains("defghijklmnopqrstuv"));
    assert_eq!(
        mask_secret("short"),
        "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"
    );
}

#[test]
fn spend_progress_is_clamped_and_handles_invalid_caps() {
    assert!((spend_progress_percent(842.10, 2_500.0) - 33.684).abs() < 1e-10);
    assert_eq!(spend_progress_percent(-5.0, 100.0), 0.0);
    assert_eq!(spend_progress_percent(150.0, 100.0), 100.0);
    assert_eq!(spend_progress_percent(25.0, 0.0), 0.0);
    assert_eq!(spend_progress_percent(f64::NAN, 100.0), 0.0);
    assert_eq!(spend_progress_percent(25.0, f64::INFINITY), 0.0);
}

#[test]
fn creation_result_contains_a_display_record_and_one_time_secret() {
    let result = create_api_key(
        "key-local-4",
        CreateKeyInput {
            name: "  Production Cluster  ".to_string(),
            rate_limit_rpm: 900,
            monthly_spend_cap: 750.0,
        },
        FIXED_SECRET.to_string(),
    )
    .expect("valid API key");

    assert_eq!(result.secret, FIXED_SECRET);
    assert_eq!(result.key.name, "Production Cluster");
    assert_eq!(result.key.key_prefix, "bc_live_abc");
    assert_eq!(result.key.masked_key, mask_secret(FIXED_SECRET));
    assert_eq!(result.key.created, "Just now");
    assert_eq!(result.key.last_used, "Never");
    assert_eq!(result.key.rate_limit_rpm, 900);
    assert_eq!(result.key.monthly_spend_cap, 750.0);
    assert_eq!(result.key.status, ApiKeyStatus::Active);
    assert!(!result.key.id.contains(FIXED_SECRET));
    assert!(!result.key.masked_key.contains(FIXED_SECRET));
}

#[test]
fn valid_creation_is_prepended_and_preserves_numeric_form_values() {
    let mut state = ApiKeysState::default();
    state.open_create_modal();
    state.new_key_name = "My service".to_string();
    state.new_key_rate_limit = 1_500;
    state.new_key_cap = 1_250.0;

    assert_eq!(
        state.create_key_with_secret(FIXED_SECRET.to_string()),
        Ok(())
    );
    assert_eq!(state.keys.len(), 4);
    assert_eq!(state.keys[0].id, "key-local-4");
    assert_eq!(state.keys[0].name, "My service");
    assert_eq!(state.new_key_name, "");
    assert_eq!(state.new_key_rate_limit, 1_500);
    assert_eq!(state.new_key_cap, 1_250.0);
    assert!(!state.create_modal_open);
    assert!(state.result_modal_open);
    assert_eq!(state.created_secret.as_deref(), Some(FIXED_SECRET));
    assert_eq!(state.active_count(), 4);
}

#[test]
fn failed_creation_does_not_mutate_the_key_list_or_open_result() {
    let mut state = ApiKeysState::default();
    state.open_create_modal();
    state.new_key_name = "   ".to_string();

    assert_eq!(
        state.create_key_with_secret(FIXED_SECRET.to_string()),
        Err(CreateKeyError::NameRequired)
    );
    assert_eq!(state.keys.len(), 3);
    assert!(state.create_modal_open);
    assert!(!state.result_modal_open);
    assert!(state.created_secret.is_none());
    assert_eq!(state.validation_error, Some(CreateKeyError::NameRequired));
}

#[test]
fn closing_result_irrecoverably_clears_the_one_time_secret_and_copy_state() {
    let mut state = ApiKeysState::default();
    state.new_key_name = "One-time key".to_string();
    state
        .create_key_with_secret(FIXED_SECRET.to_string())
        .expect("valid API key");
    state.mark_copied();
    assert!(state.copied);

    state.close_result_modal();

    assert!(!state.result_modal_open);
    assert!(state.created_secret.is_none());
    assert!(!state.copied);
    assert!(state
        .keys
        .iter()
        .all(|key| key.masked_key != FIXED_SECRET && key.id != FIXED_SECRET));
}

#[test]
fn copy_feedback_can_be_cleared_without_closing_the_result() {
    let mut state = ApiKeysState::default();
    state.new_key_name = "Copy feedback".to_string();
    state
        .create_key_with_secret(FIXED_SECRET.to_string())
        .expect("valid API key");

    state.mark_copied();
    assert!(state.copied);
    state.clear_copied();

    assert!(!state.copied);
    assert!(state.result_modal_open);
    assert_eq!(state.created_secret.as_deref(), Some(FIXED_SECRET));
}

#[test]
fn revoke_is_immediate_and_idempotent() {
    let mut state = ApiKeysState::default();

    assert!(state.revoke_key("key-dev-agent"));
    assert_eq!(state.active_count(), 2);
    assert_eq!(state.keys[1].status, ApiKeyStatus::Revoked);
    assert!(!state.revoke_key("key-dev-agent"));
    assert_eq!(state.active_count(), 2);
    assert!(!state.revoke_key("missing-key"));
}

#[test]
fn generated_secrets_have_the_expected_prefix_length_and_alphabet() {
    let secret = generate_secret();
    let random = secret.strip_prefix(SECRET_PREFIX).expect("secret prefix");

    assert_eq!(random.len(), SECRET_RANDOM_LENGTH);
    assert!(random
        .bytes()
        .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit()));
}

#[test]
fn api_keys_route_parses_to_its_dedicated_component() {
    assert!(matches!(
        "/buyer/api-keys".parse::<Route>(),
        Ok(Route::ApiKeys {})
    ));
}
