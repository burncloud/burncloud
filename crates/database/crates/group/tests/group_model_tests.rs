/// Internal tests for `database-group` — RouterGroupModel, RouterGroupMemberModel, RouterGroupRepository.
///
/// All tests run against an isolated SQLite temp-file database so they do not
/// touch the user's default database and can run in any CI environment.
use burncloud_common::CrudRepository;
use burncloud_database::create_database_with_url;
use burncloud_database_group::{
    RouterGroup, RouterGroupMember, RouterGroupMemberModel, RouterGroupModel, RouterGroupRepository,
};
use tempfile::NamedTempFile;

/// Create an isolated SQLite database with the `router_groups` and
/// `router_group_members` tables pre-created.
async fn create_test_db() -> (burncloud_database::Database, NamedTempFile) {
    let tmp = NamedTempFile::new().expect("failed to create temp file");
    let url = format!("sqlite://{}?mode=rwc", tmp.path().display());
    let db = create_database_with_url(&url)
        .await
        .expect("failed to connect to test database");

    db.execute_query(
        r#"
        CREATE TABLE IF NOT EXISTS router_groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            strategy TEXT NOT NULL DEFAULT 'round_robin',
            match_path TEXT NOT NULL
        )
        "#,
    )
    .await
    .expect("failed to create router_groups table");

    db.execute_query(
        r#"
        CREATE TABLE IF NOT EXISTS router_group_members (
            group_id TEXT NOT NULL,
            upstream_id TEXT NOT NULL,
            weight INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY (group_id, upstream_id)
        )
        "#,
    )
    .await
    .expect("failed to create router_group_members table");

    (db, tmp)
}

fn make_group(id: &str) -> RouterGroup {
    RouterGroup {
        id: id.to_string(),
        name: format!("Group {}", id),
        strategy: "round_robin".to_string(),
        match_path: "/v1/*".to_string(),
    }
}

// ---------------------------------------------------------------------------
// RouterGroupModel tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_group_model_create_and_get() {
    let (db, _tmp) = create_test_db().await;
    let g = make_group("group-1");

    RouterGroupModel::create(&db, &g)
        .await
        .expect("create failed");

    let found = RouterGroupModel::get(&db, "group-1")
        .await
        .expect("get failed")
        .expect("group not found");

    assert_eq!(found.id, g.id);
    assert_eq!(found.name, g.name);
    assert_eq!(found.strategy, g.strategy);
    assert_eq!(found.match_path, g.match_path);
}

#[tokio::test]
async fn test_group_model_get_missing_returns_none() {
    let (db, _tmp) = create_test_db().await;

    let result = RouterGroupModel::get(&db, "does-not-exist")
        .await
        .expect("get failed");

    assert!(result.is_none());
}

#[tokio::test]
async fn test_group_model_get_all_empty() {
    let (db, _tmp) = create_test_db().await;

    let all = RouterGroupModel::get_all(&db)
        .await
        .expect("get_all failed");
    assert!(all.is_empty());
}

#[tokio::test]
async fn test_group_model_get_all_multiple() {
    let (db, _tmp) = create_test_db().await;

    RouterGroupModel::create(&db, &make_group("g-a"))
        .await
        .unwrap();
    RouterGroupModel::create(&db, &make_group("g-b"))
        .await
        .unwrap();
    RouterGroupModel::create(&db, &make_group("g-c"))
        .await
        .unwrap();

    let all = RouterGroupModel::get_all(&db)
        .await
        .expect("get_all failed");
    assert_eq!(all.len(), 3);

    let ids: Vec<&str> = all.iter().map(|g| g.id.as_str()).collect();
    assert!(ids.contains(&"g-a"));
    assert!(ids.contains(&"g-b"));
    assert!(ids.contains(&"g-c"));
}

