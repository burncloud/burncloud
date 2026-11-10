use async_trait::async_trait;

#[path = "local_attachment_adapter.rs"]
mod adapter;
pub use adapter::ExistingRouterLocalAttacher;
#[path = "local_attachment_manager.rs"]
mod manager;
pub use manager::LocalRouteManager;
#[path = "local_attachment_state.rs"]
mod state;
pub(crate) use state::{configured_concurrency, LocalAttachmentState, LocalDispatchPermit};

/// 调用方已确认真实就绪的本机模型能力；不携带心跳或容量。
///
/// # 注意
/// 模型名按原样匹配，1–34 个 Unicode 标量值；根地址只接受无需上游密钥的
/// HTTP(S) 回环 IP，不能包含路径前缀、凭证、查询或片段。
///
/// # Examples
///
/// ```
/// use burncloud_router::local_attachment::LocalRouteAttachment;
/// let attachment = LocalRouteAttachment {
///     model: "qwen:4b".into(),
///     base_url: "http://127.0.0.1:8080".into(),
/// };
/// assert_eq!(attachment.model, "qwen:4b");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRouteAttachment {
    /// 已就绪且原样声明的一项模型别名。
    pub model: String,
    /// HTTP(S) 回环 IP 服务根地址。
    pub base_url: String,
}

/// 持久化成功后返回的实际供给身份；示例整数不代表实测注册。
///
/// # Examples
///
/// ```
/// use burncloud_router::local_attachment::LocalRouteAttachmentId;
/// assert_eq!(LocalRouteAttachmentId(42).0, 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalRouteAttachmentId(pub i32);

/// 可恢复的本机挂载管理错误，隔离沿用 DetachFailed。
///
/// # 注意
/// 错误字符串用于诊断，不能据此推断事务是否已经提交。
///
/// # Examples
///
/// ```
/// use burncloud_router::local_attachment::LocalRouteAttachmentError;
/// let error = LocalRouteAttachmentError::AttachFailed("invalid target".into());
/// assert!(error.to_string().contains("invalid target"));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum LocalRouteAttachmentError {
    /// 挂载输入、身份、证据或存储结果不能满足契约。
    #[error("local route attachment failed: {0}")]
    AttachFailed(String),
    /// 隔离或摘除未获确认，未知结果须继续禁派。
    #[error("local route detachment failed: {0}")]
    DetachFailed(String),
}

/// 把本机能力挂到既有供给事实的管理契约，不建立第二套路由表。
///
/// # 注意
/// 首次挂载直接探测服务及精确模型别名；运行中需要持续心跳。健康重复挂载返回原 ID，
/// 满载不影响幂等返回；隔离、故障锁、未决或心跳失效不允许重复挂载恢复。
///
/// # Examples
///
/// ```no_run
/// # async fn example(db: std::sync::Arc<burncloud_database::Database>) -> Result<(), burncloud_router::local_attachment::LocalRouteAttachmentError> {
/// use burncloud_router::local_attachment::{ExistingRouterLocalAttacher, LocalRouteAttacher, LocalRouteAttachment};
/// let attacher = ExistingRouterLocalAttacher::new(db);
/// let id = attacher.attach(LocalRouteAttachment {
///     model: "qwen:4b".into(), base_url: "http://127.0.0.1:8080".into(),
/// }).await?;
/// attacher.quarantine(id).await?;
/// attacher.detach(id).await?;
/// # Ok(()) }
/// ```
#[async_trait]
pub trait LocalRouteAttacher: Send + Sync {
    /// 校验输入并直接探测模型列表后返回实际持久化 ID；失败不自动重试写入。
    ///
    /// 使用示例见 trait 文档；同目标未摘除时不重复创建或解除隔离。
    async fn attach(
        &self,
        attachment: LocalRouteAttachment,
    ) -> Result<LocalRouteAttachmentId, LocalRouteAttachmentError>;

    /// 保留身份并停止授予新许可，隔离前的一次许可仍可完成。
    ///
    /// 使用示例见 trait 文档；重复隔离现存本机 ID 成功，不存在则失败。
    async fn quarantine(
        &self,
        attachment_id: LocalRouteAttachmentId,
    ) -> Result<(), LocalRouteAttachmentError>;

    /// 原子摘除指定本机 ID；可靠不存在时幂等成功，不影响重挂的新 ID。
    ///
    /// 使用示例见 trait 文档；非正 ID、非本机归属及数据库错误均失败。
    async fn detach(
        &self,
        attachment_id: LocalRouteAttachmentId,
    ) -> Result<(), LocalRouteAttachmentError>;
}
