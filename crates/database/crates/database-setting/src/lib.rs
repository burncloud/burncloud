//! # BurnCloud Database Setting
//!
//! 设置管理数据库库,基于 `burncloud-database` 构建
//! 提供简单的键值对存储功能

use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 设置项结构体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Setting {
    /// 设置项名称(主键)
    pub name: String,
    /// 设置项值
    pub value: String,
}

/// 设置数据库管理器
pub struct SettingDatabase {
    db: Database,
}

impl SettingDatabase {
    /// 创建新的设置数据库实例
    pub async fn new() -> Result<Self> {
        let db = Database::new().await?;
        let setting_db = Self { db };
        setting_db.init_tables().await?;
        Ok(setting_db)
    }

    /// 初始化数据库表结构
    async fn init_tables(&self) -> Result<()> {
        let create_table_sql = r#"
            CREATE TABLE IF NOT EXISTS setting (
                name TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )
        "#;
        self.db.execute_query(create_table_sql).await?;
        Ok(())
    }

    /// 添加或更新设置项
    pub async fn set(&self, name: &str, value: &str) -> Result<()> {
        let sql = "INSERT OR REPLACE INTO setting (name, value) VALUES (?1, ?2)";
        let params = vec![name.to_string(), value.to_string()];
        self.db.execute_query_with_params(sql, params).await?;
        Ok(())
    }

    /// 根据名称获取设置值
    pub async fn get(&self, name: &str) -> Result<Option<String>> {
        let sql = "SELECT * FROM setting WHERE name = ?1";
        let params = vec![name.to_string()];
        let rows = self.db.query_with_params(sql, params).await?;

        if rows.is_empty() {
            Ok(None)
        } else {
            let setting: Setting = sqlx::FromRow::from_row(&rows[0])?;
            Ok(Some(setting.value))
        }
    }

    /// 删除设置项
    pub async fn delete(&self, name: &str) -> Result<()> {
        let sql = "DELETE FROM setting WHERE name = ?1";
        let params = vec![name.to_string()];
        self.db.execute_query_with_params(sql, params).await?;
        Ok(())
    }

    /// 获取所有设置项
    pub async fn list_all(&self) -> Result<Vec<Setting>> {
        let sql = "SELECT * FROM setting ORDER BY name";
        self.db.fetch_all::<Setting>(sql).await
    }

    /// 关闭数据库连接
    pub async fn close(self) -> Result<()> {
        self.db.close().await
    }
}

/// 重新导出 burncloud_database 的公共类型
pub use burncloud_database::{DatabaseConnection, DatabaseError};
