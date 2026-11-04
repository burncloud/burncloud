# 开发序号 2：真实硬件探测与模型解析实现报告

- 日期：2026-11-04。
- 需求依据：`E:/项目/test/burncloud/docs/dev-docs/2/burncloud-1-issue.md`。
- 工作区：`E:/项目/test/burncloud`。
- 状态：生产 Node 内部探测和解析链已实现、已在 Windows 主机验证；模型选择的上游触发接口尚未定义，完整服务触发链路仍待对接和验收。

## 修改的信息

1. 新增公开的 `ProductionNode`，由服务 `AppState.production_node` 持有，使用 `SystemHardwareProbe` 和 `RealModelResolver`。组装时不探测，调用 `resolve_selected_model()` 时才执行真实探测；生产入口不依赖 `BURNCLOUD_NODE_TEST_API`。
2. `ModelResolutionRequest` 改为携带完整 `HardwareProfile`，取代仅传可选显存的 `accelerator_memory_bytes`。既有 `NodeOrchestrator` 也传入本次探测的完整快照。
3. 实现 `RealModelResolver::resolve()`：校验模型名、定位清单、按 RAM 和最大已知单设备显存过滤、按清单声明的后端过滤，返回清单顺序中的首个兼容变体。没有兼容变体返回 `Unsupported(NoCompatibleVariant)`，空模型名或未知模型返回错误。
4. 移除 `FakeHardwareProbe`、公开导出和所有注入，并移除测试里的自定义 `FakeHardware`。相关测试改用真实探测器；其他能力的测试替身保留。硬件解析器与序列化测试中的数据样本不属于运行时 Fake 注入。
5. 同步三个 crate 的 README，新增生产入口和真实 resolver 的独立集成测试及文档示例。

RAM 使用探测器报告的物理内存总量，显存使用最大已知单设备容量；不表示当前空闲容量，不累加多设备显存。GPU 探测不可用时保留既有空列表或部分结果，未知容量仍为 `None`。没有已知显存时只接受显存要求为零的候选，不生成虚构设备或容量。后端过滤仅核对清单声明，不证明本机已安装对应运行时。

## 修改的文件与文件内位置

以下路径均位于本次目标工作区；函数名用于定位随编辑变化的行号。

| 文件 | 位置与修改 |
| --- | --- |
| `E:/项目/test/burncloud/crates/interfaces/server/src/node_production.rs:26` | 新增 `ProductionNode`；第 49 行 `resolve_selected_model()` 校验所选模型并执行真实探测和解析 |
| `E:/项目/test/burncloud/crates/interfaces/server/src/lib.rs:76` | 新增 `AppState.production_node`，第 136 行在 `create_app_with_node_request_state()` 中组装 |
| `E:/项目/test/burncloud/crates/interfaces/server/src/node_orchestrator.rs:295` | `prepare_until_ready()` 获取真实快照，第 304 行将快照交给 resolver；同文件既有测试改用系统探测器 |
| `E:/项目/test/burncloud/crates/supply/model/src/resolver.rs:8` | `ModelResolutionRequest.hardware` 保存完整硬件快照 |
| `E:/项目/test/burncloud/crates/supply/model/src/real_model_resolver.rs:95` | `resolve_model()` 将原有硬件、后端过滤接入真实解析；第 131 行实现 `resolve()`；测试迁出至独立文件 |
| `E:/项目/test/burncloud/crates/platform/node/src/fake.rs` | 删除 `FakeHardwareProbe` 及其实现 |
| `E:/项目/test/burncloud/crates/platform/node/src/lib.rs` | 删除硬件 Fake 的公开导出 |
| `E:/项目/test/burncloud/crates/platform/node/src/probe/windows.rs` | 为 `SystemHardwareProbe` 派生 `Debug`，供既有组装类型使用 |
| `E:/项目/test/burncloud/crates/platform/node/src/probe/linux.rs` | 同上 |
| `E:/项目/test/burncloud/crates/platform/node/src/probe/macos.rs` | 同上 |
| `E:/项目/test/burncloud/crates/platform/node/src/probe/unsupported.rs` | 同上 |
| `E:/项目/test/burncloud/crates/interfaces/server/src/node_test.rs` | 测试入口的硬件类型和注入改为 `SystemHardwareProbe` |
| `E:/项目/test/burncloud/crates/interfaces/server/src/node_attachment.rs` | 既有测试中的硬件注入改为系统探测器 |
| `E:/项目/test/burncloud/crates/interfaces/server/tests/node_production.rs` | 新增 6 项生产入口与编排器到真实 resolver 的测试 |
| `E:/项目/test/burncloud/crates/interfaces/server/tests/node_artifact_gate_001.rs` | 更新硬件注入 |
| `E:/项目/test/burncloud/crates/interfaces/server/tests/node_framework_e2e.rs` | 更新硬件注入 |
| `E:/项目/test/burncloud/crates/interfaces/server/tests/node_route_visibility_lifecycle.rs` | 更新硬件注入 |
| `E:/项目/test/burncloud/crates/platform/node/tests/composition.rs` | 使用真实探测器，以真实 CPU 和内存断言替代固定设备数量 |
| `E:/项目/test/burncloud/crates/platform/node/tests/contracts.rs` | 删除自定义 `FakeHardware`，验证公开 trait 的真实实现 |
| `E:/项目/test/burncloud/crates/platform/node/tests/fakes.rs` | 删除已移除的硬件 Fake 测试 |
| `E:/项目/test/burncloud/crates/platform/node/tests/golden_path.rs` | 使用真实探测器，向 resolver 传递完整快照 |
| `E:/项目/test/burncloud/crates/supply/model/tests/resolver.rs` | 硬件入参改由真实探测生成 |
| `E:/项目/test/burncloud/crates/supply/model/tests/real_model_resolver.rs` | 新增 6 项清单顺序、硬件边界、后端过滤和真实解析测试 |
| `E:/项目/test/burncloud/crates/interfaces/server/README.md` | 公开生产入口调用示例及上游接口待定说明 |
| `E:/项目/test/burncloud/crates/platform/node/README.md` | 真实探测调用方式、容量语义及 Fake 移除说明 |
| `E:/项目/test/burncloud/crates/supply/model/README.md` | 更新 resolver 请求契约、算法和结果语义 |

