//! Database persistence for installer module

use burncloud_database::{Database, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Installation record
#[derive(sqlx::FromRow, Serialize, Deserialize, Clone)]
pub struct SysInstallation {
    /// Software ID
    pub software_id: String,
    /// Software name
    pub name: String,
    /// Installed version
    pub version: Option<String>,
    /// Installation status
    pub status: String,
    /// Installation directory
    pub install_dir: Option<String>,
    /// Install method used
    pub install_method: Option<String>,
    /// Installation date
    pub installed_at: Option<String>,
    /// Last updated date
    pub updated_at: String,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

/// Installer database
pub struct InstallerDB {
    db: Database,
}

impl InstallerDB {
    /// Create new installer database
    pub async fn new() -> Result<Self> {
        let db = Database::new().await?;
        let instance = Self { db };
        instance.init_tables().await?;
        Ok(instance)
    }

    /// Initialize database tables
    async fn init_tables(&self) -> Result<()> {
        self.db
            .execute_query(
                r#"
            CREATE TABLE IF NOT EXISTS sys_installations (
                software_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT,
                status TEXT NOT NULL DEFAULT 'not_installed',
                install_dir TEXT,
                install_method TEXT,
                installed_at DATETIME,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                error_message TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_sys_installations_status ON sys_installations(status);
            CREATE INDEX IF NOT EXISTS idx_sys_installations_installed_at ON sys_installations(installed_at);
        "#,
            )
            .await?;
        Ok(())
    }

    /// Get current timestamp as ISO 8601 string
    fn now_string() -> String {
        Utc::now().to_rfc3339()
    }

    /// Add or update installation record
    pub async fn upsert(&self, record: &SysInstallation) -> Result<()> {
        self.db
            .execute_query_with_params(
                r#"
                INSERT INTO sys_installations
                    (software_id, name, version, status, install_dir, install_method, installed_at, updated_at, error_message)
                VALUES
                    (?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(software_id) DO UPDATE SET
                    name = excluded.name,
                    version = excluded.version,
                    status = excluded.status,
                    install_dir = excluded.install_dir,
                    install_method = excluded.install_method,
                    installed_at = COALESCE(excluded.installed_at, sys_installations.installed_at),
                    updated_at = excluded.updated_at,
                    error_message = excluded.error_message
                "#,
                vec![
                    record.software_id.clone(),
                    record.name.clone(),
                    record.version.clone().unwrap_or_default(),
                    record.status.clone(),
                    record.install_dir.clone().unwrap_or_default(),
                    record.install_method.clone().unwrap_or_default(),
                    record.installed_at.clone().unwrap_or_default(),
                    record.updated_at.clone(),
                    record.error_message.clone().unwrap_or_default(),
                ],
            )
            .await?;
        Ok(())
    }

    /// Get installation record by software ID
    pub async fn get(&self, software_id: &str) -> Result<Option<SysInstallation>> {
        self.db
            .fetch_optional_with_params::<SysInstallation>(
                "SELECT * FROM sys_installations WHERE software_id = ?",
                vec![software_id.to_string()],
            )
            .await
    }

    /// List all installations
    pub async fn list(&self, status: Option<&str>) -> Result<Vec<SysInstallation>> {
        match status {
            Some(s) => {
                self.db
                    .fetch_all_with_params::<SysInstallation>(
                        "SELECT * FROM sys_installations WHERE status = ? ORDER BY updated_at DESC",
                        vec![s.to_string()],
                    )
                    .await
            }
            None => {
                self.db
                    .fetch_all::<SysInstallation>(
                        "SELECT * FROM sys_installations ORDER BY updated_at DESC",
                    )
                    .await
            }
        }
    }

    /// Update installation status
    pub async fn update_status(
        &self,
        software_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.db
            .execute_query_with_params(
                r#"
                UPDATE sys_installations
                SET status = ?, error_message = ?, updated_at = ?
                WHERE software_id = ?
                "#,
                vec![
                    status.to_string(),
                    error_message.unwrap_or("").to_string(),
                    Self::now_string(),
                    software_id.to_string(),
                ],
            )
            .await?;
        Ok(())
    }

    /// Delete installation record
    pub async fn delete(&self, software_id: &str) -> Result<()> {
        self.db
            .execute_query_with_params(
                "DELETE FROM sys_installations WHERE software_id = ?",
                vec![software_id.to_string()],
            )
            .await?;
        Ok(())
    }

    /// Mark software as installed
    pub async fn mark_installed(
        &self,
        software_id: &str,
        name: &str,
        version: Option<&str>,
        install_dir: Option<&str>,
        install_method: Option<&str>,
    ) -> Result<()> {
        let record = SysInstallation {
            software_id: software_id.to_string(),
            name: name.to_string(),
            version: version.map(|s| s.to_string()),
            status: "installed".to_string(),
            install_dir: install_dir.map(|s| s.to_string()),
            install_method: install_method.map(|s| s.to_string()),
            installed_at: Some(Self::now_string()),
            updated_at: Self::now_string(),
            error_message: None,
        };
        self.upsert(&record).await
    }

    /// Mark software as installing
    pub async fn mark_installing(&self, software_id: &str, name: &str) -> Result<()> {
        let record = SysInstallation {
            software_id: software_id.to_string(),
            name: name.to_string(),
            version: None,
            status: "installing".to_string(),
            install_dir: None,
            install_method: None,
            installed_at: None,
            updated_at: Self::now_string(),
            error_message: None,
        };
        self.upsert(&record).await
    }

    /// Mark software as failed
    pub async fn mark_failed(&self, software_id: &str, error_message: &str) -> Result<()> {
        self.update_status(software_id, "failed", Some(error_message))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installation_record() {
        let record = SysInstallation {
            software_id: "test-software".to_string(),
            name: "Test Software".to_string(),
            version: Some("1.0.0".to_string()),
            status: "installed".to_string(),
            install_dir: Some("/usr/local/test".to_string()),
            install_method: Some("script".to_string()),
            installed_at: Some("2024-01-01T00:00:00Z".to_string()),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            error_message: None,
        };

        assert_eq!(record.software_id, "test-software");
        assert_eq!(record.status, "installed");
    }
}
