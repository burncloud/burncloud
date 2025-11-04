# Aria2 下载数据库设计文档

## 概述

本项目为 BurnCloud Aria2 下载器设计一个极度精简的数据库系统，专门用于管理下载任务。

基于 `../burncloud-database` 架构，使用 SQLite 作为后端数据库。

## 设计原则

1. **极度精简**: 最少的表结构和字段
2. **功能完整**: 满足 aria2 下载管理的核心需求
3. **高性能**: 优化的索引和查询结构
4. **易扩展**: 为未来功能预留扩展空间

## 数据库表结构

### 下载任务表 (downloads)

存储 aria2 下载任务的核心信息。

```sql
CREATE TABLE downloads (
    -- 基础字段
    gid TEXT PRIMARY KEY,                     -- aria2 全局唯一标识符
    status TEXT NOT NULL DEFAULT 'waiting',  -- 任务状态: active/waiting/paused/error/complete/removed

    -- 下载信息
    uris TEXT NOT NULL,                       -- 下载链接(JSON数组格式)
    total_length INTEGER DEFAULT 0,          -- 总大小(字节)
    completed_length INTEGER DEFAULT 0,      -- 已完成大小(字节)
    download_speed INTEGER DEFAULT 0,        -- 下载速度(字节/秒)

    -- 文件信息
    download_dir TEXT,                        -- 下载目录
    filename TEXT,                            -- 文件名

    -- 配置选项
    connections INTEGER DEFAULT 16,          -- 连接数
    split INTEGER DEFAULT 5,                 -- 分片数

    -- 时间戳
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

## 索引设计

```sql
-- 下载任务查询优化
CREATE INDEX idx_downloads_status ON downloads(status);
CREATE INDEX idx_downloads_created_at ON downloads(created_at);
CREATE INDEX idx_downloads_updated_at ON downloads(updated_at);
```

## 数据类型说明

### URIs 字段格式
```json
["http://example.com/file.zip", "ftp://mirror.com/file.zip"]
```

### 状态枚举
- `active`: 正在下载
- `waiting`: 等待中
- `paused`: 已暂停
- `error`: 下载错误
- `complete`: 下载完成
- `removed`: 已删除

## 核心操作接口

### 下载管理
```rust
// 添加下载任务
async fn add_download(gid: &str, uris: Vec<String>, options: DownloadOptions) -> Result<()>

// 更新下载进度
async fn update_progress(gid: &str, completed: i64, speed: i64) -> Result<()>

// 更新下载状态
async fn update_status(gid: &str, status: &str) -> Result<()>

// 查询下载任务
async fn get_download(gid: &str) -> Result<Option<Download>>

// 列出下载任务(按状态筛选)
async fn list_downloads(status: Option<&str>) -> Result<Vec<Download>>

// 删除下载任务
async fn delete_download(gid: &str) -> Result<()>
```

## 数据结构定义

```rust
#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Download {
    pub gid: String,
    pub status: String,
    pub uris: String,  // JSON 格式
    pub total_length: i64,
    pub completed_length: i64,
    pub download_speed: i64,
    pub download_dir: Option<String>,
    pub filename: Option<String>,
    pub connections: i32,
    pub split: i32,
    pub created_at: String,
    pub updated_at: String,
}
```

## 特色功能

1. **状态追踪**: 完整的下载状态生命周期管理
2. **JSON 存储**: 复杂数据(如 URI 列表)使用 JSON 格式存储
3. **自动更新时间戳**: 使用触发器自动更新 updated_at 字段

## 部署说明

1. 自动创建表结构和索引