## 调用路径

```text
服务创建：create_app_with_node_request_state()
  -> AppState.production_node = Arc::new(ProductionNode::new())

待上游补齐：模型选择提交位置
  -> state.production_node.resolve_selected_model(model, manifests).await
     -> ModelDemand::new(model)
     -> SystemHardwareProbe::inspect().await
     -> ModelResolutionRequest { model, hardware }
     -> RealModelResolver::resolve()
        -> resolve_model()
        -> filter_by_hardware()：比较真实 RAM / 显存与模型要求
        -> filter_by_runtime()：核对清单声明的后端
        -> Local(ResolvedModel) 或 Unsupported(NoCompatibleVariant)
```

调用方传入已加载的 `Vec<ModelManifest>`；本次没有新增清单加载器、模型选择 HTTP API、下载或进程启动能力。既有编排器的另一条路径为 `prepare_until_ready()` -> `machine.hardware().inspect()` -> `resolver.resolve(ModelResolutionRequest { model, hardware })`。

## 检查结果

| 需求 | 核对结果 |
| --- | --- |
| 1. 生产组装与模型选择触发 | 生产对象已在服务创建时组装，公开调用入口已验证。上游模型选择接口尚未定义，具体触发调用未接入 |
| 2. 移除硬件 Fake | 已完成；全仓库 Rust 源码未发现 `FakeHardwareProbe`、自定义 `FakeHardware` 或旧字段 `accelerator_memory_bytes` |
| 3. 真实数据被 resolver 判断使用 | 已完成；真实探测快照进入实际 RAM/VRAM 比较，候选保留或排除会影响解析结果 |
| 4. 服务调用链验收 | 生产入口和既有编排器到真实 resolver 的内部调用已验证；从上游提交接口开始的完整服务端到端验收尚未完成 |

### 本机实测与判定证据

运行环境为 Windows。生产入口测试在调用前获取真实基准快照，入口内部再次执行真实探测；独立 resolver 测试直接将同一份实测快照用于所有边界比较。

| 实测项 | 值 |
| --- | --- |
| CPU | `13th Gen Intel(R) Core(TM) i9-13980HX` |
| RAM | `33959829504` 字节 |
| NVIDIA GPU | `NVIDIA GeForce RTX 4080 Laptop GPU`，显存 `12878610432` 字节 |
| Intel GPU | `Intel(R) UHD Graphics`，报告容量 `1073741824` 字节 |
| resolver 使用的最大单设备显存 | `12878610432` 字节 |

测试只改变模型要求，不伪造主机硬件：RAM 要求 `33959829505` 或显存要求 `12878610433` 的候选被排除；RAM/显存要求等于实测边界的候选被保留。生产入口返回 `Local`，选中 `https://example.invalid/compatible.gguf`；该 URI 仅标识测试候选，不执行下载。只有超限候选时返回 `Unsupported(NoCompatibleVariant)`；既有编排器进入 `LocalUnsupported`，没有生成进程计划或路由挂载记录。

### 执行命令与结果

```powershell
cargo test -p burncloud-node-runtime -p burncloud-supply-model -p burncloud-server --lib --tests --no-run
cargo test -p burncloud-node-runtime --lib --test hardware_probe --test contracts --test composition --test fakes --test golden_path
cargo test -p burncloud-supply-model --lib --test resolver --test real_model_resolver
cargo test -p burncloud-server --lib --test node_production --test node_framework_e2e --test node_route_visibility_lifecycle --test node_artifact_gate_001 -- --nocapture
cargo test -p burncloud-supply-model -p burncloud-server --test real_model_resolver --test node_production -- --nocapture
cargo clippy -p burncloud-node-runtime -p burncloud-supply-model -p burncloud-server --all-targets -- -D warnings
cargo test -p burncloud-node-runtime -p burncloud-supply-model -p burncloud-server --doc
git diff --check
```

- 三个目标包的全部 lib / integration test 目标编译通过。
- 实际执行 Node 39 项、Supply Model 9 项、Server 54 项测试，共 102 项，全部通过。新增测试调整后再次运行其中 12 项，全部通过，不重复计入总数。
- 文档测试 29 项通过，其中 Node 26 项、Server 2 项、Supply Model 1 项。
- Clippy 在 `-D warnings` 下通过。
- 对本次修改的 Rust 文件执行 `rustfmt --check --edition 2021 --config skip_children=true`，通过；`git diff --check` 通过。
- 本次只在 Windows 实机运行测试，未运行 Linux/macOS 实机测试，也未完成上游模型选择 HTTP 链路验证。
- 用户预先修改的 `E:/项目/test/burncloud/crates/platform/node/src/probe/tests/windows.rs` 未改动、未格式化。修改前后 SHA256 均为 `D77FA6DED6F24877E7E2A45E0156F01EEF09A2AC497FEE7CD3C3BD4FB54A69E3`。
