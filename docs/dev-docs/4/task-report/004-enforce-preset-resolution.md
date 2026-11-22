# 004 按 preset_id 解析模型并执行 Node 硬件匹配

## 修改的信息

- 解析请求新增可选 `preset_id`，旧版 `model/source/revision` 请求仍走兼容路径；客户端传入的未知 URL 字段不参与解析，实际下载凭据只来自经校验的上游 GGUF 或可信本地回执。
- 服务端按稳定 ID 查预置目录，并严格比较目录项的模型、来源、修订号；未知 ID 返回 `unknown_preset`，绑定不一致返回 `preset_identity_mismatch`。未提供 ID 时保留原非预置发现路径。
- 目录身份待校验的预置不访问仓库、不生成工件候选，但仍调用 Node 获取本机硬件快照，并以 `preset_identity_pending` 返回不可用结果。
- 已验证预置实时发现有效 GGUF 后，将其与内置工件配置及该目录项最低 VRAM/RAM/磁盘要求取最大值，再交给 `ProductionNode` 基于本次 Node 实测硬件筛选。空的有效工件目录返回 `no_valid_gguf` 且候选为空；硬件不兼容沿用 Node 的 `no_compatible_variant`。
- 上游 `Unavailable` 时只尝试本地回退；回执须是成功记录、非空本地路径、正大小、GGUF 文件、有效 SHA-256、实际摘要等于期望摘要，且重建的来源/模型/修订/直接文件 URI 与记录完全一致。`NotFound` 不回退。回退清单同样应用内置和预置门槛。
- ModelSource 转为统一来源字符串供下载目录查询；测试覆盖旧请求、预置 ID 错误与身份串用、客户端 URL 不被采用、本地回执过滤、真实直接 URL/摘要、硬件门槛合并及 API 边界错误。

## 修改的文件与文件内位置

- `crates/interfaces/server/src/api/model_resolution.rs:8`：接入本地校验目录；第 47 行来源线协议映射；第 73 行解析请求及可选 `preset_id`；第 204 行解析 handler；第 280 行完整身份验证；第 316 行严格本地回退清单构造；第 350 行固定业务不可用响应；第 625 行本地记录状态、摘要、大小与 GGUF 校验。
- `crates/interfaces/server/src/model_selection/sources.rs:44`：将固定来源名称读取方法开放给服务端 crate 内解析/目录逻辑，避免序列化往返推断来源。
- `crates/interfaces/server/src/api/model_resolution_tests.rs:490`：旧版请求兼容、有效及未知预置 ID、模型/来源/修订串用和不信任客户端工件 URL；第 548 行严格本地回执、直接 URL 与预置硬件回归。
- `crates/interfaces/server/tests/model_resolution_api.rs:83`：路由黑盒验证未知预置及身份绑定错误在外部发现之前返回 HTTP 400。

## 检查结果

- `cargo fmt --all -- --check`、`git diff --check`：通过。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。
- `cargo test -p burncloud-server --lib`：107 项通过、2 项依赖本机 llama-server/GGUF 的测试忽略。
- `cargo test -p burncloud-server --test model_resolution_api`：5 项通过。
- `cargo test --workspace --lib`：通过；服务端库 107 项通过、2 项忽略。
- `cargo test --workspace`：所有已执行单元、集成测试通过，但并行 Node Runtime doctest 链接阶段因 Windows MSVC 临时 PDB `LNK1318: FILE_SYSTEM (3)` 失败；Node Runtime doctest 串行复验 `cargo test -p burncloud-node-runtime --doc -- --test-threads=1`：48 项通过。
