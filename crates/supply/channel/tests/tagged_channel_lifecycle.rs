#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "独立测试中失败即表示前提或断言不成立"
)]
use burncloud_database::{create_database_with_url, Database, DatabaseError};
use burncloud_supply_channel::{ChannelAbilityModel, ChannelService};
use burncloud_supply_contracts::{Channel, ChannelType};

fn sample_channel() -> Channel {
    Channel {
        id: 0,
        type_: ChannelType::Anthropic as i32,
        key: "pg-secret".to_string(),
        status: 1,
        name: "pg-anthropic".to_string(),
        weight: 17,
        created_time: None,
        test_time: None,
        response_time: None,
        base_url: Some("https://api.anthropic.com/v1".to_string()),
        models: "claude-pg-contract".to_string(),
        group: "pg-vip".to_string(),
        used_quota: 0,
        model_mapping: None,
        priority: 9,
        auto_ban: 1,
        other_info: None,
        tag: None,
        setting: None,
        param_override: None,
        header_override: None,
        remark: None,
        api_version: Some("2024-02-01".to_string()),
        pricing_region: Some("international".to_string()),
        rpm_cap: Some(600),
        tpm_cap: Some(200_000),
        reservation_green: Some(0.5),
        reservation_yellow: Some(0.3),
        reservation_red: Some(0.2),
    }
}

async fn fresh_db() -> (Database, std::path::PathBuf) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "bc_tagged_{}_{}_{}.db",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let db = create_database_with_url(&burncloud_database::sqlite_url(
        &path.to_string_lossy(),
        cfg!(windows),
        true,
    ))
    .await
    .unwrap();
    (db, path)
}

async fn cleanup(db: Database, path: std::path::PathBuf) {
    db.close().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn parameters_ownership_missing_and_repeat_operations() {
    let (db, path) = fresh_db().await;
    for (id, tag) in [(0, "owned"), (-1, "owned"), (1, "")] {
        assert!(matches!(
            ChannelService::quarantine_if_tag(&db, id, tag).await,
            Err(DatabaseError::InvalidData { .. })
        ));
        assert!(matches!(
            ChannelService::delete_if_tag(&db, id, tag).await,
            Err(DatabaseError::InvalidData { .. })
        ));
    }
    let mut channel = sample_channel();
    channel.tag = Some("owned".into());
    let id = ChannelService::create(&db, &mut channel).await.unwrap();
    for wrong_tag in ["other", " owned", "OWNED"] {
        assert!(matches!(
            ChannelService::quarantine_if_tag(&db, id, wrong_tag).await,
            Err(DatabaseError::InvalidData { .. })
        ));
        assert!(matches!(
            ChannelService::delete_if_tag(&db, id, wrong_tag).await,
            Err(DatabaseError::InvalidData { .. })
        ));
        assert_eq!(
            ChannelService::get_by_id(&db, id)
                .await
                .unwrap()
                .unwrap()
                .status,
            1
        );
        assert_eq!(
            ChannelAbilityModel::list_by_channel(&db, id)
                .await
                .unwrap()
                .len(),
            1
        );
    }
    assert!(ChannelService::quarantine_if_tag(&db, id, "owned")
        .await
        .unwrap());
    assert!(ChannelService::quarantine_if_tag(&db, id, "owned")
        .await
        .unwrap());
    assert_eq!(
        ChannelService::get_by_id(&db, id)
            .await
            .unwrap()
            .unwrap()
            .status,
        3
    );
    assert!(ChannelAbilityModel::list_by_channel(&db, id)
        .await
        .unwrap()
        .is_empty());
    assert!(ChannelService::delete_if_tag(&db, id, "owned")
        .await
        .unwrap());
    assert!(!ChannelService::delete_if_tag(&db, id, "owned")
        .await
        .unwrap());
    assert!(!ChannelService::quarantine_if_tag(&db, id, "owned")
        .await
        .unwrap());
    channel.id = 0;
    channel.tag = None;
    let other = ChannelService::create(&db, &mut channel).await.unwrap();
    assert!(matches!(
        ChannelService::delete_if_tag(&db, other, "owned").await,
        Err(DatabaseError::InvalidData { .. })
    ));
    assert!(matches!(
        ChannelService::quarantine_if_tag(&db, other, "owned").await,
        Err(DatabaseError::InvalidData { .. })
    ));
    cleanup(db, path).await;
}

#[tokio::test]
async fn failure_after_ability_delete_rolls_back_both_operations() {
    let (db, path) = fresh_db().await;
    let mut channel = sample_channel();
    channel.tag = Some("owned".into());
    let id = ChannelService::create(&db, &mut channel).await.unwrap();
    let pool = db.get_connection().unwrap().pool();
    for operation in ["UPDATE", "DELETE"] {
        let sql = format!("CREATE TRIGGER fail_lifecycle_{operation} BEFORE {operation} ON channel_providers BEGIN SELECT RAISE(ABORT, 'injected'); END");
        sqlx::raw_sql(&sql)
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("{operation}: {e}"));
        let result = if operation == "UPDATE" {
            ChannelService::quarantine_if_tag(&db, id, "owned").await
        } else {
            ChannelService::delete_if_tag(&db, id, "owned").await
        };
        assert!(result.is_err());
        assert_eq!(
            ChannelService::get_by_id(&db, id)
                .await
                .unwrap()
                .unwrap()
                .status,
            1
        );
        assert_eq!(
            ChannelAbilityModel::list_by_channel(&db, id)
                .await
                .unwrap()
                .len(),
            1
        );
        sqlx::raw_sql(&format!("DROP TRIGGER fail_lifecycle_{operation}"))
            .execute(pool)
            .await
            .unwrap();
    }
    cleanup(db, path).await;
}

#[tokio::test]
async fn database_errors_are_not_missing_records() {
    let (db, path) = fresh_db().await;
    sqlx::query("DROP TABLE channel_providers")
        .execute(db.get_connection().unwrap().pool())
        .await
        .unwrap();
    assert!(ChannelService::quarantine_if_tag(&db, 1, "owned")
        .await
        .is_err());
    assert!(ChannelService::delete_if_tag(&db, 1, "owned")
        .await
        .is_err());
    cleanup(db, path).await;
}

#[tokio::test]
async fn changed_ownership_inside_transaction_cannot_commit_partial_ability_deletion() {
    let (db, path) = fresh_db().await;
    let mut channel = sample_channel();
    channel.tag = Some("owned".into());
    let id = ChannelService::create(&db, &mut channel).await.unwrap();
    let pool = db.get_connection().unwrap().pool();
    // 在归属检查后故意改标签，使最终条件修改影响零行；整个事务仍必须回滚。
    sqlx::raw_sql("CREATE TRIGGER change_owner AFTER DELETE ON channel_abilities BEGIN UPDATE channel_providers SET tag = 'other' WHERE id = OLD.channel_id; END").execute(pool).await.unwrap();
    for delete in [false, true] {
        let result = if delete {
            ChannelService::delete_if_tag(&db, id, "owned").await
        } else {
            ChannelService::quarantine_if_tag(&db, id, "owned").await
        };
        assert!(matches!(result, Err(DatabaseError::InvalidData { .. })));
        let stored = ChannelService::get_by_id(&db, id).await.unwrap().unwrap();
        assert_eq!(stored.tag.as_deref(), Some("owned"));
        assert_eq!(stored.status, 1);
        assert_eq!(
            ChannelAbilityModel::list_by_channel(&db, id)
                .await
                .unwrap()
                .len(),
            1
        );
    }
    cleanup(db, path).await;
}
