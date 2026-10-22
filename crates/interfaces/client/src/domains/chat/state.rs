//! Chat 首页的内存状态和交互边界。

use super::model::{
    catalog, sample_conversations, ChatConversation, ChatMessage, InferenceMetrics, MessageRole,
    SIMULATED_REPLY_SEGMENTS,
};

/// temperature 的边界错误。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TemperatureError {
    NotFinite,
    OutOfRange,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct GenerationPlan {
    pub(super) conversation_id: String,
    pub(super) generation_id: u64,
    pub(super) segments: &'static [&'static str],
}

/// Chat 首页的全部交互状态。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ChatState {
    conversations: Vec<ChatConversation>,
    active_conversation_id: Option<String>,
    selected_model_id: String,
    draft: String,
    temperature: f32,
    system_prompt: String,
    is_generating: bool,
    sidebar_collapsed: bool,
    model_menu_open: bool,
    temperature_menu_open: bool,
    generation_id: u64,
    next_conversation_number: u64,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            conversations: sample_conversations(),
            active_conversation_id: Some("conversation-1".to_string()),
            selected_model_id: "deepseek-r1".to_string(),
            draft: String::new(),
            temperature: 0.6,
            system_prompt: "You are a helpful, cryptographically attested AI model running on BurnCloud decentralized hardware enclaves.".to_string(),
            is_generating: false,
            sidebar_collapsed: false,
            model_menu_open: false,
            temperature_menu_open: false,
            generation_id: 0,
            next_conversation_number: 4,
        }
    }
}

impl ChatState {
    pub(super) fn conversations(&self) -> &[ChatConversation] {
        &self.conversations
    }

    pub(super) fn active_conversation(&self) -> Option<&ChatConversation> {
        self.active_conversation_id.as_deref().and_then(|id| {
            self.conversations
                .iter()
                .find(|conversation| conversation.id == id)
        })
    }

    pub(super) fn selected_model(&self) -> Option<super::model::ChatModel> {
        catalog()
            .into_iter()
            .find(|candidate| candidate.id() == self.selected_model_id)
    }

    pub(super) fn selected_model_id(&self) -> &str {
        &self.selected_model_id
    }

    pub(super) fn is_active_conversation(&self, conversation_id: &str) -> bool {
        self.active_conversation_id.as_deref() == Some(conversation_id)
    }

    pub(super) fn draft(&self) -> &str {
        &self.draft
    }

    pub(super) fn temperature(&self) -> f32 {
        self.temperature
    }

    pub(super) fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    pub(super) fn is_generating(&self) -> bool {
        self.is_generating
    }

    pub(super) fn sidebar_collapsed(&self) -> bool {
        self.sidebar_collapsed
    }

    pub(super) fn model_menu_open(&self) -> bool {
        self.model_menu_open
    }

    pub(super) fn temperature_menu_open(&self) -> bool {
        self.temperature_menu_open
    }

    pub(super) fn set_draft(&mut self, draft: String) {
        self.draft = draft;
    }

    pub(super) fn set_system_prompt(&mut self, system_prompt: String) {
        self.system_prompt = system_prompt;
    }

    pub(super) fn ensure_selected_model(&mut self) {
        if self.selected_model().is_some_and(|model| model.is_online()) {
            return;
        }
        if let Some(model) = catalog()
            .into_iter()
            .find(|candidate| candidate.is_online())
        {
            self.selected_model_id = model.id().to_string();
        }
    }

    pub(super) fn set_model(&mut self, model_id: &str) -> bool {
        let Some(model) = catalog()
            .into_iter()
            .find(|candidate| candidate.id() == model_id)
        else {
            return false;
        };
        if !model.is_online() {
            return false;
        }

        self.selected_model_id = model.id().to_string();
        if let Some(conversation) = self.active_conversation_mut() {
            conversation.model_id = model.id().to_string();
        }
        true
    }