#[tokio::test]
async fn test_group_model_delete_removes_group() {
    let (db, _tmp) = create_test_db().await;
    let g = make_group("del-group");

    RouterGroupModel::create(&db, &g).await.unwrap();
    assert!(RouterGroupModel::get(&db, "del-group")
        .await
        .unwrap()
        .is_some());

    RouterGroupModel::delete(&db, "del-group")
        .await
        .expect("delete failed");
    assert!(RouterGroupModel::get(&db, "del-group")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_group_model_delete_also_removes_members() {
    let (db, _tmp) = create_test_db().await;

    let g = make_group("parent-group");
    RouterGroupModel::create(&db, &g).await.unwrap();

    let members = vec![
        RouterGroupMember {
            group_id: "parent-group".to_string(),
            upstream_id: "up-1".to_string(),
            weight: 1,
        },
        RouterGroupMember {
            group_id: "parent-group".to_string(),
            upstream_id: "up-2".to_string(),
            weight: 2,
        },
    ];
    RouterGroupMemberModel::set_for_group(&db, "parent-group", members)
        .await
        .unwrap();

    // Verify members exist
    let before = RouterGroupMemberModel::get_by_group(&db, "parent-group")
        .await
        .unwrap();
    assert_eq!(before.len(), 2);

    // Delete the group — members should be cascaded
    RouterGroupModel::delete(&db, "parent-group").await.unwrap();
    let after = RouterGroupMemberModel::get_by_group(&db, "parent-group")
        .await
        .unwrap();
    assert!(after.is_empty());
}

// ---------------------------------------------------------------------------
// RouterGroupMemberModel tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_member_model_set_and_get_by_group() {
    let (db, _tmp) = create_test_db().await;

    RouterGroupModel::create(&db, &make_group("grp"))
        .await
        .unwrap();

    let members = vec![
        RouterGroupMember {
            group_id: "grp".to_string(),
            upstream_id: "upstream-a".to_string(),
            weight: 10,
        },
        RouterGroupMember {
            group_id: "grp".to_string(),
            upstream_id: "upstream-b".to_string(),
            weight: 20,
        },
    ];

    RouterGroupMemberModel::set_for_group(&db, "grp", members)
        .await
        .expect("set_for_group failed");

    let rows = RouterGroupMemberModel::get_by_group(&db, "grp")
        .await
        .expect("get_by_group failed");

    assert_eq!(rows.len(), 2);
    let upstream_ids: Vec<&str> = rows.iter().map(|m| m.upstream_id.as_str()).collect();
    assert!(upstream_ids.contains(&"upstream-a"));
    assert!(upstream_ids.contains(&"upstream-b"));
}

#[tokio::test]
async fn test_member_model_set_for_group_replaces_existing() {
    let (db, _tmp) = create_test_db().await;

    RouterGroupModel::create(&db, &make_group("replace-grp"))
        .await
        .unwrap();

    let initial = vec![RouterGroupMember {
        group_id: "replace-grp".to_string(),
        upstream_id: "old-upstream".to_string(),
        weight: 1,
    }];
    RouterGroupMemberModel::set_for_group(&db, "replace-grp", initial)
        .await
        .unwrap();

    let replacement = vec![RouterGroupMember {
        group_id: "replace-grp".to_string(),
        upstream_id: "new-upstream".to_string(),
        weight: 5,
    }];
    RouterGroupMemberModel::set_for_group(&db, "replace-grp", replacement)
        .await
        .unwrap();

    let rows = RouterGroupMemberModel::get_by_group(&db, "replace-grp")
        .await
        .unwrap();

    assert_eq!(rows.len(), 1, "old members should be replaced");
    assert_eq!(rows[0].upstream_id, "new-upstream");
    assert_eq!(rows[0].weight, 5);
}

#[tokio::test]
async fn test_member_model_get_all() {
    let (db, _tmp) = create_test_db().await;

    RouterGroupModel::create(&db, &make_group("g1"))
        .await
        .unwrap();
    RouterGroupModel::create(&db, &make_group("g2"))
        .await
        .unwrap();

    let m1 = vec![RouterGroupMember {
        group_id: "g1".to_string(),
        upstream_id: "up-x".to_string(),
        weight: 1,
    }];
    let m2 = vec![RouterGroupMember {
        group_id: "g2".to_string(),
        upstream_id: "up-y".to_string(),
        weight: 2,
    }];

    RouterGroupMemberModel::set_for_group(&db, "g1", m1)
        .await
        .unwrap();
    RouterGroupMemberModel::set_for_group(&db, "g2", m2)
        .await
        .unwrap();

    let all = RouterGroupMemberModel::get_all(&db)
        .await
        .expect("get_all failed");
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_member_model_set_empty_clears_members() {
    let (db, _tmp) = create_test_db().await;

    RouterGroupModel::create(&db, &make_group("clear-grp"))
        .await
        .unwrap();

    let members = vec![RouterGroupMember {
        group_id: "clear-grp".to_string(),
        upstream_id: "up-1".to_string(),
        weight: 1,
    }];
    RouterGroupMemberModel::set_for_group(&db, "clear-grp", members)
        .await
        .unwrap();

    // Clear by setting empty list
    RouterGroupMemberModel::set_for_group(&db, "clear-grp", vec![])
        .await
        .unwrap();

    let rows = RouterGroupMemberModel::get_by_group(&db, "clear-grp")
        .await
        .unwrap();
    assert!(rows.is_empty());
}

// ---------------------------------------------------------------------------
// RouterGroupRepository (CrudRepository) tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_repository_create_and_find() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    let g = make_group("repo-1");
    let created = repo.create(&g).await.expect("create failed");

    assert_eq!(created.id, "repo-1");
    assert_eq!(created.name, g.name);

    let found = repo
        .find_by_id(&"repo-1".to_string())
        .await
        .expect("find_by_id failed")
        .expect("should be Some");

    assert_eq!(found.id, "repo-1");
}

#[tokio::test]
async fn test_repository_find_missing_returns_none() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    let result = repo
        .find_by_id(&"no-such-group".to_string())
        .await
        .expect("find_by_id failed");

    assert!(result.is_none());
}

