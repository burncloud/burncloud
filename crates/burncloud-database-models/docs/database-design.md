# AI 模型信息数据库设计文档

## 概述

本项目为 BurnCloud AI 模型信息管理系统设计一个精简的数据库结构，专门用于存储和管理 AI 模型的核心信息。

基于 `../burncloud-database` 架构，使用 SQLite 作为后端数据库。

## 设计原则

1. **极度精简**: 最少的表结构和字段
2. **功能完整**: 满足 AI 模型信息管理的核心需求
3. **高性能**: 优化的索引和查询结构
4. **易扩展**: 为未来功能预留扩展空间

## 数据库表结构

### AI 模型信息表 (models)

存储 AI 模型的核心信息和元数据。

```sql
CREATE TABLE models (
    -- 基础字段
    model_id TEXT PRIMARY KEY,                   -- 模型唯一标识符 (如: openai-community/gpt2)
    private BOOLEAN NOT NULL DEFAULT FALSE,     -- 是否私有模型

    -- 模型分类
    pipeline_tag TEXT,                           -- 模型管道类型 (如: text-generation)
    library_name TEXT,                           -- 库名称 (如: transformers)
    model_type TEXT,                             -- 模型类型 (如: gpt2)

    -- 统计信息
    downloads INTEGER DEFAULT 0,                -- 下载次数
    likes INTEGER DEFAULT 0,                    -- 点赞数

    -- 版本信息
    sha TEXT,                                    -- Git SHA
    last_modified DATETIME,                      -- 最后修改时间
    gated BOOLEAN DEFAULT FALSE,                 -- 是否需要授权访问
    disabled BOOLEAN DEFAULT FALSE,              -- 是否已禁用

    -- JSON 存储字段
    tags TEXT,                                   -- 标签列表 (JSON数组)
    config TEXT,                                 -- 模型配置 (JSON对象)
    widget_data TEXT,                            -- 示例数据 (JSON数组)
    card_data TEXT,                              -- 卡片元数据 (JSON对象)
    transformers_info TEXT,                      -- Transformers信息 (JSON对象)
    siblings TEXT,                               -- 相关文件列表 (JSON数组)
    spaces TEXT,                                 -- 关联空间列表 (JSON数组)
    safetensors TEXT,                            -- SafeTensors信息 (JSON对象)

    -- 存储信息
    used_storage INTEGER DEFAULT 0,             -- 使用的存储空间(字节)

    -- 时间戳
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

## 索引设计

```sql
-- 模型查询优化
CREATE INDEX idx_models_pipeline_tag ON models(pipeline_tag);
CREATE INDEX idx_models_library_name ON models(library_name);
CREATE INDEX idx_models_downloads ON models(downloads DESC);
CREATE INDEX idx_models_likes ON models(likes DESC);
CREATE INDEX idx_models_created_at ON models(created_at);
CREATE INDEX idx_models_private ON models(private);
```

## 数据类型说明

### JSON 字段格式示例

**tags 字段**:
```json
["transformers", "pytorch", "text-generation", "gpt2"]
```

**config 字段**:
```json
{
  "architectures": ["GPT2LMHeadModel"],
  "model_type": "gpt2",
  "tokenizer_config": {}
}
```

**siblings 字段**:
```json
[
  {"rfilename": "config.json"},
  {"rfilename": "model.safetensors"},
  {"rfilename": "tokenizer.json"}
]
```

## 核心操作接口

### 模型管理
```rust
// 添加模型信息
async fn add_model(model: &ModelInfo) -> Result<()>

async fn delete(model_id: &str) -> Result<()>
```

## 数据结构定义

```rust
#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub model_id: String,
    pub private: bool,
    pub pipeline_tag: Option<String>,
    pub library_name: Option<String>,
    pub model_type: Option<String>,
    pub downloads: i64,
    pub likes: i64,
    pub sha: Option<String>,
    pub last_modified: Option<String>,
    pub gated: bool,
    pub disabled: bool,
    pub tags: String,           // JSON 格式
    pub config: String,         // JSON 格式
    pub widget_data: String,    // JSON 格式
    pub card_data: String,      // JSON 格式
    pub transformers_info: String, // JSON 格式
    pub siblings: String,       // JSON 格式
    pub spaces: String,         // JSON 格式
    pub safetensors: String,    // JSON 格式
    pub used_storage: i64,
    pub created_at: String,
    pub updated_at: String,
}
```

## 特色功能

1. **多维检索**: 支持按类型、标签、热度等多种方式检索模型
2. **JSON 存储**: 复杂数据结构使用 JSON 格式存储，保持数据完整性
3. **统计追踪**: 记录下载量、点赞数等关键指标
4. **版本管理**: 通过 SHA 和修改时间追踪模型版本

## 部署说明

1. 自动创建表结构和索引
2. 支持批量导入模型数据
3. 定期更新统计信息