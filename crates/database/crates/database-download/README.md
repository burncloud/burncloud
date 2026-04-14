# burncloud-database-download

Aria2 下载任务持久化。追踪下载状态、进度、速度和配置。

## 关键类型

| 类型 | 说明 |
|------|------|
| `DownloadDB` | 下载任务 CRUD + `init(&db)` |
| `Download` | 下载数据模型(含 status, progress, speed, config) |

## 依赖

- `burncloud-database` — 核心数据库抽象
