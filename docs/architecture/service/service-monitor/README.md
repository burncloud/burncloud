# service-monitor

系统监控服务，收集 CPU、内存、磁盘等系统指标。

## 🧅 第一层：在 Service 中的位置

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          burncloud-service                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌────────────────────────────────────────────────────────────────────┐    │
│   │                        服务层                                       │    │
│   │                                                                     │    │
│   │   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐    │    │
│   │   │★service-monitor │  │ service-user    │  │ service-models  │    │    │
│   │   │                 │  │                 │  │                 │    │    │
│   │   │ CPU 监控        │  │ 注册登录        │  │ 模型管理        │    │    │
│   │   │ 内存监控        │  │ JWT Token       │  │ HF 集成         │    │    │
│   │   │ 磁盘监控        │  │ 密码哈希        │  │ 文件下载        │    │    │
│   │   │ 系统指标        │  │                 │  │                 │    │    │
│   │   └─────────────────┘  └─────────────────┘  └─────────────────┘    │    │
│   │                                                                     │    │
│   └────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘

service-monitor 的职责:
├── CPU 使用率收集
├── 内存使用情况收集
├── 磁盘空间监控
├── 系统指标聚合
└── 跨平台支持 (Windows/Linux)
```

## 🧅 第二层：文件结构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       service-monitor 文件结构                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  crates/service/crates/service-monitor/                                      │
│  │                                                                           │
│  ├── Cargo.toml           # 依赖配置                                        │
│  └── src/                                                                    │
│      ├── lib.rs           # 模块入口，重导出公共 API                        │
│      ├── service.rs       # SystemMonitorService 实现                       │
│      ├── types.rs         # 数据类型定义                                    │
│      └── collectors/      # 数据收集器                                      │
│          ├── mod.rs       # 收集器模块入口                                  │
│          ├── cpu.rs       # CPU 收集器                                      │
│          ├── memory.rs    # 内存收集器                                      │
│          └── disk.rs      # 磁盘收集器                                      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 🧅 第三层：lib.rs 详解

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              lib.rs                                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  模块职责: 导出公共 API                                                      │
│                                                                              │
│  pub mod collectors;           // 收集器模块                                 │
│  pub mod service;              // 服务模块                                   │
│  pub mod types;                // 类型模块                                   │
│                                                                              │
│  // 重导出主要公共 API                                                       │
│  pub use types::{CpuInfo, DiskInfo, MemoryInfo, MonitorError, SystemMetrics};│
│  pub use service::{SystemMonitor, SystemMonitorService};                     │
│  pub use collectors::{CpuCollector, MemoryCollector, DiskCollector};         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 🧅 第四层：types.rs 数据类型

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          types.rs 数据类型                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  /// 监控错误类型                                                            │
│  pub enum MonitorError {                                                     │
│      CollectionFailed(String),    // 收集失败                               │
│      NotAvailable,                // 数据不可用                             │
│      InvalidData(String),         // 数据无效                               │
│  }                                                                           │
│                                                                              │
│  ─────────────────────────────────────────────────────────────────────────── │
│                                                                              │
│  /// CPU 信息                                                                │
│  pub struct CpuInfo {                                                        │
│      pub usage_percent: f32,      // CPU 使用率 (0-100)                     │
│      pub core_count: usize,       // CPU 核心数                             │
│      pub frequency: u64,          // CPU 频率 (MHz)                         │
│      pub brand: String,           // CPU 品牌 (如 "Intel Core i7")          │
│  }                                                                           │
│                                                                              │
│  ─────────────────────────────────────────────────────────────────────────── │
│                                                                              │
│  /// 内存信息                                                                │
│  pub struct MemoryInfo {                                                     │
│      pub total: u64,              // 总内存 (字节)                           │
│      pub used: u64,               // 已使用 (字节)                           │
│      pub available: u64,          // 可用 (字节)                             │
│      pub usage_percent: f32,      // 使用率 (0-100)                         │
│  }                                                                           │
│                                                                              │
│  // 辅助方法: format_size() 将字节转换为可读格式 (GB, MB 等)                 │
│                                                                              │
│  ─────────────────────────────────────────────────────────────────────────── │
│                                                                              │
│  /// 磁盘信息                                                                │
│  pub struct DiskInfo {                                                       │
│      pub total: u64,              // 总空间 (字节)                           │
│      pub used: u64,               // 已使用 (字节)                           │
│      pub available: u64,          // 可用 (字节)                             │
│      pub usage_percent: f32,      // 使用率 (0-100)                         │
│      pub mount_point: String,     // 挂载点 (如 "/" 或 "C:\")               │
│  }                                                                           │
│                                                                              │
│  ─────────────────────────────────────────────────────────────────────────── │
│                                                                              │
│  /// 系统综合指标                                                            │
│  pub struct SystemMetrics {                                                  │
│      pub cpu: CpuInfo,            // CPU 信息                                │
│      pub memory: MemoryInfo,      // 内存信息                                │
│      pub disks: Vec<DiskInfo>,    // 磁盘列表                                │
│      pub timestamp: u64,          // 采集时间戳                              │
│  }                                                                           │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 🧅 第五层：service.rs 服务实现

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          service.rs 服务实现                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  /// 系统监控服务                                                            │
│  pub struct SystemMonitorService {                                           │
│      cpu_collector: Arc<Mutex<CpuCollector>>,     // CPU 收集器             │
│      memory_collector: Arc<MemoryCollector>,      // 内存收集器             │
│      disk_collector: Arc<DiskCollector>,          // 磁盘收集器             │
│      cached_metrics: Arc<RwLock<Option<SystemMetrics>>>,  // 指标缓存       │
│      update_interval: Duration,                   // 更新间隔               │
│  }                                                                           │
│                                                                              │
│  核心方法:                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │  /// 创建新的监控服务                                                │   │
│  │  pub fn new() -> Self                                                │   │
│  │                                                                      │   │
│  │  /// 设置更新间隔                                                    │   │
│  │  pub fn with_update_interval(self, interval: Duration) -> Self      │   │
│  │                                                                      │   │
│  │  /// 获取当前系统指标 (带缓存)                                       │   │
│  │  pub async fn get_metrics(&self) -> Result<SystemMetrics>            │   │
│  │  // 如果缓存数据较新，直接返回缓存；否则收集新数据                    │   │
│  │                                                                      │   │
│  │  /// 强制刷新并获取最新指标                                          │   │
│  │  pub async fn refresh_metrics(&self) -> Result<SystemMetrics>        │   │
│  │                                                                      │   │
│  │  /// 启动自动更新后台任务                                            │   │
│  │  pub async fn start_auto_update(&self) -> Result<()>                 │   │
│  │  // 在后台定时收集指标，更新缓存                                     │   │
│  │                                                                      │   │
│  │  /// 获取 CPU 使用率                                                 │   │
│  │  pub async fn get_cpu_usage(&self) -> Result<f32>                    │   │
│  │                                                                      │   │
│  │  /// 获取内存信息                                                    │   │
│  │  pub async fn get_memory_info(&self) -> Result<MemoryInfo>           │   │
│  │                                                                      │   │
│  │  /// 获取磁盘信息                                                    │   │
│  │  pub async fn get_disk_info(&self) -> Result<Vec<DiskInfo>>          │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  /// 系统监控 Trait (统一接口)                                               │
│  pub trait SystemMonitor {                                                   │
│      async fn get_cpu_usage(&self) -> Result<f32>;                          │
│      async fn get_memory_info(&self) -> Result<MemoryInfo>;                 │
│      async fn get_system_metrics(&self) -> Result<SystemMetrics>;           │
│  }                                                                           │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 🧅 第六层：collectors 收集器

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          collectors 收集器                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  cpu.rs - CPU 收集器                                                        │
│  ────────────────────                                                        │
│  pub struct CpuCollector {                                                   │
│      update_interval: Duration,                                              │
│      last_cpu_times: Option<CpuTimes>,    // 上次采样数据 (计算使用率)      │
│  }                                                                           │
│                                                                              │
│  跨平台实现:                                                                 │
│  ├── Windows: 使用 winapi 读取注册表获取 CPU 信息                           │
│  └── Unix: 读取 /proc/stat 和 /proc/cpuinfo                                 │
│                                                                              │
│  ─────────────────────────────────────────────────────────────────────────── │
│                                                                              │
│  memory.rs - 内存收集器                                                     │
│  ────────────────────                                                        │
│  pub struct MemoryCollector;                                                 │
│                                                                              │
│  跨平台实现:                                                                 │
│  ├── Windows: 使用 GlobalMemoryStatusEx API                                 │
│  └── Unix: 读取 /proc/meminfo                                               │
│                                                                              │
│  扩展功能: get_detailed_info() 返回包含 swap 信息的详细内存报告              │
│                                                                              │
│  ─────────────────────────────────────────────────────────────────────────── │
│                                                                              │
│  disk.rs - 磁盘收集器                                                       │
│  ────────────────────                                                        │
│  pub struct DiskCollector;                                                   │
│                                                                              │
│  跨平台实现:                                                                 │
│  ├── Windows: 使用 GetDiskFreeSpaceExW API                                  │
│  └── Unix: 读取 /proc/mounts + statvfs 系统调用                             │
│                                                                              │
│  过滤逻辑:                                                                   │
│  ├── Unix: 只处理本地文件系统 (ext4, xfs, btrfs 等)                         │
│  └── 排除特殊挂载点 (/proc, /sys, /dev, /run)                               │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 🧅 第七层：数据采集流程

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          数据采集流程                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                      SystemMonitorService                              │  │
│  │                                                                        │  │
│  │   get_metrics() 请求                                                   │  │
│  │         │                                                              │  │
│  │         ▼                                                              │  │
│  │   ┌─────────────────┐                                                  │  │
│  │   │ 检查缓存有效性  │                                                  │  │
│  │   └────────┬────────┘                                                  │  │
│  │            │                                                            │  │
│  │     ┌──────┴──────┐                                                    │  │
│  │     ▼             ▼                                                    │  │
│  │  [有效]        [无效/过期]                                             │  │
│  │     │             │                                                    │  │
│  │     ▼             ▼                                                    │  │
│  │  返回缓存    ┌─────────────────────────────────────────┐               │  │
│  │              │ collect_fresh_metrics()                 │               │  │
│  │              │                                         │               │  │
│  │              │  tokio::join!(                          │               │  │
│  │              │    cpu_collector.collect(),      ───► CPU 信息         │  │
│  │              │    memory_collector.collect(),   ───► 内存信息         │  │
│  │              │    disk_collector.collect_all(), ───► 磁盘列表         │  │
│  │              │  )                                       │               │  │
│  │              │                                         │               │  │
│  │              │  构建 SystemMetrics { cpu, memory, disks, timestamp }  │  │
│  │              └─────────────────────────────────────────┘               │  │
│  │                          │                                             │  │
│  │                          ▼                                             │  │
│  │                    更新缓存                                            │  │
│  │                          │                                             │  │
│  │                          ▼                                             │  │
│  │                    返回数据                                            │  │
│  │                                                                        │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 🧅 依赖关系

```
service-monitor
└── external
    ├── tokio            # 异步运行时
    ├── serde            # 序列化
    ├── thiserror        # 错误定义
    ├── libc             # Unix 系统调用
    └── winapi           # Windows API (仅 Windows)
```
