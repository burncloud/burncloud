use crate::NodeContext;

/// Minimal lifecycle hook for attaching the local Node runtime to BurnCloud.
///
/// Starting the Node runtime must not bind ports or create another HTTP server.
#[derive(Debug, Default)]
pub struct NodeRuntime;

impl NodeRuntime {
    pub fn new() -> Self {
        Self
    }

    pub fn start(&self) -> NodeContext {
        let mut context = NodeContext::default();
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
