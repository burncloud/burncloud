/// Machine-level context owned by the local Node runtime.
///
/// Business concepts such as Model, Provider, Route, Billing, or Tenant do not
/// belong here. They must stay in their owning BurnCloud domains.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeContext {
    started: bool,
}

impl NodeContext {
    pub fn started(&self) -> bool {
        self.started
    }

    pub(crate) fn mark_started(&mut self) {
        self.started = true;
    }
}
