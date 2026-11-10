use super::{
    LocalAttachmentState, LocalRouteAttacher, LocalRouteAttachment, LocalRouteAttachmentError,
    LocalRouteAttachmentId,
};
use async_trait::async_trait;
use burncloud_database::Database;
use std::{net::IpAddr, sync::Arc};

/// 将调用方声明已就绪的本机端点挂入既有 Supply 供给事实。
///
/// # 注意
/// 同一 Database 对象的 Arc 克隆共享准入状态；释放句柄不会自动摘除。
/// 首次挂载直接探测精确模型别名；后续需管理器持续心跳。健康重复挂载不刷新心跳或扩容。
///
/// # Examples
///
/// ```no_run
/// # fn example(db: std::sync::Arc<burncloud_database::Database>) {
/// let attacher = burncloud_router::local_attachment::ExistingRouterLocalAttacher::new(db);
/// # }
/// ```
pub struct ExistingRouterLocalAttacher {
    db: Arc<Database>,
    local: Arc<LocalAttachmentState>,
}

impl ExistingRouterLocalAttacher {
    /// 注入数据库并取得同实例共享状态，不执行数据库或网络 I/O。
    ///
    /// # 注意
    /// Router 与适配器须传入同一 Arc 对象的克隆，不按连接字符串合并实例。
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn example(db: std::sync::Arc<burncloud_database::Database>) {
    /// let attacher = burncloud_router::local_attachment::ExistingRouterLocalAttacher::new(db.clone());
    /// let router = burncloud_router::model_router::ModelRouter::new(db);
    /// # }
    /// ```
    pub fn new(db: Arc<Database>) -> Self {
        let local = LocalAttachmentState::for_database(&db);
        Self { db, local }
    }
}

#[async_trait]
impl LocalRouteAttacher for ExistingRouterLocalAttacher {
    async fn attach(
        &self,
        attachment: LocalRouteAttachment,
    ) -> Result<LocalRouteAttachmentId, LocalRouteAttachmentError> {
        let attachment = normalize_attachment(attachment)?;
        self.local.attach(&self.db, attachment, None).await
    }

    async fn quarantine(
        &self,
        attachment_id: LocalRouteAttachmentId,
    ) -> Result<(), LocalRouteAttachmentError> {
        if attachment_id.0 <= 0 {
            return Err(LocalRouteAttachmentError::DetachFailed(
                "attachment id must be positive".into(),
            ));
        }
        self.local.quarantine(&self.db, attachment_id.0).await
    }

    async fn detach(
        &self,
        attachment_id: LocalRouteAttachmentId,
    ) -> Result<(), LocalRouteAttachmentError> {
        if attachment_id.0 <= 0 {
            return Err(LocalRouteAttachmentError::DetachFailed(
                "attachment id must be positive".into(),
            ));
        }
        self.local.detach(&self.db, attachment_id.0).await
    }
}

// 两个公有入口共享同一黑盒校验边界，避免启动清单与直接挂载的合法域漂移。
pub(super) fn normalize_attachment(
    mut attachment: LocalRouteAttachment,
) -> Result<LocalRouteAttachment, LocalRouteAttachmentError> {
    let model = &attachment.model;
    if model.is_empty()
        || model.chars().count() > 34
        || model.trim() != model
        || model.chars().any(|c| c == ',' || c.is_control())
    {
        return Err(LocalRouteAttachmentError::AttachFailed("model must be 1-34 Unicode scalars without edge whitespace, commas or control characters".into()));
    }
    let raw = &attachment.base_url;
    let invalid_url = || {
        LocalRouteAttachmentError::AttachFailed("base_url must be an HTTP(S) loopback IP root without credentials, path, query or fragment".into())
    };
    // 先查原始输入，防止 URL 解析器折叠点路径、反斜线或去掉控制字符后误放行。
    if raw.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(invalid_url());
    }
    let (_, authority) = raw.split_once("://").ok_or_else(invalid_url)?;
    let authority = authority.strip_suffix('/').unwrap_or(authority);
    if authority.contains(['/', '?', '#', '@', '\\']) {
        return Err(invalid_url());
    }
    let host = if let Some(v6) = authority.strip_prefix('[') {
        let (host, suffix) = v6.split_once(']').ok_or_else(invalid_url)?;
        if !suffix.is_empty()
            && !suffix
                .strip_prefix(':')
                .is_some_and(|port| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(invalid_url());
        }
        host
    } else {
        authority
            .split_once(':')
            .map_or(authority, |(host, _)| host)
    };
    let ip: IpAddr = host.parse().map_err(|_| invalid_url())?;
    let url = reqwest::Url::parse(raw).map_err(|_| invalid_url())?;
    if !ip.is_loopback()
        || !matches!(url.scheme(), "http" | "https")
        || url.path() != "/"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid_url());
    }
    attachment.base_url = url.as_str().trim_end_matches('/').to_owned();
    Ok(attachment)
}
