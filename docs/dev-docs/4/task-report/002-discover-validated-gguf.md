# 002 实时发现并校验 GGUF 工件及硬件门槛

## 修改的信息

- `discover_manifest_at` 现在返回动态 Manifest 与 `Variant ID → 实际文件大小` 列表，支持后续状态和解析使用真实文件大小。
- ModelScope 与 Hugging Face 发现仅保留路径安全、后缀为 GGUF、SHA-256 为 64 位十六进制且文件大小大于零的文件；上游文件元数据缺失按 `Unavailable` 处理。
- 下载工件 URI 仍由固定来源和仓库身份生成 `/resolve/` 直接文件地址，不使用仓库页面地址；摘要统一规范为小写 `sha256:` 格式。
- 内置 Manifest 与实际文件基线按逐项最大值合并；预置最低 VRAM/RAM/磁盘需求按 GiB（二进制 1024³）转换后继续取最大值。小数 GiB 使用向上取整，溢出和格式错误返回 `Invalid`；推荐值不参与门槛。
- 无预置 ID 的既有解析路径继续传入 `None` 并保留原有 Manifest；待校验身份不应用推测的预置硬件门槛，后续解析准入由任务 004 完成。

## 修改的文件与文件内位置

- `crates/interfaces/server/src/api/model_resolution.rs:203`：解析入口兼容新的发现返回值；第 282 行 `discover_manifest`；第 307 行 `discover_manifest_at` 与实际文件大小配对；第 345 行内置资源配置最大值合并；第 378 行预置硬件合并；第 401 行 GiB 安全换算；第 485/538 行 ModelScope 与 Hugging Face 元数据筛选；第 584 行来源 API URL 校验；第 642/662 行 Variant 与受信直接文件 URL 构造；第 739 行文件路径校验。
- `crates/interfaces/server/src/api/model_resolution_tests.rs:242`：模拟仓库服务；第 282/328 行 ModelScope、Hugging Face 文件发现与直接 URL 测试；第 367 行上游错误语义；第 401/414 行配置合并与预置门槛；第 435 行 GiB 边界和溢出；第 450/460/478 行非预置兼容、缺少元数据和待校验身份回归。
- `crates/interfaces/server/src/model_selection/mod.rs:3`：提供 crate 内预置目录访问；第 378 行状态发现调用适配返回值与可选预置参数。

## 检查结果

- ModelScope/Hugging Face 仅接受有效 GGUF 路径、SHA-256 和正大小；丢弃缺失/非法 SHA、零大小、非 GGUF、目录及路径穿越项：通过模拟上游测试。
- 两类来源均生成固定站点的直接 `/resolve/` 文件 URL，未使用资源页面 URL；真实文件大小和规范化摘要随 Variant 正确返回：通过。
- 内置配置、预置下限与文件大小按最大值合并；`1G`、`1.5G`、`0.1G` 二进制换算、溢出和无效单位覆盖：通过。
- 非预置 Manifest 保持原门槛；404 映射 `NotFound`，上游故障或 ModelScope 成功响应缺少文件元数据映射 `Unavailable`，非法仓库身份映射 `Invalid`：通过。
- 定向测试 `cargo test -p burncloud-server --lib api::model_resolution::tests -- --nocapture`：19 项通过。
- 服务端单元测试 `cargo test -p burncloud-server --lib`：103 项通过、2 项因依赖本机 llama-server/GGUF 而忽略。
- `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`git diff --check`：通过。
