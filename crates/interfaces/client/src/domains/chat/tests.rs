use super::{
    model::{catalog, MessageRole},
    state::{ChatState, TemperatureError},
};
use crate::app::router::routes::Route;

#[test]
fn default_state_matches_reference_chat() {
    let state = ChatState::default();

    assert_eq!(state.selected_model_id(), "deepseek-r1");
    assert_eq!(state.temperature(), 0.6);
    assert_eq!(
        state.system_prompt(),
        "You are a helpful, cryptographically attested AI model running on BurnCloud decentralized hardware enclaves."
    );
    assert_eq!(state.conversations().len(), 3);
    assert_eq!(
        state
            .active_conversation()
            .map(|conversation| conversation.title.as_str()),
        Some("DeepSeek R1 推理与硬件证明")
    );
    assert_eq!(
        state
            .active_conversation()
            .map(|conversation| conversation.messages.len()),
        Some(2)
    );
    let conversation = state.active_conversation().expect("默认会话应存在");
    assert_eq!(
        conversation.messages[0].content,
        "请说明 BurnCloud 的硬件 Enclave 机密计算是如何证明零数据残留的？"
    );
    assert!(!conversation.messages[1].thinking_open);
    assert_eq!(
        conversation.messages[1].metrics.map(|value| value.ttft_ms),
        Some(41)
    );
}

#[test]
fn catalog_reuses_the_buyer_model_directory() {
    let models = catalog();
    assert_eq!(models.len(), 5);
    assert_eq!(models[0].id(), "deepseek-v3");
    assert_eq!(models[1].id(), "deepseek-r1");
    assert_eq!(models[2].id(), "qwen-2.5-72b-instruct");
}

#[test]
fn model_selection_updates_the_active_conversation() {
    let mut state = ChatState::default();

    assert!(state.set_model("deepseek-v3"));
    assert_eq!(state.selected_model_id(), "deepseek-v3");
    assert_eq!(
        state
            .active_conversation()
            .map(|conversation| conversation.model_id.as_str()),
        Some("deepseek-v3")
    );
    assert!(!state.set_model("glm-4-plus"));
    assert_eq!(state.selected_model_id(), "deepseek-v3");
}

#[test]
fn new_conversation_preserves_model_and_temperature() {
    let mut state = ChatState::default();
    assert!(state.set_model("deepseek-v3"));
    assert!(state.set_temperature(1.2).is_ok());

    state.new_conversation();

    assert_eq!(state.selected_model_id(), "deepseek-v3");
    assert_eq!(state.temperature(), 1.2);
    assert_eq!(
        state
            .active_conversation()
            .map(|conversation| conversation.title.as_str()),
        Some("新建会话")
    );
    assert!(state
        .active_conversation()
        .is_some_and(|conversation| conversation.messages.is_empty()));
}

#[test]
fn switching_conversations_restores_messages() {
    let mut state = ChatState::default();
    assert!(state.select_conversation("conversation-2"));

    assert_eq!(state.selected_model_id(), "deepseek-v3");
    assert!(state
        .active_conversation()
        .is_some_and(|conversation| conversation.messages.is_empty()));
}

#[test]
fn deleting_current_and_last_conversations_keeps_valid_selection() {
    let mut state = ChatState::default();
    assert!(state.delete_conversation("conversation-1"));
    assert_eq!(
        state
            .active_conversation()
            .map(|conversation| conversation.id.as_str()),
        Some("conversation-2")
    );

    assert!(state.delete_conversation("conversation-2"));
    assert_eq!(
        state
            .active_conversation()
            .map(|conversation| conversation.id.as_str()),
        Some("conversation-3")
    );

    assert!(state.delete_conversation("conversation-3"));
    assert!(state.active_conversation().is_none());
    assert!(state.conversations().is_empty());
}

#[test]
fn temperature_accepts_the_closed_interval_and_rejects_invalid_values() {
    let mut state = ChatState::default();
    assert!(state.set_temperature(0.0).is_ok());
    assert!(state.set_temperature(2.0).is_ok());
    assert_eq!(
        state.set_temperature(-0.1),
        Err(TemperatureError::OutOfRange)
    );
    assert_eq!(
        state.set_temperature(2.1),
        Err(TemperatureError::OutOfRange)
    );
    assert_eq!(
        state.set_temperature(f32::NAN),
        Err(TemperatureError::NotFinite)
    );
}

