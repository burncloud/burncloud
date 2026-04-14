# burncloud-database-upstream

上游/Channel 配置持久化。提供 API Key 的 AES-256-GCM 加密存储。

## 关键类型

| 类型 | 说明 |
|------|------|
| `UpstreamModel` | 上游 CRUD(静态方法) |
| `DbUpstream` | 上游数据模型 |
| `encrypt_api_key()` | AES-256-GCM 加密 |
| `decrypt_api_key()` | AES-256-GCM 解密 |
| `get_master_key()` | 从环境变量获取加密主密钥 |

## 依赖

- `burncloud-database` — 核心数据库抽象
- `aes-gcm`, `hex`, `rand` — 加密相关
