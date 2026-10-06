//! # BurnCloud Supply Channel
//!
//! Supply-owned implementation for upstream Channel management. This crate is the
//! vertical owner for Channel use cases and persistence; it replaces the historical
//! `database-channel` + `service-channel` technical split without changing behavior.
//!
//! ## Domain-crate contract (the 10 questions)
//!
//! 1. **Owner** — Supply. Business capability: Channel / upstream provider management.
//! 2. **Responsibility** — create, read, update, delete and synchronize Channel providers,
//!    abilities and protocol configurations. It must not own Traffic scheduling, Billing,
//!    Auth/Token, HTTP presentation or Node lifecycle behavior.
//! 3. **Data Truth** — `channel_providers`, `channel_abilities` and
//!    `channel_protocol_configs`. Schema and historical migrations are not owned by this
//!    migration and must not be rewritten here.
//! 4. **Public contract** — cross-domain value objects live in
//!    `burncloud-supply-contracts`; [`ChannelService`] is the preferred Channel use-case
//!    facade. `ChannelProviderModel`, `ChannelAbilityModel` and
//!    `ChannelProtocolConfigModel` remain public only as compatibility surfaces for
//!    existing consumers and must not be treated as the preferred boundary for new code.
//! 5. **Allowed dependencies** — Supply contracts, platform database infrastructure and
//!    implementation libraries (`sqlx`, `tokio`, serde/tracing). This crate must not depend
//!    on Commerce, Traffic, Identity or interface implementation crates.
//! 6. **Allowed consumers** — interfaces may call [`ChannelService`]; other domains should
//!    depend on `burncloud-supply-contracts` for values and require architecture review
//!    before depending on this implementation crate. Existing Router/Inference consumers
//!    are migration debt, not precedent.
//! 7. **Internal structure** — SQL and row mapping stay inside this crate; `rows` and
//!    helper modules remain private. Do not create empty service/repository/factory layers
//!    merely to imitate a diagram.
//! 8. **Errors / logs / security** — database failures propagate as `DatabaseError`;
//!    callers decide presentation. SQL values are bound rather than interpolated. Channel
//!    credentials are data, not log fields: do not log API keys or secrets.
//! 9. **Proof** — SQLite integration suites pin row mapping, CRUD, ability synchronization
//!    and protocol config behavior; the PostgreSQL dialect contract exercises backend-only
//!    SQL; repository acceptance is `cargo run -- code test --all`. The unit migration
//!    invariants below additionally prove the merged [`ChannelService`] preserves automatic
//!    ability synchronization without tests manually repairing it.
//! 10. **Correct / incorrect calls** — correct cross-domain type usage goes through
//!     `burncloud-supply-contracts`; Channel use cases go through [`ChannelService`]. New
//!     Traffic/Commerce/Identity code must not issue SQL against Channel tables or add a
//!     fresh dependency on `Channel*Model` just because those compatibility symbols remain
//!     public during migration.
//!
//! ```text
//! Correct:   Interface -> ChannelService -> Supply-owned persistence -> platform Database
//! Correct:   Other domain -> burncloud-supply-contracts (when only Channel values are needed)
//! Incorrect: Traffic -> SELECT/UPDATE channel_providers or channel_abilities directly
//! Incorrect: New domain code -> ChannelProviderModel solely to bypass the use-case boundary
//! ```

mod channel_ability;
mod channel_protocol_config;
#[expect(
    clippy::cognitive_complexity,
    reason = "channel ability synchronization intentionally reconciles create/update/delete cases in one legacy routine; contract tests cover its behavior while decomposition remains maintainability work"
)]
mod channel_provider;
mod common;
mod rows;

use burncloud_database::Database;

pub use burncloud_database::DatabaseError;
pub use burncloud_supply_contracts::{Ability, Channel, ChannelAbility, ChannelProvider};
pub use channel_ability::{ChannelAbilityInput, ChannelAbilityModel};
pub use channel_protocol_config::{
    ChannelProtocolConfig, ChannelProtocolConfigInput, ChannelProtocolConfigModel,
};
pub use channel_provider::ChannelProviderModel;

type Result<T> = std::result::Result<T, DatabaseError>;

/// Supply-owned facade for upstream Channel use cases.
///
/// These methods intentionally preserve the former `burncloud-service-channel` behavior:
/// they delegate to the same persistence implementation without changing SQL, error, or
/// synchronization semantics.
pub struct ChannelService;

impl ChannelService {
    /// List channels with pagination.
    pub async fn list(db: &Database, limit: i32, offset: i32) -> Result<Vec<Channel>> {
        ChannelProviderModel::list(db, limit, offset).await
    }

    /// Create a new channel. Sets `channel.id` to the newly assigned ID.
    pub async fn create(db: &Database, channel: &mut Channel) -> Result<i32> {
        ChannelProviderModel::create(db, channel).await
    }

    /// Update an existing channel.
    pub async fn update(db: &Database, channel: &Channel) -> Result<()> {
        ChannelProviderModel::update(db, channel).await
    }

    /// Delete a channel by ID.
    pub async fn delete(db: &Database, id: i32) -> Result<()> {
        ChannelProviderModel::delete(db, id).await
    }

    /// Get a channel by ID.
    pub async fn get_by_id(db: &Database, id: i32) -> Result<Option<Channel>> {
        ChannelProviderModel::get_by_id(db, id).await
    }

