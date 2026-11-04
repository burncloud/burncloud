# 模型选择后通过生产 Node 入口执行真实硬件探测并用于 resolver 判断

## 背景与目标

- **背景**：HardwareProbe 已集成到 `burncloud-node-runtime`，公开 `HardwareProbe` 接口和 `SystemHardwareProbe` 实现能够提供本机硬件数据。但在当前代码中，server 的 Node 组装位于测试入口，使用 `FakeHardwareProbe`；生产启动路径尚未组装并触发真实硬件探测。用户已选择新建生产 Node 组装入口，由上一步模型选择完成后触发，并要求硬件检测相关参数不再使用 Fake。
- **现有缺口**：`NodeOrchestrator::prepare_until_ready()` 已调用 `hardware().inspect()`，随后仅向 `ModelResolutionRequest` 传递最大可报告加速器显存。`FakeModelResolver::resolve()` 忽略该硬件参数；`RealModelResolver::resolve()` 当前直接返回未实现错误。已有 `RealModelResolver::filter_by_hardware()` 能按 RAM/VRAM 过滤模型变体，但未接入实际 `resolve()` 调用链。因此仅替换探测器无法完成本次目标。
- **目标**：形成“上一步完成模型选择并提交所选模型 → 生产 Node 入口 → 真实硬件探测 → 真实硬件数据传入 resolver → 参与相关判断并影响结果”的服务运行时调用链。
- **上游接口状态**：用户已确认“模型选择”的接口目前尚未明确。当前确认的是业务触发时机，不是某个已有页面、URL、请求字段或提交函数；具体对接契约需在上游接口确定后补充。
- **开发序号**：`2`。目标工作区为 `E:\项目\test\burncloud`。本报告为待实现需求，不表示生产链路已经完成。

## 需求列表

| # | 需求 | 是否必需 |
| --- | --- | --- |
| 1 | 新建生产 Node 组装入口，并在上一步模型选择后触发硬件检测 | 是 |
| 2 | 硬件检测及其相关参数使用真实数据，移除硬件 Fake 注入 | 是 |
| 3 | 将真实硬件数据传入 resolver 并用于实际硬件相关判断 | 是 |
| 4 | 通过真实服务调用链验证数据来源、传递和判断结果 | 是 |

## 每项需求的期望行为与期望结果

| # | 需求 | 预期行为 | 期望结果 |
| --- | --- | --- | --- |
| 1 | 生产组装与触发 | 服务创建生产 Node 组装入口，注入 `SystemHardwareProbe`；上一步模型选择完成并提交所选模型后，经该入口触发硬件检测，并在 resolver 的硬件相关判断前取得结果 | 能从模型选择提交位置追踪到生产 Node、`HardwareProbe::inspect()` 和 resolver 的连续调用链；在未开启 `BURNCLOUD_NODE_TEST_API` 时，该生产链路仍可触发，不依赖 `/test/node/*` |
| 2 | 移除硬件 Fake | 硬件探测以及传入 resolver 的硬件参数来自当前服务进程所在主机的真实检测；现有服务入口、相关单元测试与集成测试中的硬件 Fake 注入一并调整，不再以固定硬件参数代替实际探测 | 移除 `FakeHardwareProbe` 的定义、公开导出及相关引用，并检查其他名称的伪硬件探测器；相关探测调用与硬件判断验收使用真实检测值，检测失败时不得回退为虚构 CPU、内存或显存数据 |
| 3 | resolver 使用真实数据 | 将真实探测所得、硬件判断实际需要的数据传入 resolver，并在真实解析路径中调用相应判断；接通已有 RAM/VRAM 兼容性过滤所需数据，解决当前请求只携带显存值而 RAM 过滤需要内存数据的缺口；实际判断不得由忽略硬件参数的 Fake resolver 代替 | 能证明参与比较的可用资源值来自本次真实探测；满足与不满足硬件条件时，候选保留、排除或不支持结果与相应规则一致；不能以仅收到参数、打印日志或返回硬件 JSON 作为完成依据 |
| 4 | 核心验收 | 从服务的模型选择后续入口执行真实链路，关联所选模型、探测结果、resolver 入参、实际判断条件和判断结果；使用真实硬件资源值与不同模型资源要求验证判定 | 核心验收标准为“真实数据传入 resolver 并被相关判断使用”。对于同一实测资源值，构造满足与超过资源条件的模型候选，验证真实判断分别保留与排除；该验证改变模型要求，不伪造主机硬件参数。保留可复核的调用及结果证据 |

## 范围边界

