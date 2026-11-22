# 006 实现精选目录、显示更多和安全确认交互

## 修改的信息

- 空查询按 18 个稳定 `preset_id` 的指定顺序呈现精选模型；展开后追加其余目录项并按 ID 去重，收起后回到精选项。非空搜索不受精选项数量限制。
- 选择器状态同步记录身份校验、工件可下载状态与不可用原因；待校验身份、状态查询未完成/失败、无有效远程候选且无精确已验证本地凭据时，确认入口不予准入。
- 预览变化和列表刷新会取消旧状态任务；列表及状态请求通过序号和预览身份检查丢弃迟到响应。界面显示待校验、状态加载和不可用原因。
- 更新 UI 测试目录桩以使用真实精选 ID，并覆盖不可下载状态的确认拒绝；修正 Dioxus 路由派生宏在 Clippy 检查中的不可达分支告警。

## 修改的文件与文件内位置

| 文件 | 位置 | 修改 |
| --- | --- | --- |
| `crates/interfaces/client/src/domains/chat/selector.rs` | `FEATURED_PRESET_IDS`、`visible_models` | 配置精选顺序；实现空查询精选/展开派生列表、目录项去重和搜索不截断。 |
| `crates/interfaces/client/src/domains/chat/selector.rs` | `PickerState::{clear_preview, begin_list, begin_status, finish_status}`、`can_confirm_selection` | 取消旧任务、忽略陈旧响应、保存下载状态/身份/不可用原因，并按有效候选及身份准入确认。 |
| `crates/interfaces/client/src/domains/chat/selector.rs` | `CatalogPicker`、`confirm_selection` | 展示精选目录、显示更多/收起、状态提示和安全确认入口；通过原 Chat 状态机交接选择。 |
| `crates/interfaces/client/src/domains/chat/selector_tests.rs` | 精选目录、搜索、确认和状态测试 | 验证精选顺序、展开后 388 项唯一性、搜索完整呈现、无凭据/待校验/加载状态拒绝确认及不可用原因保存。 |
| `crates/interfaces/client/src/domains/chat/ui_tests.rs` | `CatalogServer`、目录 UI 测试与预览辅助函数 | 使用精选 ID；按新确认入口交互；验证无有效 GGUF 时不显示确认操作，并清理二次弹层测试中的旧按钮快照。 |
| `crates/interfaces/client/src/app/router/routes.rs` | `Route` 派生属性 | 对 Dioxus 派生宏产生的不可达分支添加有原因的局部 Clippy 抑制。 |

## 检查结果

- 精选顺序、388 项展开数量与唯一性、收起、搜索不截断、身份/状态/凭据准入测试：通过。
- `cargo fmt --all`：通过。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。
- `cargo test -p burncloud-client --lib -- --test-threads=1`：通过，236 项测试通过。
- `CARGO_BUILD_JOBS=1 cargo test --workspace`：所有常规单元、集成及已运行文档测试通过；Node Runtime 的 29 个并行 doctest 因 Windows 页面文件不足导致链接/元数据失败。随后 `cargo test -p burncloud-node-runtime --doc -- --test-threads=1` 串行复验通过，48 项 doctest 全部通过。初次并行运行 `cargo test --workspace` 也因页面文件不足在链接阶段中止。
- `git diff --check`：通过。