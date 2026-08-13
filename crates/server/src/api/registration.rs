use bcrypt::{hash, DEFAULT_COST};
use burncloud_database::Database;
use sqlx::Any;
use uuid::Uuid;

const ACTIVE_STATUS: i32 = 1;
const BOOTSTRAP_TABLE: &str = "burncloud_bootstrap_state";

#[derive(Debug, Clone)]
pub(crate) struct PublicRegistrationInput {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub bootstrap_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublicRegistrationResult {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PublicRegistrationError {
    BootstrapRequired,
    BootstrapNotConfigured,
    InvalidBootstrapToken,
    BootstrapAlreadyComplete,
    RegistrationClosed,
    UserAlreadyExists,
    InvalidInput(String),
    Configuration(String),
    Database(String),
}

pub(crate) async fn initialize_bootstrap_state(db: &Database) -> anyhow::Result<()> {
    let connection = db.get_connection()?;
    let pool = connection.pool();
    let create_sql = if db.kind() == "postgres" {
        format!(
            "CREATE TABLE IF NOT EXISTS {BOOTSTRAP_TABLE} (\
                id SMALLINT PRIMARY KEY CHECK (id = 1), \
                completed_by TEXT NOT NULL, \
                completed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP\
            )"
        )
    } else {
        format!(
            "CREATE TABLE IF NOT EXISTS {BOOTSTRAP_TABLE} (\
                id INTEGER PRIMARY KEY CHECK (id = 1), \
                completed_by TEXT NOT NULL, \
                completed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP\
            )"
        )
    };
    sqlx::query(&create_sql).execute(pool).await?;

    // Existing installations predate the bootstrap marker. Mark them complete
    // before serving requests so an upgrade can never reopen first-admin setup.
    let existing_users: i64 = sqlx::query_scalar::<Any, i64>(
        "SELECT COUNT(*) FROM user_accounts WHERE username != 'demo-user'",
    )
    .fetch_one(pool)
    .await?;
    if existing_users > 0 {
        let backfill_sql = if db.kind() == "postgres" {
            format!(
                "INSERT INTO {BOOTSTRAP_TABLE} (id, completed_by) \
                 VALUES (1, 'existing-installation') ON CONFLICT (id) DO NOTHING"
            )
        } else {
            format!(
                "INSERT OR IGNORE INTO {BOOTSTRAP_TABLE} (id, completed_by) \
                 VALUES (1, 'existing-installation')"
            )
        };
        sqlx::query(&backfill_sql).execute(pool).await?;
    }

    Ok(())
}

pub(crate) async fn register_public_user(
    db: &Database,
    input: &PublicRegistrationInput,
) -> Result<PublicRegistrationResult, PublicRegistrationError> {
    let username = input.username.trim();
    if username.is_empty() {
        return Err(PublicRegistrationError::InvalidInput(
            "Username is required".to_string(),
        ));
    }
    if input.password.len() < 8 {
        return Err(PublicRegistrationError::InvalidInput(
            "Password must contain at least 8 characters".to_string(),
        ));
    }

    let password_hash = hash(&input.password, DEFAULT_COST)
        .map_err(|error| PublicRegistrationError::Configuration(error.to_string()))?;
    let user_id = Uuid::new_v4().to_string();
    let bootstrap_token = input
        .bootstrap_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let connection = db
        .get_connection()
        .map_err(|error| PublicRegistrationError::Database(error.to_string()))?;
    let mut transaction = connection
        .pool()
        .begin()
        .await
        .map_err(|error| PublicRegistrationError::Database(error.to_string()))?;
    let postgres = db.kind() == "postgres";

    let duplicate_sql = if postgres {
        "SELECT COUNT(*) FROM user_accounts WHERE username = $1"
    } else {
        "SELECT COUNT(*) FROM user_accounts WHERE username = ?"
    };
    let duplicate_count: i64 = sqlx::query_scalar::<Any, i64>(duplicate_sql)
        .bind(username)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| PublicRegistrationError::Database(error.to_string()))?;
    if duplicate_count > 0 {
        return Err(PublicRegistrationError::UserAlreadyExists);
    }