- **做什么**：在现有 server 内组装和触发真实硬件探测，接通与模型 resolver 相关的数据契约和实际判断，调整硬件 Fake 的定义、导出、调用、相关测试及受影响文档。
- **平台范围**：沿用既有 Windows、Linux、macOS 探测能力和公开数据类型；保留 GPU 探测不可用时的既有空列表或部分结果语义，真实未知值不得用虚构设备容量替代。
- **职责边界**：硬件事实采集仍归 `crates/platform/node`，模型兼容性判断仍归 `crates/supply/model`，入口组装和调用衔接归 `crates/interfaces/server`。本次不重新设计模型选择界面，不扩展为模型下载、运行时启动、进程管理及健康探测等所有能力的全面实现；这些后续能力不属于本次核心验收。
- **上游依赖与验收边界**：模型选择接口待定期间，可推进生产 Node 组装、真实硬件探测及 resolver 数据使用的设计与验证；不能据此宣称“模型选择 → Node → resolver”的完整服务链路已经接通。上游接口明确后，再补齐触发调用和端到端验收。本次不擅自新增模型选择 API，也不将测试接口替代为生产触发点。
- **代码定位**：以下位置依据本地提交 `d34cd9c830749c3afdcc9068e8779386c4ad84e7` 及当前工作区核对。行号是定位参考；新增入口的文件名及最终修改集合由后续开发文档确定。

| 路径 / 当前定位 | 与本需求的关系及预期调整 |
| --- | --- |
| `crates/interfaces/server/src/lib.rs:104` | `create_app_with_node_request_state()`：接入生产 Node 组装与所需服务状态；不把测试路由挂载作为生产入口 |
| `crates/interfaces/server/src/` 下的生产 Node 组装模块 | 新增生产组装入口，注入真实硬件探测器并承接模型选择提交；具体文件名待开发设计确定 |
| 上一步模型选择的提交文件 / 函数（上游接口尚未明确） | 业务要求是在已选模型提交后调用生产 Node 入口；待上游接口确定后补充具体路径和对接契约，目前不指定页面、URL 或函数 |
| `crates/interfaces/server/src/node_orchestrator.rs:295` | 真实探测的调用点；第 302 行起的数据提取、第 309 行起的 resolver 请求需要满足实际硬件判断的数据需求；同文件测试中的硬件 Fake 注入同步调整 |
| `crates/supply/model/src/resolver.rs:5` | `ModelResolutionRequest` 当前只有模型名和可选显存，需要承载硬件判断所需的真实数据；第 62 行起的 Fake resolver 不能承担本次生产判断 |
| `crates/supply/model/src/real_model_resolver.rs:46` | 复用既有硬件过滤规则，并将其接入第 81 行起尚未实现的 `resolve()` 路径；该路径必须实际使用传入的硬件数据 |
| `crates/platform/node/src/fake.rs:15`、`crates/platform/node/src/lib.rs:26` | 移除硬件 Fake 实现及其导出，处理所有受影响调用；其他能力不因同处一个文件而被纳入全面替换 |
| `crates/interfaces/server/src/node_test.rs:20`、`:35`、`:76` | 调整硬件类型导入、类型参数和注入；该测试入口的调整不能代替新增生产入口 |
| `crates/interfaces/server/src/node_attachment.rs` 及其测试 | 对接现有准备调用的位置为第 94 行；第 205 行起的测试含硬件 Fake，需随相关契约调整 |
| `crates/interfaces/server/tests/node_framework_e2e.rs`、`node_route_visibility_lifecycle.rs`、`node_artifact_gate_001.rs` | 调整硬件 Fake 注入及受其固定数据影响的断言；补充生产入口到 resolver 的实际数据使用证据 |
| `crates/platform/node/tests/composition.rs`、`golden_path.rs`、`fakes.rs`、`contracts.rs` | 调整硬件 Fake 注入；`contracts.rs` 中还存在名为 `FakeHardware` 的自定义实现，不能仅搜索替换 `FakeHardwareProbe` |
| `crates/platform/node/tests/hardware_probe.rs`、`crates/platform/node/src/probe/tests/` | 复用和补充真实探测验证，记录实际可用设备和未知容量；不把单独探测测试通过等同于生产链路验收通过 |
| `crates/supply/model/tests/resolver.rs`、`crates/supply/model/src/real_model_resolver.rs` 的测试部分 | 调整相关硬件参数来源，验证真实输入被兼容性判断使用 |
| `crates/platform/node/README.md`、`crates/supply/model/README.md` 及受影响公开接口注释 | 同步真实调用方式、数据传递与判断职责说明 |

## 待确认问题

- [ ] 上游依赖待明确：模型选择接口当前尚未定义，后续需补充其调用位置、提交数据及与生产 Node 入口的对接契约。无需当前提供一个不存在的接口；在该依赖落实前，完整服务触发链路的验收保持未完成。业务顺序“模型选择完成 → 真实硬件探测 → resolver 判断”已确认。
