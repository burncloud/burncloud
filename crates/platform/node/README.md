# BurnCloud Node Runtime Skeleton

This crate owns only local machine/runtime orchestration primitives.

## Reconcile rail

```text
Absent
  -> Resolving
  -> PreparingArtifact
  -> ArtifactReady
  -> PreparingRuntime
  -> Starting
  -> WaitingReady
  -> Ready
  -> Routable
```

`Starting` means a process is being spawned. `WaitingReady` means a process exists but readiness has not been proven. `Ready` requires explicit readiness evidence. A fixed sleep is not readiness evidence.

## Boundary

The reconciler may request actions, but it does not own the business implementation of model resolution, artifact selection, Provider selection, or routing policy. Those contracts and decisions must remain with their owning BurnCloud domains.

The Node runtime must not create a second HTTP server, listener, or ModelRouter. `Routable` means attachment to the existing BurnCloud routing path.

## 真实硬件探测

`SystemHardwareProbe` 是 `HardwareProbe` 的系统实现，支持 Windows、Linux、macOS。
通过 `SystemHardwareProbe::new()` 注入 `NodeComposition::start()` 的第一个参数，
再调用 `node.hardware().inspect().await`。硬件 Fake 实现及导出已移除；
其它能力的测试替身仍用于验证各自生命周期。

```rust,no_run
use burncloud_node_runtime::{HardwareProbe, SystemHardwareProbe};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let hardware = SystemHardwareProbe::new().inspect().await?;
println!("CPU: {}, RAM: {}", hardware.cpu, hardware.memory_bytes);
# Ok(())
# }
```

返回值包含 CPU 型号、物理内存总量、当前工作目录所在文件系统可用空间及
加速器列表，容量单位为字节。GPU 不可用时保留基础信息，未知显存为 `None`。
检测失败不会回退到固定硬件配置。模型兼容性判断由 Supply resolver 负责；
server 通过完整 `HardwareProfile` 传递本次探测结果。
