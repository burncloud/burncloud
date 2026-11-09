//! Node 部署编排对外提供的事件、结果和错误契约。
//!
//! # Examples
//!
//! ```
//! use burncloud_node_runtime::deployment::{DeploymentEventKind, DeploymentStage};
//!
//! let event_kind = DeploymentEventKind::Progress(DeploymentStage::Queued);
//! assert_eq!(event_kind, DeploymentEventKind::Progress(DeploymentStage::Queued));
//! ```

// ============================================================================
// 模块级文档（//! 是 inner doc，作用于“包含它的这个模块”，即整个 deployment 模块）
// ============================================================================
//
// 文档说明本模块的职责：定义 Node 部署编排对外暴露的三类“契约”：
//   1. 事件（events）  —— DeploymentEvent / DeploymentEventKind
//   2. 结果（results） —— DeploymentOutcome
//   3. 错误（errors）  —— DeploymentError / DeploymentMonitorError 等
//
// “契约”一词强调：本模块是边界层，不关心内部如何实现，
// 只规定外部观察者能看到的数据形状与语义。
//
// # Examples 段落里的代码块是 doctest：
//   - cargo test 时会被编译并真正运行；
//   - 相当于给文档加了一个“可执行断言”，防止文档与代码漂移。
//
// 示例内容解析：
//   - 从 burncloud_node_runtime::deployment 导入 DeploymentEventKind 与 DeploymentStage；
//   - 构造 DeploymentEventKind::Progress(DeploymentStage::Queued)，
//     即“进度事件，阶段为 Queued（已排队）”；
//   - 断言该值等于自身，验证 DeploymentEventKind 实现了 PartialEq 且可正常构造。
//   - 同时隐含证明：DeploymentEventKind 是带数据的枚举变体 Progress(stage)，
//     DeploymentStage 是枚举且拥有 Queued 变体。

// ============================================================================
// 子模块声明（均为私有，外部无法通过 deployment::http_probe::... 访问）
// ============================================================================

// 声明 http_probe 子模块，内容位于 http_probe.rs。
// 该文件实现基于 HTTP 的 readiness/health 探测器 HttpProbe。
mod http_probe;

// 声明 process 子模块，内容位于 process.rs。
// 该文件实现本地进程管理器 LocalProcessManager（实现 ProcessManager trait，
// 负责启动/停止运行时进程）。
mod process;

// 声明 service 子模块，内容位于 service.rs。
// 该文件实现部署编排服务 DeploymentService，是对外的主要入口
// （编排 prepare / attach / detach 等流程）。
mod service;

// 声明 types 子模块，内容位于 types.rs。
// 该文件集中定义所有契约类型：事件、结果、错误、阶段等。
mod types;

// 说明：以上四个 mod 都没有 pub 前缀 => 模块本身对外不可见。
// 好处是内部文件可自由增删、重命名、拆分，不影响外部 API；
// 外部只能看到下面 pub use 刻意暴露的类型。

// ============================================================================
// 重新导出（pub use）：真正定义对外 API 表面的地方
// ============================================================================
//
// pub use 不是复制，而是把名字重新挂到当前模块下：
// 外部写 deployment::HttpProbe 时，实际解析到 deployment::http_probe::HttpProbe。
// 外部看不到 http_probe 这个模块名本身（因为 mod 是私有的），
// 只能看到 HttpProbe 这个类型 => “类型可见、模块不可见”的封装。

// 导出 HTTP 探测器：基于 HTTP 的 readiness/health 探测实现。
pub use http_probe::HttpProbe;

// 导出本地进程管理器：实现 ProcessManager，负责启动/停止本地运行时进程。
pub use process::LocalProcessManager;

// 导出部署编排服务：对外的主要入口，编排部署相关流程。
pub use service::DeploymentService;

// 从 types 子模块批量导出所有契约类型，分组列出便于阅读：
pub use types::{
    // 部署过程的顶层错误
    DeploymentError,
    // 错误分类（便于匹配/分类处理）
    DeploymentErrorKind,
    // 部署事件（携带 kind 与元数据）
    DeploymentEvent,
    // 事件种类（如 Progress(stage)）
    DeploymentEventKind,
    // 发布事件时发生的错误
    DeploymentEventPublishError,
    // 事件发布器 trait（观察者/订阅侧接口）
    DeploymentEventPublisher,
    // 部署监控通道错误（测试里 closed_monitor_errors() 返回的 receiver 即其通道）
    DeploymentMonitorError,
    // 部署最终结果
    DeploymentOutcome,
    // 部署阶段（Queued 等），与 Progress 事件配合使用
    DeploymentStage,
};