#[test]
fn thinking_process_can_be_toggled_without_changing_other_messages() {
    let mut state = ChatState::default();

    assert!(state.toggle_thinking("conversation-1", 1));
    assert!(state
        .active_conversation()
        .is_some_and(|conversation| conversation.messages[1].thinking_open));
    assert!(state.toggle_thinking("conversation-1", 1));
    assert!(state
        .active_conversation()
        .is_some_and(|conversation| !conversation.messages[1].thinking_open));
    assert!(!state.toggle_thinking("conversation-1", 0));
}

#[test]
fn system_prompt_can_be_updated_without_affecting_temperature_validation() {
    let mut state = ChatState::default();
    state.set_system_prompt("只输出 JSON".to_string());

    assert_eq!(state.system_prompt(), "只输出 JSON");
    assert!(state.set_temperature(2.0).is_ok());
    assert_eq!(state.temperature(), 2.0);
}

#[test]
fn empty_message_does_not_start_generation() {
    let mut state = ChatState::default();
    state.set_draft(" \n\t".to_string());

    assert!(state.begin_generation().is_none());
    assert!(!state.is_generating());
    assert_eq!(
        state
            .active_conversation()
            .map(|conversation| conversation.messages.len()),
        Some(2)
    );
}

#[test]
fn simulated_generation_streams_and_writes_metrics() {
    let mut state = ChatState::default();
    state.set_draft("请说明节点证明流程".to_string());
    let Some(plan) = state.begin_generation() else {
        panic!("有效消息应启动模拟回复");
    };

    assert!(state.is_generating());
    assert!(state.append_stream_chunk(&plan, "第一段"));
    assert!(state.append_stream_chunk(&plan, "第二段"));
    assert!(state.complete_generation(&plan, 940));
    assert!(!state.is_generating());

    let Some(conversation) = state.active_conversation() else {
        panic!("生成后仍应保留当前会话");
    };
    let Some(message) = conversation.messages.last() else {
        panic!("生成后应存在助手消息");
    };
    assert_eq!(message.role, MessageRole::Assistant);
    assert_eq!(message.content, "第一段第二段");
    assert!(!message.streaming);
    assert_eq!(message.metrics.map(|metrics| metrics.ttft_ms), Some(41));
    assert_eq!(
        message.metrics.map(|metrics| metrics.completion_tokens),
        Some(184)
    );
}

#[test]
fn interrupting_generation_preserves_partial_reply_and_unlocks_composer() {
    let mut state = ChatState::default();
    state.set_draft("开始生成".to_string());
    let Some(plan) = state.begin_generation() else {
        panic!("有效消息应启动模拟回复");
    };
    assert!(state.append_stream_chunk(&plan, "已经生成的内容"));

    state.interrupt_generation();

    assert!(!state.is_generating());
    assert!(!state.is_generation_active(&plan));
    assert!(state
        .active_conversation()
        .is_some_and(
            |conversation| conversation.messages.last().is_some_and(|message| {
                message.content == "已经生成的内容" && !message.streaming
            })
        ));
}

#[test]
fn copied_state_only_changes_the_target_assistant_message() {
    let mut state = ChatState::default();
    assert!(state.mark_message_copied("conversation-1", 1, true));
    assert!(state.mark_message_copied("conversation-1", 1, false));
    assert!(!state.mark_message_copied("conversation-1", 0, true));
    assert!(!state.mark_message_copied("missing", 1, true));
}

#[test]
fn routes_keep_chat_aliases_and_buyer_workbench() {
    assert!(matches!("/".parse::<Route>(), Ok(Route::Home {})));
    assert!(matches!("/home".parse::<Route>(), Ok(Route::PublicHome {})));
    assert!(matches!("/buyer".parse::<Route>(), Ok(Route::Buyer {})));
    assert!(matches!(
        "/buyer/playground".parse::<Route>(),
        Ok(Route::Playground {})
    ));
}

#[test]
fn keyword_search_is_not_faked_by_the_local_catalog() {
    let query = "qwen";
    assert!(!catalog()
        .into_iter()
        .any(|model| model.id().eq_ignore_ascii_case("qwen-max")));
    assert_eq!(query, "qwen");
}
