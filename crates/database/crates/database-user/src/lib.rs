use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, FromRow};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: Option<String>, // Nullable for OIDC users
    pub github_id: Option<String>,
    // created_at handled by DB
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbRole {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbUserRole {
    pub user_id: String,
    pub role_id: String,
}

pub struct UserDatabase;

impl UserDatabase {
    pub async fn init(db: &Database) -> Result<()> {
        let conn = db.connection()?;
        let kind = db.kind();
        
        // Table definitions
        let (users_sql, roles_sql, user_roles_sql) = match kind {
            sqlx::any::AnyKind::Sqlite => (
                r#"
                CREATE TABLE IF NOT EXISTS users (
                    id TEXT PRIMARY KEY,
                    username TEXT NOT NULL UNIQUE,
                    email TEXT UNIQUE,
                    password_hash TEXT,
                    github_id TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS roles (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    description TEXT
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS user_roles (
                    user_id TEXT NOT NULL,
                    role_id TEXT NOT NULL,
                    PRIMARY KEY (user_id, role_id),
                    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
                    FOREIGN KEY(role_id) REFERENCES roles(id) ON DELETE CASCADE
                );
                "#
            ),
            sqlx::any::AnyKind::Postgres => (
                r#"
                CREATE TABLE IF NOT EXISTS users (
                    id TEXT PRIMARY KEY,
                    username TEXT NOT NULL UNIQUE,
                    email TEXT UNIQUE,
                    password_hash TEXT,
                    github_id TEXT,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS roles (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    description TEXT
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS user_roles (
                    user_id TEXT NOT NULL,
                    role_id TEXT NOT NULL,
                    PRIMARY KEY (user_id, role_id),
                    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
                    FOREIGN KEY(role_id) REFERENCES roles(id) ON DELETE CASCADE
                );
                "#
            )
        };

        sqlx::query(users_sql).execute(conn.pool()).await?;
        sqlx::query(roles_sql).execute(conn.pool()).await?;
        sqlx::query(user_roles_sql).execute(conn.pool()).await?;

        // Initialize default roles
        let role_count: i64 = sqlx::query("SELECT COUNT(*) FROM roles")
            .fetch_one(conn.pool())
            .await?
            .get(0);
        
        if role_count == 0 {
            sqlx::query("INSERT INTO roles (id, name, description) VALUES ('role-admin', 'admin', 'Administrator'), ('role-user', 'user', 'Standard User')")
                .execute(conn.pool())
                .await?;
        }

        // Ensure demo user exists
        let user_count: i64 = sqlx::query("SELECT COUNT(*) FROM users WHERE username = 'demo-user'")
            .fetch_one(conn.pool())
            .await?
            .get(0);
            
        if user_count == 0 {
            sqlx::query("INSERT INTO users (id, username) VALUES ('demo-user', 'demo-user')")
                .execute(conn.pool())
                .await?;
            // Assign admin role
            sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ('demo-user', 'role-admin')")
                .execute(conn.pool())
                .await?;
        }

        Ok(())
    }

    pub async fn create_user(db: &Database, user: &DbUser) -> Result<()> {
        let conn = db.connection()?;
        sqlx::query("INSERT INTO users (id, username, email, password_hash, github_id) VALUES (?, ?, ?, ?, ?)")
            .bind(&user.id)
            .bind(&user.username)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(&user.github_id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn get_user_by_username(db: &Database, username: &str) -> Result<Option<DbUser>> {
        let conn = db.connection()?;
        let user = sqlx::query_as::<_, DbUser>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(conn.pool())
            .await?;
        Ok(user)
    }

    pub async fn get_user_roles(db: &Database, user_id: &str) -> Result<Vec<String>> {
        let conn = db.connection()?;
        let rows = sqlx::query("SELECT r.name FROM roles r JOIN user_roles ur ON r.id = ur.role_id WHERE ur.user_id = ?")
            .bind(user_id)
            .fetch_all(conn.pool())
            .await?;
        
        let roles = rows.iter().map(|r| r.get(0)).collect();
        Ok(roles)
    }

    pub async fn assign_role(db: &Database, user_id: &str, role_name: &str) -> Result<()> {
        let conn = db.connection()?;
        let role_id: Option<String> = sqlx::query("SELECT id FROM roles WHERE name = ?")
            .bind(role_name)
            .fetch_optional(conn.pool())
            .await?
            .map(|r| r.get(0));
            
        if let Some(rid) = role_id {
            let res = sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES (?, ?)")
                .bind(user_id)
                .bind(rid)
                .execute(conn.pool())
                .await;
            
            if let Err(e) = res {
                println!("Role assignment skipped (maybe already exists): {}", e);
            }
        }
        Ok(())
    }
}
