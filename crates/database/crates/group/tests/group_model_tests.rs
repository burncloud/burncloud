#![allow(clippy::unwrap_used, clippy::expect_used, clippy::unnecessary_unwrap)]
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
    let tmp = NamedTempFile::new().unwrap_or_else(|e| panic!("failed to create temp file: {e}"));
    let url = format!("sqlite://{}?mode=rwc", tmp.path().display());
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("failed to connect to test database: {e}"));

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
    .unwrap_or_else(|e| panic!("failed to create router_groups table: {e}"));

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
    .unwrap_or_else(|e| panic!("failed to create router_group_members table: {e}"));

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
        .unwrap_or_else(|e| panic!("create failed: {e}"));

    let found = RouterGroupModel::get(&db, "group-1")
        .await
        .unwrap_or_else(|e| panic!("get failed: {e}"))
        .unwrap_or_else(|| panic!("group not found"));

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
        .unwrap_or_else(|e| panic!("get failed: {e}"));

    assert!(result.is_none());
}

#[tokio::test]
async fn test_group_model_get_all_empty() {
    let (db, _tmp) = create_test_db().await;

    let all = RouterGroupModel::get_all(&db)
        .await
        .unwrap_or_else(|e| panic!("get_all failed: {e}"));
    assert!(all.is_empty());
}

#[tokio::test]
async fn test_group_model_get_all_multiple() {
    let (db, _tmp) = create_test_db().await;

    RouterGroupModel::create(&db, &make_group("g-a"))
        .await
        .unwrap_or_else(|e| panic!("create g-a failed: {e}"));
    RouterGroupModel::create(&db, &make_group("g-b"))
        .await
        .unwrap_or_else(|e| panic!("create g-b failed: {e}"));
    RouterGroupModel::create(&db, &make_group("g-c"))
        .await
        .unwrap_or_else(|e| panic!("create g-c failed: {e}"));

    let all = RouterGroupModel::get_all(&db)
        .await
        .unwrap_or_else(|e| panic!("get_all failed: {e}"));
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

    RouterGroupModel::create(&db, &g)
        .await
        .unwrap_or_else(|e| panic!("create failed: {e}"));
    assert!(RouterGroupModel::get(&db, "del-group")
        .await
        .unwrap_or_else(|e| panic!("get failed: {e}"))
        .is_some());

    RouterGroupModel::delete(&db, "del-group")
        .await
        .unwrap_or_else(|e| panic!("delete failed: {e}"));
    assert!(RouterGroupModel::get(&db, "del-group")
        .await
        .unwrap_or_else(|e| panic!("get failed: {e}"))
        .is_none());
}

#[tokio::test]
async fn test_group_model_delete_also_removes_members() {
    let (db, _tmp) = create_test_db().await;

    let g = make_group("parent-group");
    RouterGroupModel::create(&db, &g)
        .await
        .unwrap_or_else(|e| panic!("create failed: {e}"));

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
        .unwrap_or_else(|e| panic!("set_for_group failed: {e}"));

    // Verify members exist
    let before = RouterGroupMemberModel::get_by_group(&db, "parent-group")
        .await
        .unwrap_or_else(|e| panic!("get_by_group failed: {e}"));
    assert_eq!(before.len(), 2);

    // Delete the group — members should be cascaded
    RouterGroupModel::delete(&db, "parent-group")
        .await
        .unwrap_or_else(|e| panic!("delete failed: {e}"));
    let after = RouterGroupMemberModel::get_by_group(&db, "parent-group")
        .await
        .unwrap_or_else(|e| panic!("get_by_group failed: {e}"));
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
        .unwrap_or_else(|e| panic!("create failed: {e}"));

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
        .unwrap_or_else(|e| panic!("set_for_group failed: {e}"));

    let rows = RouterGroupMemberModel::get_by_group(&db, "grp")
        .await
        .unwrap_or_else(|e| panic!("get_by_group failed: {e}"));

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
        .unwrap_or_else(|e| panic!("create failed: {e}"));

    let initial = vec![RouterGroupMember {
        group_id: "replace-grp".to_string(),
        upstream_id: "old-upstream".to_string(),
        weight: 1,
    }];
    RouterGroupMemberModel::set_for_group(&db, "replace-grp", initial)
        .await
        .unwrap_or_else(|e| panic!("set_for_group initial failed: {e}"));

    let replacement = vec![RouterGroupMember {
        group_id: "replace-grp".to_string(),
        upstream_id: "new-upstream".to_string(),
        weight: 5,
    }];
    RouterGroupMemberModel::set_for_group(&db, "replace-grp", replacement)
        .await
        .unwrap_or_else(|e| panic!("set_for_group replacement failed: {e}"));

    let rows = RouterGroupMemberModel::get_by_group(&db, "replace-grp")
        .await
        .unwrap_or_else(|e| panic!("get_by_group failed: {e}"));

    assert_eq!(rows.len(), 1, "old members should be replaced");
    assert_eq!(rows[0].upstream_id, "new-upstream");
    assert_eq!(rows[0].weight, 5);
}

