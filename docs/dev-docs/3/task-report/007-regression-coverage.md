# 007 覆盖部署与竞态回归：任务报告

## 修改的信息

- 补充断连、旧成功与旧失败、重复验证回调、凭据不一致以及取消后重启的回归检查。
- 新增独立页面任务测试，控制旧部署 future 在新运行句柄登记后实际返回，覆盖成功和失败的迟到收尾；生产页面只增加测试模块声明，未新增生产函数。
- 结合前序任务已提交的 HTTP、后端、状态与 UI 测试覆盖部署协议和取消行为。

## 修改的文件与文件内位置

- `crates/interfaces/client/src/domains/chat/catalog_tests.rs`：部署断连回归。
- `crates/interfaces/client/src/domains/chat/tests.rs`：旧部署结果不影响新运行及重复验证事件回归。
- `crates/interfaces/client/src/domains/chat/ui_tests.rs`：凭据不一致不部署、取消后重启保留新任务的 UI 回归。
- `crates/interfaces/client/src/domains/chat/page.rs`：条件编译的独立测试模块声明。
- `crates/interfaces/client/src/domains/chat/page_task_tests.rs`：`stale_task_cleanup_cannot_clear_new_run_handle` 可控竞态测试。

## 检查结果

- 与任务 002 的测试合计覆盖 ready、unhealthy、failed、400、503、其他非 200、断连、超时、坏 JSON、字段缺失、未知和中间状态；核对错误类型和阶段。
- 原始路径、凭据不匹配不发部署、重复验证不重复认领、ArtifactReady 可取消且不提前就绪：状态及组件测试通过。
- 旧运行成功和失败不会改变新运行、就绪集合和草稿；独立任务测试让旧 future 在新句柄安装后实际完成，验证成功与失败两条路径均不清除新句柄或更改新状态。
- 部署取消不发送下载取消；非目录模型和下载取消失败由前序回归与完整客户端测试覆盖。
- 格式、clippy 和测试按项目约定在提交前后执行；测试规划与独立测试执行留待后续阶段。
