# error.rs 数据结构

## 公有数据结构

### DownloadError

#### 完整定义
```rust
#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("数据库错误: {0}")]
    Database(#[from] burncloud_database::DatabaseError),
    #[error("Aria2错误: {0}")]
    Aria2(#[from] burncloud_download_aria2::Aria2Error),
}
```

#### 内部参数
| 变体 | 关联数据 | 属性 | 信息 |
|---|---|---|---|
| `Database` | `burncloud_database::DatabaseError` | `#[from]`、错误文本为 `数据库错误: {0}` | 表示数据库操作失败；支持从 `DatabaseError` 自动转换。 |
| `Aria2` | `burncloud_download_aria2::Aria2Error` | `#[from]`、错误文本为 `Aria2错误: {0}` | 表示 aria2 操作失败；支持从 `Aria2Error` 自动转换。 |

## 私有数据结构

无。
