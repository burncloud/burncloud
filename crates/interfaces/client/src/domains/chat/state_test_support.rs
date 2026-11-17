//! 仅用于工件流程回归的受控目录注入，不进入生产构建。

use super::{ChatModel, ChatState};

impl ChatState {
    /// 替换测试目录快照，不改变已选模型、会话或就绪凭据。
    ///
    /// # Examples
    /// 受控测试先注入目录，再通过原有选择与初始化入口驱动状态。
    ///
    /// # Notes
    /// 仅对 Chat 内部测试开放，非空模型 ID 与生产目录的身份约束一致。
    pub(in crate::domains::chat) fn set_test_models(&mut self, models: Vec<ChatModel>) {
        assert!(models.iter().all(|model| !model.id.trim().is_empty()));
        self.models = models;
    }
}
