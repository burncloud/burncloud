# 003 实时查询预置目录项的工件状态

## 修改的信息

- 状态查询始终按查询到的完整仓库身份实时发现 GGUF，不再因本地下载记录为空而回退到有限的内置 Manifest；预置查询将已校验的精确目录项传入发现及硬件门槛合并逻辑。
- 增加 `downloadable`、`unavailable_reason`、`identity_status` 和每个工件的 `file_size_bytes`，保留原有平铺身份字段及仓库页面 `status` 兼容字段。待校验预置不会触发上游发现，也不会返回候选。
- 远端工件仅按 Variant ID、直接 URI、期望 SHA-256 三项完全匹配本地记录；“已下载”还要求成功状态、非空本地路径和实际摘要等于期望摘要。
- 仅上游不可用或发现超时才尝试本地回退；回退记录要求成功状态、正文件大小、GGUF 路径、完整身份生成的直接 URI、有效且相同的期望/实际 SHA-256，以及非空本地路径。`NotFound` 不回退。上游不可用时，即使返回经验证的本地复用记录，`downloadable` 仍为 `false` 并附带不可用原因。
- 无 GGUF、仓库/修订不存在、上游不可用及待校验身份均返回明确的不可下载状态，不把仓库页面地址交给下载流程。

## 修改的文件与文件内位置

- `crates/interfaces/server/src/model_selection/mod.rs:251`：限制状态查询参数；第 260 行定义工件状态和响应字段；第 343 行精准合并本地下载记录；第 382 行实现实时发现及不可用原因映射；第 462 行实现严格本地回退；第 507 行组装兼容状态响应；第 553 行处理预置、非预置和待校验身份查询。
- `crates/interfaces/server/src/model_selection/tests.rs:33`：扩展本地 ModelScope/Hugging Face 模拟上游；第 328 行覆盖 ModelScope 预置和仓库错误状态；第 398 行覆盖无内置 Manifest 时的实时发现；第 419 行覆盖待校验预置阻止发现；第 436 行覆盖上游不可用时的严格本地回退；第 534 行覆盖 `NotFound` 不回退和无 GGUF；第 614 行覆盖非法状态查询参数。
- `crates/interfaces/server/src/api/model_resolution.rs:625`：历史本地工件只在身份、GGUF 路径、大小、摘要与直接 URI 验证通过后重建，并合并内置及预置硬件门槛。

## 检查结果

- `cargo fmt --all`、`cargo fmt --all -- --check`、`git diff --check`：通过。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。
- `cargo test --workspace`：单元、集成和大部分文档测试通过；Node Runtime 文档测试并发链接时触发 MSVC `LNK1318: FILE_SYSTEM (3)` 临时 PDB 错误，命令最终未通过。
- 对上述工具链问题串行复验：`CARGO_BUILD_JOBS=1 cargo test -p burncloud-node-runtime --doc -- --test-threads=1`：48 项通过。
- `cargo test --workspace` 中服务端库测试：105 项通过、2 项因需本机 llama-server 与 GGUF 而忽略；其余已执行测试未报告失败。
