# 005 客户端目录、状态和解析协议校验

## 修改的信息

- 客户端空查询加载预置目录时校验目录恰好包含 388 项，并验证每项身份及 ID 唯一性；非空搜索仍使用现有 NDJSON 累计快照协议。
- 状态响应新增身份校验状态、可下载标志、不可用原因和实际文件大小解析。客户端拒绝与查询身份不符的状态或解析结果，并校验 GGUF 直接文件链接的来源、仓库、修订号和文件路径、SHA-256、非零文件大小、硬件字段及本地文件校验凭据。
- 预置模型解析请求携带 `preset_id` 与仓库身份，非预置搜索结果继续省略 `preset_id`，保持旧请求路径兼容。
- 更新客户端测试数据及回归测试，覆盖 388 项目录、重复 ID、预置/搜索请求体差异、状态凭据校验和伪造解析工件拒绝。

## 修改的文件与文件内位置

| 文件 | 位置 | 修改 |
| --- | --- | --- |
| `crates/interfaces/client/src/domains/chat/catalog.rs` | `CatalogModel`、`CatalogInspection`、`CatalogClient::{models, inspect, resolve_model}`、响应/凭据校验函数 | 实现目录数量与身份校验、状态及解析协议反序列化和下载凭据校验。 |
| `crates/interfaces/client/src/domains/chat/model.rs` | `ModelVariant` | 增加实际文件大小字段，供状态及硬件/磁盘匹配使用。 |
| `crates/interfaces/client/src/domains/chat/backend.rs` | 本地模型 Variant 构造 | 初始化新增文件大小字段。 |
| `crates/interfaces/client/src/domains/chat/catalog_tests.rs` | 目录、状态、解析接口测试 | 增加全目录数量/唯一性、预置与搜索请求、状态凭据及不可信响应回归覆盖。 |
| `crates/interfaces/client/src/domains/chat/backend_tests.rs` | Backend 测试构造 | 补齐 Variant 测试数据字段。 |
| `crates/interfaces/client/src/domains/chat/page_task_tests.rs` | 页面任务测试构造 | 补齐 Variant 测试数据字段。 |
| `crates/interfaces/client/src/domains/chat/selector_tests.rs` | 选择器测试构造 | 更新协议测试数据及新增字段。 |
| `crates/interfaces/client/src/domains/chat/tests.rs` | Chat 测试模块 | 注册客户端目录协议测试。 |
| `crates/interfaces/client/src/domains/chat/ui_tests.rs` | UI/状态交接测试构造 | 更新目录、状态及 Variant 响应测试数据。 |

## 检查结果

- `cargo fmt --all -- --check`：通过。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。
- `cargo test -p burncloud-client --lib`：通过，232 项测试通过。
- 全工作区 `CARGO_BUILD_JOBS=1 cargo test --workspace` 已执行；单元及集成测试通过，但 Windows MSVC 在 Node Runtime doctest 链接阶段报 `LNK1318: FILE_SYSTEM (3)`。随后 `cargo test -p burncloud-node-runtime --doc -- --test-threads=1` 单独复验通过，48 项 doctest 通过。
- `git diff --check`：通过。
