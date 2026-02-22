# burncloud-core

核心工具和配置管理。

## 架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            burncloud-core                                    │
│                          (Core Utilities)                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────────────────┐  ┌──────────────────────────────────────┐ │
│  │      config_manager.rs       │  │        model_manager.rs              │ │
│  │                              │  │                                      │ │
│  │  ┌────────────────────────┐  │  │  ┌────────────────────────────────┐  │ │
│  │  │    ConfigManager       │  │  │  │      ModelManager              │  │ │
│  │  │────────────────────────│  │  │  │────────────────────────────────│  │ │
│  │  │ load_config()          │  │  │  │ list_models()                  │  │ │
│  │  │ save_config()          │  │  │  │ get_model_info()               │  │ │
│  │  │ get/set fields         │  │  │  │ download_model()               │  │ │
│  │  └────────────────────────┘  │  │  │ delete_model()                 │  │ │
│  │                              │  │  │ check_model_status()           │  │ │
│  │  配置文件管理               │  │  └────────────────────────────────┘  │ │
│  │                              │  │                                      │ │
│  └──────────────────────────────┘  │  模型生命周期管理                   │ │
│                                    └──────────────────────────────────────┘ │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 模块清单

| 模块 | 文件 | 职责 |
|------|------|------|
| **lib.rs** | `lib.rs` | 入口 |
| **config_manager** | `config_manager.rs` | 配置文件管理 |
| **model_manager** | `model_manager.rs` | 模型生命周期管理 |

## ConfigManager

```rust
pub struct ConfigManager {
    // 配置管理器
}

impl ConfigManager {
    pub fn load_config() -> Result<Config>;
    pub fn save_config(&self) -> Result<()>;
    // ...
}
```

## ModelManager

```rust
pub struct ModelManager {
    // 模型管理器
}

impl ModelManager {
    pub fn list_models() -> Result<Vec<ModelInfo>>;
    pub fn get_model_info(&self, name: &str) -> Result<Option<ModelInfo>>;
    pub fn download_model(&self, name: &str) -> Result<()>;
    pub fn delete_model(&self, name: &str) -> Result<()>;
    // ...
}
```

## 依赖关系

```
burncloud-core
├── burncloud-common        # 共享类型
└── external: serde, tokio
```

## 使用场景

- 本地模型管理
- 配置文件读写
- 模型下载和删除
