//! 聊天页的模型搜索和本地消息状态。

use super::model::{catalog, ChatModel};

const MAX_MODEL_SUGGESTIONS: usize = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MessageRole {
    User,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ChatMessage {
    role: MessageRole,
    content: String,
    model_name: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct ChatState {
    model_query: String,
    selected_model_id: Option<&'static str>,
    draft: String,
    messages: Vec<ChatMessage>,
    model_picker_open: bool,
    last_sent_draft: Option<String>,
}

impl ChatState {
    pub(super) fn model_query(&self) -> &str {
        &self.model_query
    }

    pub(super) fn draft(&self) -> &str {
        &self.draft
    }

    pub(super) fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub(super) fn model_picker_open(&self) -> bool {
        self.model_picker_open
    }

    pub(super) fn selected_model(&self) -> Option<ChatModel> {
        self.selected_model_id.and_then(|id| {
            catalog()
                .into_iter()
                .find(|candidate| candidate.model.id == id)
        })
    }

    pub(super) fn suggestions(&self) -> Vec<ChatModel> {
        if !self.model_picker_open {
            return Vec::new();
        }

        search_models(&self.model_query)
    }

    pub(super) fn set_model_query(&mut self, query: String) {
        self.model_query = query;
        self.model_picker_open = !self.model_query.trim().is_empty();
        self.selected_model_id = None;
    }

    pub(super) fn select_model(&mut self, model_id: &str) {
        if let Some(model) = catalog()
            .into_iter()
            .find(|candidate| candidate.model.id == model_id)
        {
            self.selected_model_id = Some(model.model.id);
            self.model_query = model.model.name.to_string();
            self.model_picker_open = false;
        }
    }

    pub(super) fn set_draft(&mut self, draft: String) {
        let stale_input = self
            .last_sent_draft
            .as_deref()
            .is_some_and(|sent| draft == sent || draft.strip_suffix('\n') == Some(sent));

        self.last_sent_draft = None;
        if !stale_input {
            self.draft = draft;
        }
    }

    pub(super) fn can_send(&self) -> bool {
        self.selected_model_id.is_some() && !self.draft.trim().is_empty()
    }

    pub(super) fn send_message(&mut self) {
        if !self.can_send() {
            return;
        }

        let Some(model) = self.selected_model() else {
            return;
        };

        let draft = self.draft.clone();
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: draft.trim().to_string(),
            model_name: model.model.name,
        });
        self.last_sent_draft = Some(draft);
        self.draft.clear();
    }
}

pub(super) fn search_models(query: &str) -> Vec<ChatModel> {
    let query = query.trim().to_ascii_lowercase();
    let mut matches = catalog()
        .into_iter()
        .filter(|candidate| {
            query.is_empty()
                || candidate.model.id.to_ascii_lowercase().contains(&query)
                || candidate.model.name.to_ascii_lowercase().contains(&query)
                || candidate.model.family.to_ascii_lowercase().contains(&query)
                || candidate
                    .model
                    .description
                    .to_ascii_lowercase()
                    .contains(&query)
        })
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| {
        right
            .updated_at
            .cmp(left.updated_at)
            .then_with(|| left.model.name.cmp(right.model.name))
    });
    matches.truncate(MAX_MODEL_SUGGESTIONS);
    matches
}

impl ChatMessage {
    pub(super) fn content(&self) -> &str {
        &self.content
    }

    pub(super) fn model_name(&self) -> &'static str {
        self.model_name
    }

    pub(super) fn is_user(&self) -> bool {
        self.role == MessageRole::User
    }
}
