# 001 实现 388 项预置目录与查询接口

## 修改的信息

- 保留完整 388 项预置清单，按 `id` 唯一校验；重复仓库继续按不同预置项保留。
- 在目录加载阶段校验来源、仓库地址主机与路径、默认修订号，以及三项最低值和三项推荐值；无效目录通过服务端 `preset_catalog_invalid` 响应失败关闭。
- 将 18 个精选项绑定为稳定 ID 与指定显示名，其余项用仓库名展示；修正 Cohere Command-R 绑定、Open-Orca/Mistral 标签及 OLMo 3.1 标签。
- 为无法确认语义身份的 5 项标记 `pending_validation`，目录保留供展示；后续状态与解析准入由执行顺序中的任务 003、004 接入并验证。
- 调整预置查询：未匹配 ID（包括格式异常的查询值）作为未知预置处理，不会被误报为目录内部故障。

## 修改的文件与文件内位置

- `crates/interfaces/server/src/model_selection/presets.rs:9`：388 项静态目录；`IdentityStatus`、`Preset` 与硬件结构；第 487 行身份待校验 ID；第 495 行精选项；第 526 行 `all`；第 546 行 `find`；第 550 行目录校验；第 595 行身份校验；第 631 行硬件校验。
- `crates/interfaces/server/src/model_selection/mod.rs:95`：完整目录查询错误响应；第 433 行状态查询对目录错误和未知 ID 的区分。
- `crates/interfaces/server/src/model_selection/sources.rs:24`：为目录复用调整来源类型与模型资源的 crate 内可见性和文档说明。
- `crates/interfaces/server/src/model_selection/tests.rs:151`：目录数量、ID、来源、修订号和硬件回归检查；第 265 行精选标签及待校验身份；第 398 行重复仓库身份；第 425 行无效输入不触发上游。

## 检查结果

- 388 项目录、唯一 ID、仓库身份/来源/修订号、六项硬件数据及重复仓库区分：通过 `presets_keep_spreadsheet_rows_links_and_hardware`。
- Command-R Cohere 绑定、Open-Orca/Mistral 标签与待校验项标记：通过 `preset_names_and_pending_identities_are_serialized`。
- 未知及格式异常预置查询均为客户端错误且不请求上游：通过 `invalid_inputs_never_reach_upstream`。
- 定向服务端模型选择测试：`cargo test -p burncloud-server --lib model_selection::tests -- --nocapture`，12 项通过。
- 工作区单元测试：`CARGO_BUILD_JOBS=1 cargo test --workspace --lib`，全部通过；2 项既有本机依赖测试忽略。
- `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`git diff --check`：通过。
- 默认并行运行 `CARGO_BUILD_JOBS=1 cargo test --workspace` 时，Node Runtime 文档测试在 MSVC 链接阶段遇到临时 PDB 文件系统错误（`LNK1318: FILE_SYSTEM (3)`）；随后改为串行执行 `cargo test -p burncloud-node-runtime --doc -- --test-threads=1`，48 项文档测试全部通过。工作区单元测试亦全部通过。
