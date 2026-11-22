# 007 同步服务端 OpenAPI 与接口说明

## 修改的信息

- OpenAPI 新增 `/api/models/presets`、`/api/models/search` 和 `/api/models/status` 描述，明确完整 388 项目录、NDJSON 累计搜索快照、两种状态查询形式、`downloadable` 与硬件兼容的区别，以及直接 GGUF URL、SHA-256、文件大小和本地校验凭据。
- 更新 `/api/node/models/resolve` 的可选 `preset_id` 绑定约束、旧非预置请求兼容、Node 实测硬件准入、有效工件条件、业务 reason 和错误码；补充上游不可用的 502 响应。
- 更新服务端接口说明，移除旧的 17 项 Manifest 结论，记录当前 388 项目录、18 项精选顺序、状态/解析请求响应、错误映射和资源门槛规则。
- 新增 OpenAPI 契约单测，检查目录数量/schema、搜索媒体类型、状态候选字段及预置解析字段和错误码。

## 修改的文件与文件内位置

| 文件 | 位置 | 修改 |
| --- | --- | --- |
| `crates/interfaces/server/src/api/openapi.rs` | `generate_paths` | 添加预置目录、外部搜索、工件状态三个公开路径及响应/参数定义；更新模型解析说明和 502 错误响应。 |
| `crates/interfaces/server/src/api/openapi.rs` | `generate_schemas` | 添加目录、六项硬件要求、搜索行、状态 Variant、不可用原因和目录错误 schema；扩展解析请求、reason 与错误码。 |
| `crates/interfaces/server/src/api/openapi_tests.rs` | OpenAPI 单元测试 | 覆盖 388 项、身份和硬件字段、NDJSON、状态凭据、预置 ID 及解析错误码。 |
| `crates/interfaces/server/docs/hardware-model-resolution-api.md` | 全文 | 同步为当前模型目录/搜索/状态、Node 硬件和解析接口契约。 |

## 检查结果

- `cargo fmt --all`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。
- `cargo test -p burncloud-server --lib`：通过，110 项通过、2 项因需要本地 llama-server/GGUF 环境而忽略。
- `git diff --check`：通过。
