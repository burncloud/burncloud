/// Machine-level context owned by the local Node runtime.
///
/// Business concepts such as Model, Provider, Route, Billing, or Tenant do not
/// belong here. They must stay in their owning BurnCloud domains.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeContext {
    started: bool,
}

impl NodeContext {
    /// 返回节点运行时是否已完成启动挂接。
    pub fn started(&self) -> bool {
        self.started
    }

    /// 将上下文标记为已启动，供节点生命周期入口调用。
    pub(crate) fn mark_started(&mut self) {
        self.started = true;
    }
}