    pub(super) fn set_temperature(&mut self, temperature: f32) -> Result<(), TemperatureError> {
        if !temperature.is_finite() {
            return Err(TemperatureError::NotFinite);
        }
        if !(0.0..=2.0).contains(&temperature) {
            return Err(TemperatureError::OutOfRange);
        }
        self.temperature = temperature;
        Ok(())
    }

    pub(super) fn new_conversation(&mut self) {
        let id = format!("conversation-{}", self.next_conversation_number);
        self.next_conversation_number = self.next_conversation_number.wrapping_add(1);
        self.conversations.push(ChatConversation {
            id: id.clone(),
            title: "新建会话".to_string(),
            model_id: self.selected_model_id.clone(),
            messages: Vec::new(),
        });
        self.active_conversation_id = Some(id);
        self.draft.clear();
        self.close_menus();
    }

    pub(super) fn select_conversation(&mut self, conversation_id: &str) -> bool {
        if self.is_generating {
            return false;
        }
        if self
            .conversations
            .iter()
            .any(|conversation| conversation.id == conversation_id)
        {
            self.active_conversation_id = Some(conversation_id.to_string());
            if let Some(conversation) = self.active_conversation() {
                self.selected_model_id = conversation.model_id.clone();
            }
            self.ensure_selected_model();
            self.draft.clear();
            self.close_menus();
            return true;
        }
        false
    }

    pub(super) fn delete_conversation(&mut self, conversation_id: &str) -> bool {
        if self.is_generating {
            return false;
        }
        let Some(index) = self
            .conversations
            .iter()
            .position(|conversation| conversation.id == conversation_id)
        else {
            return false;
        };
        let was_active = self.active_conversation_id.as_deref() == Some(conversation_id);
        self.conversations.remove(index);
        if was_active {
            self.active_conversation_id = if self.conversations.is_empty() {
                None
            } else if index > 0 {
                Some(self.conversations[index - 1].id.clone())
            } else {
                Some(self.conversations[0].id.clone())
            };
            if let Some(conversation) = self.active_conversation() {
                self.selected_model_id = conversation.model_id.clone();
            }
            self.ensure_selected_model();
        }
        true
    }

