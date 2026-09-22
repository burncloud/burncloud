//! Pure API key creation, masking, validation, and revocation helpers.

use super::model::{ApiKeyStatus, BuyerApiKey};
use rand::Rng;
use std::fmt;

pub const DEFAULT_RATE_LIMIT_RPM: u32 = 600;
pub const DEFAULT_MONTHLY_SPEND_CAP: f64 = 500.0;
pub const MIN_RATE_LIMIT_RPM: u32 = 10;
pub const MAX_RATE_LIMIT_RPM: u32 = 5_000;
pub const MIN_MONTHLY_SPEND_CAP: f64 = 10.0;
pub const MAX_MONTHLY_SPEND_CAP: f64 = 50_000.0;
pub const SECRET_PREFIX: &str = "bc_live_";
pub const SECRET_RANDOM_LENGTH: usize = 26;

const SECRET_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
const MASK_VISIBLE_PREFIX: usize = 11;
const MASK_VISIBLE_SUFFIX: usize = 4;
const MASKED_CHARACTER_COUNT: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct CreateKeyInput {
    pub name: String,
    pub rate_limit_rpm: u32,
    pub monthly_spend_cap: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateKeyResult {
    pub key: BuyerApiKey,
    pub secret: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreateKeyError {
    NameRequired,
    RateLimitOutOfRange,
    MonthlySpendCapOutOfRange,
    InvalidSecret,
}

impl fmt::Display for CreateKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NameRequired => "API key name is required",
            Self::RateLimitOutOfRange => "rate limit must be between 10 and 5000 RPM",
            Self::MonthlySpendCapOutOfRange => "monthly spend cap must be between 10 and 50000 USD",
            Self::InvalidSecret => "generated API key has an invalid format",
        };
        formatter.write_str(message)
    }
}

pub fn validate_create_input(input: &CreateKeyInput) -> Result<(), CreateKeyError> {
    if input.name.trim().is_empty() {
        return Err(CreateKeyError::NameRequired);
    }
    if !(MIN_RATE_LIMIT_RPM..=MAX_RATE_LIMIT_RPM).contains(&input.rate_limit_rpm) {
        return Err(CreateKeyError::RateLimitOutOfRange);
    }
    if !input.monthly_spend_cap.is_finite()
        || !(MIN_MONTHLY_SPEND_CAP..=MAX_MONTHLY_SPEND_CAP).contains(&input.monthly_spend_cap)
    {
        return Err(CreateKeyError::MonthlySpendCapOutOfRange);
    }
    Ok(())
}

pub fn generate_secret() -> String {
    let mut rng = rand::thread_rng();
    let random = (0..SECRET_RANDOM_LENGTH)
        .map(|_| {
            let index = rng.gen_range(0..SECRET_ALPHABET.len());
            SECRET_ALPHABET[index] as char
        })
        .collect::<String>();
    format!("{SECRET_PREFIX}{random}")
}

pub fn mask_secret(secret: &str) -> String {
    let characters = secret.chars().collect::<Vec<_>>();
    if characters.len() <= MASK_VISIBLE_PREFIX + MASK_VISIBLE_SUFFIX {
        return "\u{2022}".repeat(characters.len());
    }

    let prefix = characters[..MASK_VISIBLE_PREFIX].iter().collect::<String>();
    let suffix = characters[characters.len() - MASK_VISIBLE_SUFFIX..]
        .iter()
        .collect::<String>();
    format!(
        "{prefix}{}{suffix}",
        "\u{2022}".repeat(MASKED_CHARACTER_COUNT)
    )
}

pub fn spend_progress_percent(spend: f64, cap: f64) -> f64 {
    if !spend.is_finite() || !cap.is_finite() || cap <= 0.0 {
        return 0.0;
    }
    (spend / cap * 100.0).clamp(0.0, 100.0)
}

pub fn create_api_key(
    id: impl Into<String>,
    input: CreateKeyInput,
    secret: String,
) -> Result<CreateKeyResult, CreateKeyError> {
    validate_create_input(&input)?;
    if !is_valid_secret(&secret) {
        return Err(CreateKeyError::InvalidSecret);
    }

    let key_prefix = secret.chars().take(MASK_VISIBLE_PREFIX).collect();
    let key = BuyerApiKey {
        id: id.into(),
        name: input.name.trim().to_string(),
        key_prefix,
        masked_key: mask_secret(&secret),
        created: "Just now".to_string(),
        last_used: "Never".to_string(),
        tier: "All Tiers".to_string(),
        rate_limit_rpm: input.rate_limit_rpm,
        monthly_spend_cap: input.monthly_spend_cap,
        spend_this_month: 0.0,
        status: ApiKeyStatus::Active,
    };
    Ok(CreateKeyResult { key, secret })
}

pub fn revoke_api_key(keys: &mut [BuyerApiKey], id: &str) -> bool {
    let Some(key) = keys
        .iter_mut()
        .find(|key| key.id == id && key.status.is_active())
    else {
        return false;
    };
    key.status = ApiKeyStatus::Revoked;
    true
}

fn is_valid_secret(secret: &str) -> bool {
    let Some(random) = secret.strip_prefix(SECRET_PREFIX) else {
        return false;
    };
    random.len() == SECRET_RANDOM_LENGTH
        && random
            .bytes()
            .all(|character| SECRET_ALPHABET.contains(&character))
}
