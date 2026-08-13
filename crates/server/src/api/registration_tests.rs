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

#[tokio::test]
async fn fresh_install_requires_one_time_bootstrap_then_only_creates_users() {
    let temp = tempfile::tempdir().expect("temporary database directory");
    let path = temp.path().join("bootstrap.db");
    let url = format!("sqlite://{}?mode=rwc", path.to_string_lossy().replace('\\', "/"));

    std::env::set_var("BURNCLOUD_DATABASE_URL", &url);
    std::env::set_var(
        "BURNCLOUD_BOOTSTRAP_TOKEN",
        "0123456789abcdef-bootstrap-test-token",
    );
    std::env::set_var("BURNCLOUD_PUBLIC_REGISTRATION", "open");
    std::env::set_var("BURNCLOUD_PUBLIC_SIGNUP_BONUS_USD", "1.25");

    let db = Database::new().await.expect("fresh sqlite database");
    UserDatabase::init(&db).await.expect("user schema/roles");
    initialize_bootstrap_state(&db)
        .await
        .expect("bootstrap state initialization");

    let missing = register_public_user(&db, &input("missing-token", None)).await;
    assert_eq!(missing, Err(PublicRegistrationError::BootstrapRequired));

    let wrong = register_public_user(&db, &input("wrong-token", Some("wrong-wrong-wrong-wrong"))).await;
    assert_eq!(wrong, Err(PublicRegistrationError::InvalidBootstrapToken));

    let admin = register_public_user(
        &db,
        &input(
            "bootstrap-admin",
            Some("0123456789abcdef-bootstrap-test-token"),
        ),
    )
    .await
    .expect("trusted bootstrap must create admin");
    assert_eq!(admin.role, "admin");

    let connection = db.get_connection().expect("database connection");
    let admin_balance: i64 = sqlx::query_scalar::<Any, i64>(
        "SELECT balance_usd FROM user_accounts WHERE id = ?",
    )
    .bind(&admin.user_id)
    .fetch_one(connection.pool())
    .await
    .expect("admin balance");
    assert_eq!(admin_balance, 0, "bootstrap admin must not receive signup credit");

    let marker_count: i64 = sqlx::query_scalar::<Any, i64>(
        "SELECT COUNT(*) FROM burncloud_bootstrap_state WHERE id = 1",
    )
    .fetch_one(connection.pool())
    .await
    .expect("bootstrap marker");
    assert_eq!(marker_count, 1);

    let replay = register_public_user(
        &db,
        &input(
            "bootstrap-replay",
            Some("0123456789abcdef-bootstrap-test-token"),
        ),
    )
    .await;
    assert_eq!(replay, Err(PublicRegistrationError::BootstrapAlreadyComplete));

    let ordinary = register_public_user(&db, &input("ordinary-user", None))
        .await
        .expect("open post-bootstrap registration");
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
    .bind(&admin.user_id)
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
    std::env::remove_var("BURNCLOUD_BOOTSTRAP_TOKEN");
    std::env::remove_var("BURNCLOUD_PUBLIC_REGISTRATION");
    std::env::remove_var("BURNCLOUD_PUBLIC_SIGNUP_BONUS_USD");
}
