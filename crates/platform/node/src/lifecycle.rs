use crate::NodeContext;

/// Minimal lifecycle hook for attaching the local Node runtime to BurnCloud.
///
/// Starting the Node runtime must not bind ports or create another HTTP server.
#[derive(Debug, Default)]
pub struct NodeRuntime;

impl NodeRuntime {
    /// 创建节点运行时生命周期入口。
    pub fn new() -> Self {
        Self
    }

    /// 初始化节点上下文并记录运行时已挂接。
    pub fn start(&self) -> NodeContext {
        let mut context = NodeContext::default();
        // 此生命周期入口只更新本地状态，不创建监听端口或 HTTP 服务。
        context.mark_started();
        tracing::info!("BurnCloud Node runtime attached");
        context
    }
}

#[cfg(test)]
mod tests {
    use super::NodeRuntime;

    #[test]
    fn start_returns_started_context() {
        let context = NodeRuntime::new().start();
        assert!(context.started());
    }
}