    let marker_sql = format!("SELECT COUNT(*) FROM {BOOTSTRAP_TABLE} WHERE id = 1");
    let bootstrap_complete: i64 = sqlx::query_scalar::<Any, i64>(&marker_sql)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| PublicRegistrationError::Database(error.to_string()))?;

    let (role, balance_usd) = if bootstrap_complete > 0 {
        if bootstrap_token.is_some() {
            return Err(PublicRegistrationError::BootstrapAlreadyComplete);
        }
        if !public_registration_open() {
            return Err(PublicRegistrationError::RegistrationClosed);
        }
        ("user", public_signup_bonus_nano()?)
    } else {
        let supplied = bootstrap_token.ok_or(PublicRegistrationError::BootstrapRequired)?;
        let expected = std::env::var("BURNCLOUD_BOOTSTRAP_TOKEN")
            .map_err(|_| PublicRegistrationError::BootstrapNotConfigured)?;
        let expected = expected.trim();
        if expected.len() < 16 {
            return Err(PublicRegistrationError::Configuration(
                "BURNCLOUD_BOOTSTRAP_TOKEN must contain at least 16 characters".to_string(),
            ));
        }
        if !constant_time_eq(supplied, expected) {
            return Err(PublicRegistrationError::InvalidBootstrapToken);
        }

        // Claim the singleton marker inside the same transaction as admin user
        // creation. The unique primary key prevents two processes from both
        // successfully completing first-admin bootstrap.
        let claim_sql = if postgres {
            format!(
                "INSERT INTO {BOOTSTRAP_TABLE} (id, completed_by) VALUES (1, $1) \
                 ON CONFLICT (id) DO NOTHING"
            )
        } else {
            format!(
                "INSERT OR IGNORE INTO {BOOTSTRAP_TABLE} (id, completed_by) VALUES (1, ?)"
            )
        };
        let claimed = sqlx::query(&claim_sql)
            .bind(&user_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| PublicRegistrationError::Database(error.to_string()))?;
        if claimed.rows_affected() != 1 {
            return Err(PublicRegistrationError::BootstrapAlreadyComplete);
        }
        ("admin", 0)
    };

    let insert_user_sql = if postgres {
        "INSERT INTO user_accounts \
         (id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency) \
         VALUES ($1, $2, $3, $4, NULL, $5, $6, 0, 'USD')"
    } else {
        "INSERT INTO user_accounts \
         (id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency) \
         VALUES (?, ?, ?, ?, NULL, ?, ?, 0, 'USD')"
    };
    sqlx::query(insert_user_sql)
        .bind(&user_id)
        .bind(username)
        .bind(input.email.clone())
        .bind(password_hash)
        .bind(ACTIVE_STATUS)
        .bind(balance_usd)
        .execute(&mut *transaction)
        .await
        .map_err(|error| PublicRegistrationError::Database(error.to_string()))?;

    let role_id_sql = if postgres {
        "SELECT id FROM user_roles WHERE name = $1"
    } else {
        "SELECT id FROM user_roles WHERE name = ?"
    };
    let role_id: Option<String> = sqlx::query_scalar::<Any, String>(role_id_sql)
        .bind(role)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| PublicRegistrationError::Database(error.to_string()))?;
    let role_id = role_id.ok_or_else(|| {
        PublicRegistrationError::Database(format!("Required role '{role}' is missing"))
    })?;

    let bind_role_sql = if postgres {
        "INSERT INTO user_role_bindings (user_id, role_id) VALUES ($1, $2)"
    } else {
        "INSERT INTO user_role_bindings (user_id, role_id) VALUES (?, ?)"
    };
    sqlx::query(bind_role_sql)
        .bind(&user_id)
        .bind(role_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| PublicRegistrationError::Database(error.to_string()))?;

    transaction
        .commit()
        .await
        .map_err(|error| PublicRegistrationError::Database(error.to_string()))?;

    Ok(PublicRegistrationResult {
        user_id,
        username: username.to_string(),
        role: role.to_string(),
    })
}

