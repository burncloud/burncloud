# BurnCloud 本地修复汇总

本文简要记录截至目前在 Windows 本地调试、全量测试、性能检查和 Debian 适配过程中完成的修复。

## 1. 构建、依赖和发布

- 将根目录 `Cargo.lock` 纳入版本管理，构建、测试和发布使用 `--locked`，保证依赖可复现。
- Docker 构建工具链升级到 Rust 1.88，解决 Dioxus 和 `home` 等依赖的最低 Rust 版本不兼容问题。
- 发布流程不再按固定旧版本字符串批量修改 `Cargo.toml`，改为校验 Git tag 与根 `burncloud` 包版本是否一致。
- 修复 CLI 写死 `0.1.0` 的问题；`--version`、`--help` 现在正确输出并以状态码 0 退出。
- 自动更新器改为使用根应用版本，而不是自动更新子 crate 的版本。
- 新增 Linux x86_64 发布产物，并调整 Windows/Linux artifact 的汇总发布流程。

## 2. 安全和认证

- 为价格同步、熔断器、指标和内部健康检查等控制面接口增加 `BURNCLOUD_INTERNAL_SECRET` 保护。
- CLI 刷新价格缓存时自动携带 `x-internal-secret`，并覆盖密钥存在和空值场景。
- 修复 CLI 登录字段与服务端接口不一致的问题，并支持 `BURNCLOUD_SERVER_URL` 或 `PORT`。
- Unix 下保存的 CLI 凭据文件权限限制为 `0600`。
- API 集成测试改用真实注册/登录流程获取 JWT，内部接口测试统一携带内部密钥。
- 首个管理员注册竞态按要求暂不处理。

## 3. 数据库和迁移

- 修复 SQLite 新安装和旧版本升级时 BOOLEAN/INTEGER 列类型不一致的问题。
- 迁移重建过程增加事务，并可恢复中断迁移遗留的 `*_boolfix` 临时表数据。
- 修复 Windows 临时 SQLite 数据库 URL，统一生成 `sqlite:///C:/...` 形式的绝对路径。
- 更新推理和路由集成测试，适配当前 `channel_providers`、`channel_abilities` 等数据库结构和 API。

## 4. 路由、计费和稳定性

- 修复 Prometheus 指标名称重复添加 `burncloud` 命名空间的问题。
- 修正健康探测测试状态、并发探测保护及熔断器重复测试属性。
- 修复汇率反序列化和跨币种换算逻辑，补充反向汇率及 EUR/CNY 等覆盖。
- OpenAI 兼容请求可正确使用 provider adaptor，同时保留 Anthropic 原生路径限制和 Vertex 独立协议。
- 在转换为 OpenAI 格式前校验 provider 原生响应，并支持 Vertex 流式数组响应。
- 全部上游失败时保留响应质量失败原因，方便定位空响应、畸形响应和上游错误。
- 空响应计数器遇到 poisoned lock 时恢复运行并记录告警，避免后续请求持续 panic。
- 删除透传流式 token 计数热路径中的重复 `Arc::clone`。

## 5. 前端和性能

- 服务端启用响应压缩，LiveView HTML 实测由 164,093 字节降至 32,423 字节。
- 删除 Web 布局中重复加载的 LiveView 样式，减少重复资源和渲染开销。
- Playground 仅在用户明确触发后发送请求，避免状态变化导致重复提交。
- 前端 API 和 Playground 请求不再写死 3000 端口，统一读取 `PORT`。
- 反向代理为 HTTPS 时生成 `wss://` LiveView 地址，并在插值前校验 Host header。
- 修复 desktop/web 全 feature 同时启用时的入口冲突，提供独立 Web 启动入口。

## 6. Debian 和容器适配

- 新增 Debian 部署指南、Nginx 示例及加固后的 systemd service。
- Linux 服务启动时将自动生成的 `MASTER_KEY` 持久化到工作目录，避免尝试写入只读安装目录。
- Docker 改为从仓库根目录构建完整 workspace，并要求提供内部控制面密钥。
- 容器最终运行层使用 Debian Bookworm，包含 CA、OpenSSL 和健康检查依赖。

## 7. 验证结果

- 根程序测试：7 项通过。
- 自动更新 crate：3 项通过。
- 路由库测试：198 项通过。
- 数据库测试：88 项通过。
- API 集成测试：73 项执行通过，105 项显式忽略，0 失败。
- Workspace 全 target、全 feature 测试通过。
- Clippy 通过，仍保留少量既有 unused、dead-code 和 `expect` 告警。
- Release 构建通过，本地 `/health` 返回 `200 ok`。
- `git diff --check` 通过。

## 8. 已知剩余项

- 首个管理员注册竞态未修复，属于明确排除项。
- 健康探测调度器目前仍以周期调度框架为主，尚未实现完整的主动上游探测流程。
- 仓库仍有少量历史格式和 Clippy 告警，不影响当前编译和测试。

详细逐项记录见 `docs/PR_FIXES.md`。

## 9. 本轮新增修复

- 修复独立 Web 模式没有注入应用 CSS 的问题；LiveView 模式继续复用服务端
  已提供的样式，避免重复加载。
- 价格缓存同步请求现在优先使用 `BURNCLOUD_SERVER_URL`，并在未指定服务地址时
  读取 `PORT`；URL 和端口会做基本规范化。

## 10. 本轮验证

- 根 CLI 最新锁定测试：11 项通过。
- 纯 Web client 测试：4 项通过。
- LiveView client 锁定编译：通过。
- Router 库测试：198 项通过；Database 测试：88 项通过。
- Workspace 全量全 target/全 feature 重跑在 10 分钟编译超时，未产生测试失败；
  分层测试和 Clippy 均通过。
- 根 binary Clippy：通过，仅保留既有告警。
- `git diff --check`：通过。