#[tokio::test]
async fn test_member_model_get_all() {
    let (db, _tmp) = create_test_db().await;

    RouterGroupModel::create(&db, &make_group("g1"))
        .await
        .unwrap_or_else(|e| panic!("create g1 failed: {e}"));
    RouterGroupModel::create(&db, &make_group("g2"))
        .await
        .unwrap_or_else(|e| panic!("create g2 failed: {e}"));

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
        .unwrap_or_else(|e| panic!("set_for_group g1 failed: {e}"));
    RouterGroupMemberModel::set_for_group(&db, "g2", m2)
        .await
        .unwrap_or_else(|e| panic!("set_for_group g2 failed: {e}"));

    let all = RouterGroupMemberModel::get_all(&db)
        .await
        .unwrap_or_else(|e| panic!("get_all failed: {e}"));
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_member_model_set_empty_clears_members() {
    let (db, _tmp) = create_test_db().await;

    RouterGroupModel::create(&db, &make_group("clear-grp"))
        .await
        .unwrap_or_else(|e| panic!("create failed: {e}"));

    let members = vec![RouterGroupMember {
        group_id: "clear-grp".to_string(),
        upstream_id: "up-1".to_string(),
        weight: 1,
    }];
    RouterGroupMemberModel::set_for_group(&db, "clear-grp", members)
        .await
        .unwrap_or_else(|e| panic!("set_for_group failed: {e}"));

    // Clear by setting empty list
    RouterGroupMemberModel::set_for_group(&db, "clear-grp", vec![])
        .await
        .unwrap_or_else(|e| panic!("set_for_group clear failed: {e}"));

    let rows = RouterGroupMemberModel::get_by_group(&db, "clear-grp")
        .await
        .unwrap_or_else(|e| panic!("get_by_group failed: {e}"));
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
    let created = repo
        .create(&g)
        .await
        .unwrap_or_else(|e| panic!("create failed: {e}"));

    assert_eq!(created.id, "repo-1");
    assert_eq!(created.name, g.name);

    let found = repo
        .find_by_id(&"repo-1".to_string())
        .await
        .unwrap_or_else(|e| panic!("find_by_id failed: {e}"))
        .unwrap_or_else(|| panic!("should be Some"));

    assert_eq!(found.id, "repo-1");
}

#[tokio::test]
async fn test_repository_find_missing_returns_none() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    let result = repo
        .find_by_id(&"no-such-group".to_string())
        .await
        .unwrap_or_else(|e| panic!("find_by_id failed: {e}"));

    assert!(result.is_none());
}

#[tokio::test]
async fn test_repository_list() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    repo.create(&make_group("list-a"))
        .await
        .unwrap_or_else(|e| panic!("create list-a failed: {e}"));
    repo.create(&make_group("list-b"))
        .await
        .unwrap_or_else(|e| panic!("create list-b failed: {e}"));

    let all = repo
        .list()
        .await
        .unwrap_or_else(|e| panic!("list failed: {e}"));
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_repository_update() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    repo.create(&make_group("upd-1"))
        .await
        .unwrap_or_else(|e| panic!("create failed: {e}"));

    let updated = RouterGroup {
        id: "upd-1".to_string(),
        name: "Updated Name".to_string(),
        strategy: "weighted".to_string(),
        match_path: "/api/*".to_string(),
    };
    let found = repo
        .update(&"upd-1".to_string(), &updated)
        .await
        .unwrap_or_else(|e| panic!("update failed: {e}"));
    assert!(found, "update should return true for existing group");

    let after = repo
        .find_by_id(&"upd-1".to_string())
        .await
        .unwrap_or_else(|e| panic!("find_by_id failed: {e}"))
        .unwrap_or_else(|| panic!("upd-1 should exist after update"));
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
        .unwrap_or_else(|e| panic!("update failed: {e}"));
    assert!(!result, "update on non-existent group should return false");
}

#[tokio::test]
async fn test_repository_delete() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    repo.create(&make_group("del-1"))
        .await
        .unwrap_or_else(|e| panic!("create failed: {e}"));

    let deleted = repo
        .delete(&"del-1".to_string())
        .await
        .unwrap_or_else(|e| panic!("delete failed: {e}"));
    assert!(deleted, "delete should return true for existing group");

    let after = repo
        .find_by_id(&"del-1".to_string())
        .await
        .unwrap_or_else(|e| panic!("find_by_id failed: {e}"));
    assert!(after.is_none());
}

#[tokio::test]
async fn test_repository_delete_nonexistent_returns_false() {
    let (db, _tmp) = create_test_db().await;
    let repo = RouterGroupRepository(&db);

    let result = repo
        .delete(&"phantom".to_string())
        .await
        .unwrap_or_else(|e| panic!("delete failed: {e}"));
    assert!(!result, "delete on non-existent group should return false");
}