pub(crate) fn public_registration_open() -> bool {
    std::env::var("BURNCLOUD_PUBLIC_REGISTRATION")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "open" | "true" | "1" | "yes"
            )
        })
        .unwrap_or(false)
}

fn public_signup_bonus_nano() -> Result<i64, PublicRegistrationError> {
    let value = std::env::var("BURNCLOUD_PUBLIC_SIGNUP_BONUS_USD")
        .unwrap_or_else(|_| "0".to_string());
    parse_usd_nano(&value)
}

pub(crate) fn parse_usd_nano(value: &str) -> Result<i64, PublicRegistrationError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(0);
    }
    if value.starts_with('-') || value.starts_with('+') {
        return Err(PublicRegistrationError::Configuration(
            "BURNCLOUD_PUBLIC_SIGNUP_BONUS_USD must be non-negative".to_string(),
        ));
    }

    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || fraction.len() > 9
        || (!whole.is_empty() && !whole.chars().all(|ch| ch.is_ascii_digit()))
        || (!fraction.is_empty() && !fraction.chars().all(|ch| ch.is_ascii_digit()))
    {
        return Err(PublicRegistrationError::Configuration(
            "BURNCLOUD_PUBLIC_SIGNUP_BONUS_USD must contain digits and at most 9 decimal places"
                .to_string(),
        ));
    }

    let whole_value = if whole.is_empty() {
        0i64
    } else {
        whole.parse::<i64>().map_err(|_| {
            PublicRegistrationError::Configuration(
                "BURNCLOUD_PUBLIC_SIGNUP_BONUS_USD is too large".to_string(),
            )
        })?
    };
    let mut fraction_text = fraction.to_string();
    while fraction_text.len() < 9 {
        fraction_text.push('0');
    }
    let fraction_nano = if fraction_text.is_empty() {
        0i64
    } else {
        fraction_text.parse::<i64>().map_err(|_| {
            PublicRegistrationError::Configuration(
                "BURNCLOUD_PUBLIC_SIGNUP_BONUS_USD is invalid".to_string(),
            )
        })?
    };

    whole_value
        .checked_mul(1_000_000_000)
        .and_then(|base| base.checked_add(fraction_nano))
        .ok_or_else(|| {
            PublicRegistrationError::Configuration(
                "BURNCLOUD_PUBLIC_SIGNUP_BONUS_USD is too large".to_string(),
            )
        })
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();
    for index in 0..max_len {
        let a = left.get(index).copied().unwrap_or(0);
        let b = right.get(index).copied().unwrap_or(0);
        diff |= (a ^ b) as usize;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::{constant_time_eq, parse_usd_nano, public_registration_open};

    #[test]
    fn bootstrap_token_comparison_requires_exact_match() {
        assert!(constant_time_eq("0123456789abcdef", "0123456789abcdef"));
        assert!(!constant_time_eq("0123456789abcdef", "0123456789abcdeg"));
        assert!(!constant_time_eq("short", "shorter"));
    }

    #[test]
    fn public_signup_bonus_uses_exact_nano_usd_precision() {
        assert_eq!(parse_usd_nano("0").expect("zero"), 0);
        assert_eq!(parse_usd_nano("10").expect("ten"), 10_000_000_000);
        assert_eq!(parse_usd_nano("0.000000001").expect("nano"), 1);
        assert!(parse_usd_nano("1.0000000001").is_err());
        assert!(parse_usd_nano("-1").is_err());
    }

    #[test]
    fn public_registration_is_closed_by_default() {
        std::env::remove_var("BURNCLOUD_PUBLIC_REGISTRATION");
        assert!(!public_registration_open());
    }
}
