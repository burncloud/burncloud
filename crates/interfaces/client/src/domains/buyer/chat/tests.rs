use super::{model::catalog, state::ChatState};
use crate::app::router::routes::Route;

#[test]
fn model_search_matches_name_family_and_description() {
    let mut state = ChatState::default();

    state.set_model_query("qwen".to_string());
    assert_eq!(state.suggestions().len(), 1);
    assert_eq!(state.suggestions()[0].model.id, "qwen-2.5-72b-instruct");

    state.set_model_query("anthropic".to_string());
    assert_eq!(
        state.suggestions()[0].model.name,
        "Claude 3.5 Sonnet Pass-Through"
    );

    state.set_model_query("multilingual".to_string());
    assert_eq!(state.suggestions()[0].model.family, "Qwen");
}

#[test]
fn model_suggestions_are_sorted_by_latest_update_and_limited() {
    let mut state = ChatState::default();
    state.set_model_query(String::new());

    let suggestions = super::state::search_models("");
    assert_eq!(suggestions[0].model.id, "qwen-2.5-72b-instruct");
    assert!(suggestions.len() <= 10);
    assert!(suggestions
        .windows(2)
        .all(|models| models[0].updated_at >= models[1].updated_at));
}

#[test]
fn selecting_a_model_closes_the_picker_and_allows_sending() {
    let mut state = ChatState::default();
    state.set_model_query("qwen".to_string());
    state.select_model("qwen-2.5-72b-instruct");
    state.set_draft("请介绍这个模型".to_string());

    assert_eq!(
        state.selected_model().map(|model| model.model.id),
        Some("qwen-2.5-72b-instruct")
    );
    assert!(!state.model_picker_open());
    assert!(state.can_send());

    state.send_message();
    assert_eq!(state.messages().len(), 1);
    assert_eq!(state.messages()[0].content(), "请介绍这个模型");
    assert!(state.draft().is_empty());
}

#[test]
fn buyer_chat_route_renders_the_chat_page() {
    assert!(matches!(
        "/buyer/chat".parse::<Route>(),
        Ok(Route::BuyerChatRoute {})
    ));
    assert_eq!(catalog().len(), 6);
}

#[test]
fn empty_query_hides_model_picker() {
    let mut state = ChatState::default();
    state.set_model_query("   ".to_string());

    assert!(!state.model_picker_open());
    assert!(state.suggestions().is_empty());
}

#[test]
fn unmatched_query_keeps_picker_open_with_no_results() {
    let mut state = ChatState::default();
    state.set_model_query("not-a-real-model".to_string());

    assert!(state.model_picker_open());
    assert!(state.suggestions().is_empty());
}

#[test]
fn model_id_can_be_used_for_search() {
    let mut state = ChatState::default();
    state.set_model_query("qwen-2.5-72b-instruct".to_string());

    assert_eq!(state.suggestions().len(), 1);
    assert_eq!(state.suggestions()[0].model.id, "qwen-2.5-72b-instruct");
}

#[test]
fn stale_browser_input_after_send_is_ignored() {
    let mut state = ChatState::default();
    state.select_model("qwen-2.5-72b-instruct");
    state.set_draft("hello".to_string());

    state.send_message();
    assert!(state.draft().is_empty());

    state.set_draft("hello\n".to_string());
    assert!(state.draft().is_empty());

    state.set_draft("new message".to_string());
    assert_eq!(state.draft(), "new message");
}

#[test]
fn resetting_chat_state_clears_model_draft_and_messages() {
    let mut state = ChatState::default();
    state.select_model("qwen-2.5-72b-instruct");
    state.set_draft("hello".to_string());
    state.send_message();

    state = ChatState::default();

    assert!(state.selected_model().is_none());
    assert!(state.draft().is_empty());
    assert!(state.messages().is_empty());
    assert!(!state.model_picker_open());
}
