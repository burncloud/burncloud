# burncloud-router-aws

AWS 请求签名工具。实现 AWS SigV4 签名,用于 Bedrock 等 AWS API 认证。

## 关键类型

| 类型 | 说明 |
|------|------|
| `AwsConfig` | AWS 配置(access_key, secret_key, region) |
| `sign_request()` | HTTP 请求签名 |

## 依赖

- `reqwest`, `http` — HTTP 相关
- `sha2`, `hmac`, `hex` — 签名计算
- `time`, `chrono` — 时间戳
