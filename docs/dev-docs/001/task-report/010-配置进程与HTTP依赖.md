# 010 配置进程与 HTTP 依赖

## 修改的信息

- Node crate 继承工作区 `reqwest` 依赖，为 HTTP readiness 与 health 探测提供依赖。
- Node crate 显式启用工作区 Tokio 依赖的 `process` feature，为真实子进程管理提供能力。

## 修改的文件与文件内位置

- `crates/platform/node/Cargo.toml`：`[dependencies]` 中的 `reqwest` 与 `tokio` 条目。

## 检查结果

- `reqwest.workspace = true`：使用工作区统一版本。通过清单检查。
- Tokio `process` feature：已显式启用。通过清单检查。
- 未增加开发文档之外的依赖或能力。通过清单检查。
- `cargo fmt --all`：通过。
- `cargo clippy --offline --workspace --all-targets` 与 `cargo test --offline --workspace`：未完成；离线缓存缺少 `aws-lc-rs`，为工作区既有 `reqwest` / `rustls` 依赖所需。
