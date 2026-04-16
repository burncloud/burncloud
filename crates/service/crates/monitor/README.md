# burncloud-service-monitor

跨平台系统监控。采集 CPU、内存、磁盘指标,支持定时自动采集。

## 关键类型

| 类型 | 说明 |
|------|------|
| `SystemMonitor` | 监控核心,采集系统指标 |
| `SystemMonitorService` | 服务包装(含自动采集) |
| `SystemMetrics` | 聚合指标(CPU + 内存 + 磁盘) |
| `CpuInfo` / `MemoryInfo` / `DiskInfo` | 各维度详细指标 |
| `MonitorError` | 监控服务错误 |

## 依赖

- `tokio`, `serde`, `thiserror`, `async-trait`
- 平台特定:`winapi`(Windows), `libc`(Unix)