    pub(super) fn toggle_sidebar(&mut self) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
    }

    pub(super) fn collapse_sidebar(&mut self) {
        self.sidebar_collapsed = true;
    }

    pub(super) fn toggle_thinking(&mut self, conversation_id: &str, message_index: usize) -> bool {
        let Some(conversation) = self
            .conversations
            .iter_mut()
            .find(|conversation| conversation.id == conversation_id)
        else {
            return false;
        };
        let Some(message) = conversation.messages.get_mut(message_index) else {
            return false;
        };
        if message.role != MessageRole::Assistant || message.thinking.is_none() {
            return false;
        }
        message.thinking_open = !message.thinking_open;
        true
    }

    pub(super) fn toggle_model_menu(&mut self) {
        self.model_menu_open = !self.model_menu_open;
        if self.model_menu_open {
            self.temperature_menu_open = false;
        }
    }

    pub(super) fn toggle_temperature_menu(&mut self) {
        self.temperature_menu_open = !self.temperature_menu_open;
        if self.temperature_menu_open {
            self.model_menu_open = false;
        }
    }

    pub(super) fn close_menus(&mut self) {
        self.model_menu_open = false;
        self.temperature_menu_open = false;
    }

    pub(super) fn can_send(&self) -> bool {
        self.selected_model().is_some_and(|model| model.is_online())
            && !self.is_generating
            && !self.draft.trim().is_empty()
            && self.active_conversation_id.is_some()
    }

    pub(super) fn begin_generation(&mut self) -> Option<GenerationPlan> {
        if !self.can_send() {
            return None;
        }

        let conversation_id = self.active_conversation_id.clone()?;
        let model = self.selected_model()?;
        let draft = self.draft.trim().to_string();
        let conversation = self.active_conversation_mut()?;
        if conversation.title == "新建会话" && conversation.messages.is_empty() {
            conversation.title = draft
                .lines()
                .next()
                .unwrap_or("新建会话")
                .chars()
                .take(32)
                .collect();
        }
        conversation.messages.push(ChatMessage {
            role: MessageRole::User,
            content: draft,
            thinking: None,
            thinking_open: false,
            streaming: false,
            copied: false,
            metrics: None,
        });
        conversation.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            thinking: Some("正在校验请求上下文、模型能力和硬件证明链。".to_string()),
            thinking_open: true,
            streaming: true,
            copied: false,
            metrics: None,
        });
        self.draft.clear();
        self.is_generating = true;
        self.generation_id = self.generation_id.wrapping_add(1);
        self.close_menus();
        let generation_id = self.generation_id;
        let _ = model;
        Some(GenerationPlan {
            conversation_id,
            generation_id,
            segments: &SIMULATED_REPLY_SEGMENTS,
        })
    }

    pub(super) fn is_generation_active(&self, plan: &GenerationPlan) -> bool {
        self.is_generating
            && self.generation_id == plan.generation_id
            && self.active_conversation_id.as_deref() == Some(plan.conversation_id.as_str())
    }

    pub(super) fn append_stream_chunk(&mut self, plan: &GenerationPlan, chunk: &str) -> bool {
        if !self.is_generation_active(plan) {
            return false;
        }
        let Some(conversation) = self
            .conversations
            .iter_mut()
            .find(|conversation| conversation.id == plan.conversation_id)
        else {
            return false;
        };
        let Some(message) = conversation.messages.last_mut() else {
            return false;
        };
        if message.role != MessageRole::Assistant || !message.streaming {
            return false;
        }
        message.content.push_str(chunk);
        true
    }

    pub(super) fn complete_generation(&mut self, plan: &GenerationPlan, _total_ms: u32) -> bool {
        if !self.is_generation_active(plan) {
            return false;
        }
        let metrics = InferenceMetrics {
            tokens_per_second: 128.4,
            ttft_ms: 41,
            node: "AMD SEV-SNP #H100-08",
            cost_usd: 0.00014,
            prompt_tokens: 42,
            completion_tokens: 184,
        };
        let Some(conversation) = self
            .conversations
            .iter_mut()
            .find(|conversation| conversation.id == plan.conversation_id)
        else {
            return false;
        };
        let Some(message) = conversation.messages.last_mut() else {
            return false;
        };
        if message.role != MessageRole::Assistant || !message.streaming {
            return false;
        }
        message.streaming = false;
        message.metrics = Some(metrics);
        self.is_generating = false;
        true
    }

    pub(super) fn interrupt_generation(&mut self) {
        if !self.is_generating {
            return;
        }
        self.is_generating = false;
        self.generation_id = self.generation_id.wrapping_add(1);
        if let Some(conversation) = self.active_conversation_mut() {
            if let Some(message) = conversation.messages.last_mut() {
                if message.role == MessageRole::Assistant {
                    message.streaming = false;
                }
            }
        }
    }

    pub(super) fn mark_message_copied(
        &mut self,
        conversation_id: &str,
        message_index: usize,
        copied: bool,
    ) -> bool {
        let Some(conversation) = self
            .conversations
            .iter_mut()
            .find(|conversation| conversation.id == conversation_id)
        else {
            return false;
        };
        let Some(message) = conversation.messages.get_mut(message_index) else {
            return false;
        };
        if message.role != MessageRole::Assistant {
            return false;
        }
        message.copied = copied;
        true
    }

    fn active_conversation_mut(&mut self) -> Option<&mut ChatConversation> {
        let id = self.active_conversation_id.as_deref()?;
        self.conversations
            .iter_mut()
            .find(|conversation| conversation.id == id)
    }
}
