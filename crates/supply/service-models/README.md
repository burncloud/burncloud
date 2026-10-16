# BurnCloud Service Models

Supply-owned model resolution and HuggingFace discovery helpers.

本 crate 当前阶段只实现模型解析、Variant 兼容性过滤和 HuggingFace 发现辅助能力。
阶段三不新增数据库 CRUD 服务层，也不提前实现 UI、下载执行、Runtime 启动或进程管理。

## What this crate owns

- 模型清单和 Variant 解析；
- 本地模型 Resolver 契约及其实现；
- 模型 Variant 的硬件和 Runtime 兼容性过滤；
- HuggingFace 模型及文件发现；
- 下载 URL 和本地数据目录辅助函数。

## What this crate does not own

本 crate 不负责：

- HuggingFace 仓库元数据的数据库 CRUD；
- 模型下载执行、缓存写入、Runtime 启动或进程管理；
- ModelRouter、CLI、Client 或 Node 编排；
- 后续阶段才需要的通用 Repository、Factory、扩展点或其他抽象层。

旧的 `ModelService` CRUD facade 调用了 `burncloud-database-model` 中七个空实现方法；
#621 已移除该虚假能力，不应重新添加。

运行时模型能力事实由 `burncloud-database-model::ModelCapabilityModel` 持久化，定价仍由
Commerce 负责。

## Public API constraints

新增公开 API 必须同时满足以下要求：

- 默认保持私有，只有当前 Issue 或既有公共契约明确要求时才使用 `pub`；
- 使用中文 `///` 文档注释，说明用途、参数、返回值、`# Examples` 和注意事项；
- 对空字符串、非法枚举值、越界资源值和其他不合法输入进行公开参数校验；
- 使用 `Result` 表示可恢复错误，不得静默吞错；
- 不得对外部输入使用 `unwrap()` 或 `expect()`；
- 不得使用 `unsafe`。

公开 API 的 README 描述必须与真实实现同步，不得提前描述未实现能力。

## Testing and verification

测试代码与业务源码分离：新增集成测试放在 `tests/`，不得为了测试内部实现扩大 `pub` API。
每个新增公开 API 都必须覆盖正常情况、边界情况和错误情况；修复问题必须增加回归测试。

提交前运行：

```text
cargo fmt --all -- --check
cargo clippy -p burncloud-service-models --all-targets --all-features -- -D warnings
cargo test -p burncloud-service-models
```

## HuggingFace example

```rust
let models = ModelService::fetch_from_huggingface().await?;
```

该示例只表示现有 HuggingFace 发现接口，不表示本 crate 负责下载、缓存或数据库持久化。
