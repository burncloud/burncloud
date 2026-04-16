//! # BurnCloud Service Setting
//!
//! 设置服务层，提供简洁的增删改查接口

type Result<T> = std::result::Result<T, burncloud_database_setting::DatabaseError>;

/// 设置服务（无状态）
pub struct SettingService;

impl SettingService {
    /// 设置配置项（添加或更新）
    pub async fn set(db: &SettingDatabase, name: &str, value: &str) -> Result<()> {
        db.set(name, value).await
    }

    /// 获取配置值
    pub async fn get(db: &SettingDatabase, name: &str) -> Result<Option<String>> {
        db.get(name).await
    }

    /// 删除配置项
    pub async fn delete(db: &SettingDatabase, name: &str) -> Result<()> {
        db.delete(name).await
    }

    /// 获取所有配置项
    pub async fn list_all(
        db: &SettingDatabase,
    ) -> Result<Vec<burncloud_database_setting::SysSetting>> {
        db.list_all().await
    }
}

/// 重新导出常用类型
pub use burncloud_database_setting::{DatabaseError, SettingDatabase, SysSetting};
