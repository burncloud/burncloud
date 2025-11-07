//! # BurnCloud Service Setting
//!
//! 设置服务层，提供简洁的增删改查接口

use burncloud_database_setting::SettingDatabase;

type Result<T> = std::result::Result<T, burncloud_database_setting::DatabaseError>;

/// 设置服务
pub struct SettingService {
    db: SettingDatabase,
}

impl SettingService {
    /// 创建新的设置服务实例
    pub async fn new() -> Result<Self> {
        Ok(Self {
            db: SettingDatabase::new().await?,
        })
    }

    /// 设置配置项（添加或更新）
    pub async fn set(&self, name: &str, value: &str) -> Result<()> {
        self.db.set(name, value).await
    }

    /// 获取配置值
    pub async fn get(&self, name: &str) -> Result<Option<String>> {
        self.db.get(name).await
    }

    /// 删除配置项
    pub async fn delete(&self, name: &str) -> Result<()> {
        self.db.delete(name).await
    }

    /// 获取所有配置项
    pub async fn list_all(&self) -> Result<Vec<burncloud_database_setting::Setting>> {
        self.db.list_all().await
    }

    /// 关闭服务
    pub async fn close(self) -> Result<()> {
        self.db.close().await
    }
}

/// 重新导出常用类型
pub use burncloud_database_setting::{DatabaseError, Setting};