    /// Synchronize model abilities for a channel.
    pub async fn sync_abilities(db: &Database, channel: &Channel) -> Result<()> {
        ChannelProviderModel::sync_abilities(db, channel).await
    }
}

#[cfg(test)]
mod migration_invariants {
    use super::{ChannelAbilityModel, ChannelService};
    use burncloud_database::{create_database_with_url, Database};
    use burncloud_supply_contracts::{Channel, ChannelType};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    type TestResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    static NEXT_DB: AtomicU64 = AtomicU64::new(0);

    fn sample_channel(name: &str, models: &str, group: &str, status: i32) -> Channel {
        Channel {
            id: 0,
            type_: ChannelType::OpenAI as i32,
            key: "sk-migration-invariant".to_owned(),
            status,
            name: name.to_owned(),
            weight: 10,
            created_time: None,
            test_time: None,
            response_time: None,
            base_url: Some("https://upstream.invalid".to_owned()),
            models: models.to_owned(),
            group: group.to_owned(),
            used_quota: 0,
            model_mapping: None,
            priority: 7,
            auto_ban: 1,
            other_info: None,
            tag: None,
            setting: None,
            param_override: None,
            header_override: None,
            remark: None,
            api_version: Some("default".to_owned()),
            pricing_region: None,
            rpm_cap: None,
            tpm_cap: None,
            reservation_green: None,
            reservation_yellow: None,
            reservation_red: None,
        }
    }

    async fn fresh_db(tag: &str) -> TestResult<(Database, PathBuf)> {
        let serial = NEXT_DB.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!(
            "bc_supply_channel_{tag}_{}_{}_{}.db",
            std::process::id(),
            serial,
            nanos
        ));
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Box::new(error)),
        }
        let normalized = path.to_string_lossy().replace('\\', "/");
        let url = format!("sqlite:///{}?mode=rwc", normalized);
        let db = create_database_with_url(&url).await?;
        Ok((db, path))
    }

    async fn cleanup(db: Database, path: &Path) -> TestResult<()> {
        db.close().await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        for suffix in ["", "-wal", "-shm"] {
            let mut candidate = path.as_os_str().to_os_string();
            candidate.push(suffix);
            let candidate = PathBuf::from(candidate);
            match std::fs::remove_file(&candidate) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(Box::new(error)),
            }
        }
        Ok(())
    }

    /// The service facade must preserve the production invariant that create/update themselves
    /// synchronize abilities. The test deliberately never calls `sync_abilities` directly.
    #[tokio::test]
    async fn service_lifecycle_keeps_abilities_synchronized_without_manual_sync() -> TestResult<()>
    {
        let (db, path) = fresh_db("service_lifecycle").await?;
        let mut channel = sample_channel("service", "m1,m2", "default", 1);

        let id = ChannelService::create(&db, &mut channel).await?;
        assert!(id > 0);
        assert_eq!(channel.id, id);

        let created_abilities = ChannelAbilityModel::list_by_channel(&db, id).await?;
        assert_eq!(
            created_abilities.len(),
            2,
            "create must synchronize abilities itself"
        );

        channel.models = "m2,m3".to_owned();
        channel.group = "premium".to_owned();
        channel.status = 0;
        ChannelService::update(&db, &channel).await?;
        let disabled_abilities = ChannelAbilityModel::list_by_channel(&db, id).await?;
        assert!(
            disabled_abilities.is_empty(),
            "disabling through update must remove abilities without a second sync call"
        );

        channel.status = 1;
        ChannelService::update(&db, &channel).await?;
        let reenabled = ChannelAbilityModel::list_by_channel(&db, id).await?;
        assert_eq!(reenabled.len(), 2);
        let all_premium = reenabled.iter().all(|ability| ability.group == "premium");
        assert!(all_premium);
        let mut models: Vec<_> = reenabled
            .iter()
            .map(|ability| ability.model.as_str())
            .collect();
        models.sort_unstable();
        assert_eq!(models, vec!["m2", "m3"]);

        let fetched = ChannelService::get_by_id(&db, id).await?;
        assert_eq!(
            fetched.as_ref().map(|item| item.name.as_str()),
            Some("service")
        );
        let listed = ChannelService::list(&db, 100, 0).await?;
        assert!(listed.iter().any(|item| item.id == id));

        ChannelService::delete(&db, id).await?;
        assert!(ChannelService::get_by_id(&db, id).await?.is_none());
        let remaining_abilities = ChannelAbilityModel::list_by_channel(&db, id).await?;
        assert!(remaining_abilities.is_empty());

        cleanup(db, &path).await
    }

    /// This migration must not silently change the legacy duplicate-model behavior. Today the
    /// plain INSERT used by automatic synchronization rejects a repeated model with a uniqueness
    /// error. A future behavior fix may deliberately change this test in its own issue.
    #[tokio::test]
    async fn duplicate_model_behavior_remains_an_error() -> TestResult<()> {
        let (db, path) = fresh_db("duplicate_model").await?;
        let mut channel = sample_channel("duplicate", "same,same", "default", 1);

        let result = ChannelService::create(&db, &mut channel).await;
        assert!(
            result.is_err(),
            "#744 is a structural migration and must not change the existing duplicate-model failure behavior"
        );

        cleanup(db, &path).await
    }
}
