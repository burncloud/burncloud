use super::{
    actions::code_snippet,
    model::{CodeLanguage, RoutingTier, MODEL_CATALOG, STREAMED_RESPONSE},
    state::{estimate_cost, PlaygroundState, DEFAULT_SYSTEM_PROMPT, DEFAULT_USER_PROMPT},
};
use crate::app::router::routes::Route;

#[test]
fn default_state_matches_reference_playground() {
    let state = PlaygroundState::default();

    assert_eq!(state.selected_model().id, "deepseek-v3");
    assert_eq!(state.tier, RoutingTier::Standard);
    assert_eq!(state.system_prompt, DEFAULT_SYSTEM_PROMPT);
    assert_eq!(state.user_prompt, DEFAULT_USER_PROMPT);
    assert_eq!(state.temperature, 0.7);
    assert_eq!(state.max_tokens, 2_048);
    assert_eq!(state.code_language, CodeLanguage::Curl);
    assert!(!state.running);
    assert!(state.output.is_empty());
    assert!(state.stats.is_none());
}

#[test]
fn reference_model_catalog_contains_six_expected_models() {
    assert_eq!(MODEL_CATALOG.len(), 6);
    assert_eq!(
        MODEL_CATALOG
            .iter()
            .map(|model| model.id)
            .collect::<Vec<_>>(),
        vec![
            "deepseek-v3",
            "deepseek-r1",
            "qwen-2.5-72b-instruct",
            "claude-3-5-sonnet",
            "llama-3.3-70b-instruct",
            "glm-4-plus",
        ]
    );
}

#[test]
fn every_code_example_reflects_live_parameters() {
    let mut state = PlaygroundState::default();
    state.set_model_id("deepseek-r1");
    state.tier = RoutingTier::Performance;
    state.temperature = 0.3;
    state.max_tokens = 1_024;
    state.system_prompt = "System test".to_string();
    state.user_prompt = "User test".to_string();

    for language in CodeLanguage::ALL {
        state.code_language = language;
        let code = code_snippet(&state);
        assert!(code.contains("deepseek-r1"));
        assert!(code.contains("performance"));
        assert!(code.contains("System test"));
        assert!(code.contains("User test"));
        assert!(code.contains("0.3"));
        assert!(code.contains("1024"));
    }
}

#[test]
fn default_curl_example_matches_reference_content() {
    let code = code_snippet(&PlaygroundState::default());
    let expected = concat!(
        "curl https://api.burncloud.io/v1/chat/completions \\\n",
        "  -H \"Content-Type: application/json\" \\\n",
        "  -H \"Authorization: Bearer $BURNCLOUD_API_KEY\" \\\n",
        "  -H \"X-BurnCloud-Tier: standard\" \\\n",
        "  -d '{\n",
        "    \"model\": \"deepseek-v3\",\n",
        "    \"messages\": [\n",
        "      {\"role\": \"system\", \"content\": \"You are an expert full-stack engineer and distributed systems architect. Provide precise, actionable technical answers.\"},\n",
        "      {\"role\": \"user\", \"content\": \"Explain how hardware attestation guarantees secure multi-tenant LLM token generation.\"}\n",
        "    ],\n",
        "    \"temperature\": 0.7,\n",
        "    \"max_tokens\": 2048\n",
        "  }'",
    );

    assert_eq!(code, expected);
}

#[test]
fn routing_tiers_have_reference_ttft_values() {
    assert_eq!(RoutingTier::Economy.ttft_ms(), 230);
    assert_eq!(RoutingTier::Standard.ttft_ms(), 148);
    assert_eq!(RoutingTier::Performance.ttft_ms(), 112);
}

#[test]
fn inference_cost_uses_reference_token_counts_and_model_prices() {
    assert_eq!(estimate_cost(MODEL_CATALOG[0]), 0.000057);
    assert_eq!(estimate_cost(MODEL_CATALOG[1]), 0.000426);

    let mut state = PlaygroundState {
        tier: RoutingTier::Performance,
        ..Default::default()
    };
    state.begin_inference();
    state.append_stream_chunk(13);
    assert!(!state.output.is_empty());
    state.complete_inference(2_030);

    let stats = state.stats.expect("completed inference provides stats");
    assert_eq!(state.output, STREAMED_RESPONSE);
    assert_eq!(stats.ttft_ms, 112);
    assert_eq!(stats.total_ms, 2_030);
    assert_eq!(stats.prompt_tokens, 42);
    assert_eq!(stats.completion_tokens, 184);
    assert_eq!(stats.cost_usd, 0.000057);
}

#[test]
fn clear_only_resets_inference_result() {
    let mut state = PlaygroundState {
        system_prompt: "Keep system prompt".to_string(),
        user_prompt: "Keep user prompt".to_string(),
        temperature: 1.2,
        max_tokens: 4_096,
        ..Default::default()
    };
    state.begin_inference();
    state.complete_inference(1_000);
    state.clear_output();

    assert!(state.output.is_empty());
    assert!(state.stats.is_none());
    assert!(!state.running);
    assert_eq!(state.system_prompt, "Keep system prompt");
    assert_eq!(state.user_prompt, "Keep user prompt");
    assert_eq!(state.temperature, 1.2);
    assert_eq!(state.max_tokens, 4_096);
}

#[test]
fn playground_route_parses_to_its_dedicated_component() {
    assert!(matches!(
        "/buyer/playground".parse::<Route>(),
        Ok(Route::Playground {})
    ));
}