#[tokio::test]
async fn test_repository_list() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    repo.create(&make_group("list-a")).await.unwrap();
    repo.create(&make_group("list-b")).await.unwrap();

    let all = repo.list().await.expect("list failed");
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_repository_update() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    repo.create(&make_group("upd-1")).await.unwrap();

    let updated = RouterGroup {
        id: "upd-1".to_string(),
        name: "Updated Name".to_string(),
        strategy: "weighted".to_string(),
        match_path: "/api/*".to_string(),
    };
    let found = repo
        .update(&"upd-1".to_string(), &updated)
        .await
        .expect("update failed");
    assert!(found, "update should return true for existing group");

    let after = repo
        .find_by_id(&"upd-1".to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.name, "Updated Name");
    assert_eq!(after.strategy, "weighted");
}

#[tokio::test]
async fn test_repository_update_nonexistent_returns_false() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    let g = make_group("ghost");
    let result = repo
        .update(&"ghost".to_string(), &g)
        .await
        .expect("update failed");
    assert!(!result, "update on non-existent group should return false");
}

#[tokio::test]
async fn test_repository_delete() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    repo.create(&make_group("del-1")).await.unwrap();

    let deleted = repo
        .delete(&"del-1".to_string())
        .await
        .expect("delete failed");
    assert!(deleted, "delete should return true for existing group");

    let after = repo.find_by_id(&"del-1".to_string()).await.unwrap();
    assert!(after.is_none());
}

#[tokio::test]
async fn test_repository_delete_nonexistent_returns_false() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    let result = repo
        .delete(&"phantom".to_string())
        .await
        .expect("delete failed");
    assert!(!result, "delete on non-existent group should return false");
}
