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

## 补充复核：替代 GGUF 仓库唯一性（2026-11-23）

- 本次按 388 个预置项、367 个不同仓库身份复核。原仓库发现结果汇总为：113 个身份已有有效 GGUF、240 个未发现符合条件的 GGUF、14 个身份因上游请求失败未能核实。原仓库已有工件的条目不迁移；无工件但没有唯一、明确关联候选的条目，以及上游失败项均保留原身份。
- 初审发现四个预置存在多个符合关联与工件条件的候选；当时按“多候选暂不迁移”规则保留原 model/source/revision/resource_url、显示名称与硬件要求：

| 预置 ID | 明确关联且具有有效 GGUF 的候选仓库 | 结论 |
| --- | --- | --- |
| llama2-chinese-13b-chat | [s3nh/FlagAlpha-Llama2-Chinese-13b-Chat-GGUF](https://huggingface.co/s3nh/FlagAlpha-Llama2-Chinese-13b-Chat-GGUF) 的模型卡链接到 FlagAlpha 原模型，列有 7 个有效 GGUF；[mradermacher/Llama2-Chinese-13b-Chat-GGUF](https://huggingface.co/mradermacher/Llama2-Chinese-13b-Chat-GGUF) 的 base_model 同为 FlagAlpha，列有 11 个有效 GGUF。 | 多候选，保持 FlagAlpha/Llama2-Chinese-13b-Chat。 |
| dolphin-2_6-phi-2 | [TheBloke/dolphin-2_6-phi-2-GGUF](https://huggingface.co/TheBloke/dolphin-2_6-phi-2-GGUF) 明确关联原模型并列有 12 个有效 GGUF；[Demonthos/dolphin-2_6-phi-2-candle](https://huggingface.co/Demonthos/dolphin-2_6-phi-2-candle) 明确链接原模型，含 model-q4k.gguf（1,580,084,256 字节，SHA-256 f240cc36fa191a763e6fe854ba6c113c1dd77140c63aeca136bac9f169aff435）。 | 多候选，保持 cognitivecomputations/dolphin-2_6-phi-2。 |
| jev-9b | [prithivMLmods/JEV-9B-GGUF](https://www.modelscope.cn/models/prithivMLmods/JEV-9B-GGUF) 的 ModelScope BaseModel 为 autotrust/JEV-9B，文件清单有有效 GGUF；[VrKnight/JEV-9B-GGUF](https://huggingface.co/VrKnight/JEV-9B-GGUF) 的 Hugging Face base_model 包含同一原模型，文件清单有 3 个有效 GGUF。 | 多候选，保持 autotrust/JEV-9B。 |
| qwen3-8b | [Qwen/Qwen3-8B-GGUF](https://www.modelscope.cn/models/Qwen/Qwen3-8B-GGUF) 及 [unsloth/Qwen3-8B-GGUF](https://www.modelscope.cn/models/unsloth/Qwen3-8B-GGUF) 均关联 Qwen/Qwen3-8B 且有 GGUF 文件。 | 多候选，保持原 ModelScope 预置身份。 |

- 初审未添加运行时搜索或回退；初版回归测试按当时的多候选规则检查原身份保留。

- `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`CARGO_BUILD_JOBS=1 cargo test --workspace` 与 `git diff --check`：复核后全部通过。

## 用户指定规范 GGUF 仓库后的映射更新（2026-11-23）

- 用户指定 `Qwen/Qwen3-8B-GGUF` 为 Qwen3-8B 的规范仓库，并要求把已核实的其他对应预置也映射到可用 GGUF 仓库。对于非 Qwen 多候选项，选择按现有服务端验证规则可发现量化版本较完整的候选，以减少映射后可选工件不足；未发现可信对应仓库的预置不猜测、不改写。
- 更新映射：`qwen3-8b` → `Qwen/Qwen3-8B-GGUF`（用户指定）；`llama2-chinese-13b-chat` → `mradermacher/Llama2-Chinese-13b-Chat-GGUF`（11 个有效工件，高于另一候选的 7 个）；`dolphin-2_6-phi-2` → `TheBloke/dolphin-2_6-phi-2-GGUF`（12 个，高于另一候选的 1 个）；`jev-9b` → `prithivMLmods/JEV-9B-GGUF`（ModelScope 8 个，高于另一候选的 3 个）。本地服务以精确 `model/source/revision` 查询四个选定身份均返回 `downloadable=true`，分别发现 5/11/12/8 个工件，并生成直接 `/resolve/{revision}/{path}` 地址。
- 四项均保留原稳定 ID、目录中已有的显示名称和六项硬件要求；仅按所选仓库同步 `model`、`source`、`revision` 和 `resource_url`。Qwen 资源配置已定位到其选定仓库，其余三项无需改动内置资源配置。
- 本次仍不添加运行时自动搜索或回退。按此前 388 项扫描结果，另外 113 个唯一仓库身份原已含有效 GGUF；其余 236 个未找到有足够证据的对应仓库，14 个上游请求失败身份仍无法核实，全部保持不变。
- 对当前浏览器服务复核时，`/api/models/presets` 中三个非 Qwen 预置仍返回旧仓库身份，按旧 `preset_id` 查询仍报 `no_gguf_artifact`；对以上新仓库身份直接查询则已返回可下载状态。当前本地服务尚未加载工作区中的新静态映射；需重新构建并重启后端，再通过 `/api/models/status?preset=<id>` 验证界面绑定结果。
- 回归测试 `verified_gguf_presets_use_selected_repositories_and_preserve_catalog_metadata` 验证四项更新后的仓库身份、稳定 ID、原目录显示名称、来源、修订、资源页和六项硬件要求；`alternate_gguf_presets_discover_files_and_create_direct_downloads` 验证三个替代来源映射仍经实时工件发现并生成直接下载地址；既有 Qwen 状态测试覆盖 ModelScope 路径。
- 本轮检查：`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`CARGO_BUILD_JOBS=1 cargo test --workspace --tests --lib`、`CARGO_BUILD_JOBS=1 cargo test --workspace --doc -- --test-threads=1`、`git diff --check` 均通过。
