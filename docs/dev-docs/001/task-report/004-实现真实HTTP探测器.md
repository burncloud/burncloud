# 004 实现真实 HTTP 探测器

## 修改的信息

- 新增 `HttpProbe`，构造时创建带 10 秒单次请求超时的 reqwest client，并由 readiness 与 health 共享。
- readiness 仅请求传入的 endpoint；2xx 返回成功，非 2xx 和传输错误等待 1 秒后重试。请求构造错误映射为 `ReadinessError::CheckFailed`。
- health 对 endpoint 只请求一次，2xx 返回 `true`、非 2xx 返回 `false`，请求错误映射为 `HealthError::CheckFailed`。
- 按授权在 `deployment/mod.rs` 声明模块并导出 `HttpProbe`。

## 修改的文件与文件内位置

- `crates/platform/node/src/deployment/http_probe.rs`：`HttpProbe`、默认 client 配置及 `ReadinessProbe`、`HealthProbe` 实现。
- `crates/platform/node/src/deployment/mod.rs`：声明 `http_probe` 模块并导出 `HttpProbe`。

## 检查结果

- readiness 成功状态、单次请求超时、失败后 1 秒重试及无总时限：通过代码检查。
- health 对 2xx、非 2xx 和请求错误的返回映射：通过代码检查。
- 两种探测共用同一 client，且只使用传入的 endpoint：通过代码检查。
- `cargo fmt --all -- --check` 与 `git diff --check`：通过。
- `cargo clippy --offline -p burncloud-node-runtime --all-targets`：未完成；离线缓存缺少 `aws-lc-rs`。
