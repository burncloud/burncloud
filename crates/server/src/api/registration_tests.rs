use super::registration::{
    initialize_bootstrap_state, register_public_user, PublicRegistrationError,
    PublicRegistrationInput,
};
use burncloud_database::Database;
use burncloud_database_user::UserDatabase;
use sqlx::Any;

fn input(username: &str, bootstrap_token: Option<&str>) -> PublicRegistrationInput {
    PublicRegistrationInput {
        username: username.to_string(),
        password: "SecureBootstrap123!".to_string(),
        email: Some(format!("{username}@example.invalid")),
        bootstrap_token: bootstrap_token.map(str::to_string),
    }
}

async fn fresh_db(path: &std::path::Path) -> Database {
    let url = format!(
        "sqlite://{}?mode=rwc",
        path.to_string_lossy().replace('\\', "/")
    );
    std::env::set_var("BURNCLOUD_DATABASE_URL", &url);
    let db = Database::new().await.expect("fresh sqlite database");
    UserDatabase::init(&db).await.expect("user schema/roles");
    let completed = initialize_bootstrap_state(&db)
        .await
        .expect("bootstrap state initialization");
    assert!(!completed, "fresh database must require first-admin setup");
    db
}

#[tokio::test]
async fn first_admin_is_zero_config_locally_and_code_guarded_remotely() {
    let temp = tempfile::tempdir().expect("temporary database directory");
    std::env::set_var("BURNCLOUD_PUBLIC_REGISTRATION", "open");
    std::env::set_var("BURNCLOUD_PUBLIC_SIGNUP_BONUS_USD", "1.25");

    // Remote/exposed server policy: BurnCloud requires its generated setup code.
    let remote_db = fresh_db(&temp.path().join("remote-bootstrap.db")).await;
    let setup_code = "0123456789abcdef-remote-setup-code";

    let missing = register_public_user(
        &remote_db,
        &input("missing-code", None),
        true,
        Some(setup_code),
    )
    .await;
    assert_eq!(missing, Err(PublicRegistrationError::BootstrapRequired));

    let wrong = register_public_user(
        &remote_db,
        &input("wrong-code", Some("wrong-wrong-wrong-wrong")),
        true,
        Some(setup_code),
    )
    .await;
    assert_eq!(wrong, Err(PublicRegistrationError::InvalidBootstrapToken));

    let remote_admin = register_public_user(
        &remote_db,
        &input("remote-admin", Some(setup_code)),
        true,
        Some(setup_code),
    )
    .await
    .expect("correct remote setup code must create admin");
    assert_eq!(remote_admin.role, "admin");

    let replay = register_public_user(
        &remote_db,
        &input("remote-replay", Some(setup_code)),
        true,
        Some(setup_code),
    )
    .await;
    assert_eq!(
        replay,
        Err(PublicRegistrationError::BootstrapAlreadyComplete)
    );

    // Default BurnCloud policy: HOST=127.0.0.1, so first-run setup requires
    // only username/password. No env bootstrap secret and no setup-code field.
    let local_db = fresh_db(&temp.path().join("local-bootstrap.db")).await;
    let local_admin = register_public_user(&local_db, &input("local-admin", None), false, None)
        .await
        .expect("local first-run setup must be zero configuration");
    assert_eq!(local_admin.role, "admin");

    let connection = local_db.get_connection().expect("database connection");
    let admin_balance: i64 = sqlx::query_scalar::<Any, i64>(
        "SELECT balance_usd FROM user_accounts WHERE id = ?",
    )
    .bind(&local_admin.user_id)
    .fetch_one(connection.pool())
    .await
    .expect("admin balance");
    assert_eq!(
        admin_balance, 0,
        "bootstrap admin must not receive public signup credit"
    );

    let marker_count: i64 = sqlx::query_scalar::<Any, i64>(
        "SELECT COUNT(*) FROM burncloud_bootstrap_state WHERE id = 1",
    )
    .fetch_one(connection.pool())
    .await
    .expect("bootstrap marker");
    assert_eq!(marker_count, 1);

    let ordinary = register_public_user(&local_db, &input("ordinary-user", None), false, None)
        .await
        .expect("open post-bootstrap public registration");
    assert_eq!(ordinary.role, "user");

    let ordinary_balance: i64 = sqlx::query_scalar::<Any, i64>(
        "SELECT balance_usd FROM user_accounts WHERE id = ?",
    )
    .bind(&ordinary.user_id)
    .fetch_one(connection.pool())
    .await
    .expect("ordinary user balance");
    assert_eq!(ordinary_balance, 1_250_000_000);

    let admin_roles: i64 = sqlx::query_scalar::<Any, i64>(
        "SELECT COUNT(*) FROM user_role_bindings b JOIN user_roles r ON r.id = b.role_id WHERE b.user_id = ? AND r.name = 'admin'",
    )
    .bind(&local_admin.user_id)
    .fetch_one(connection.pool())
    .await
    .expect("admin role binding");
    assert_eq!(admin_roles, 1);

    let ordinary_admin_roles: i64 = sqlx::query_scalar::<Any, i64>(
        "SELECT COUNT(*) FROM user_role_bindings b JOIN user_roles r ON r.id = b.role_id WHERE b.user_id = ? AND r.name = 'admin'",
    )
    .bind(&ordinary.user_id)
    .fetch_one(connection.pool())
    .await
    .expect("ordinary role binding");
    assert_eq!(ordinary_admin_roles, 0);

    std::env::remove_var("BURNCLOUD_DATABASE_URL");
    std::env::remove_var("BURNCLOUD_PUBLIC_REGISTRATION");
    std::env::remove_var("BURNCLOUD_PUBLIC_SIGNUP_BONUS_USD");
}
