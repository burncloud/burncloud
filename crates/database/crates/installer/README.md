# burncloud-database-installer

软件安装记录持久化。追踪版本、状态、安装目录和安装方式。

## 关键类型

| 类型 | 说明 |
|------|------|
| `InstallerDB` | 安装记录 CRUD + `init(&db)` |
| `InstallationRecord` | 安装记录(含 version, status, directory, method) |

## 依赖

- `burncloud-database` — 核心数据库抽象
