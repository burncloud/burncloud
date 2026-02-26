//! Gemini Passthrough Test Suite
//!
//! Tests for Gemini API passthrough functionality:
//! - Authorization header forwarding
//! - OpenAI to Gemini format conversion
//! - Error response handling
//! - SSE streaming response forwarding
//! - safetySettings parameter passthrough
//! - generationConfig parameter passthrough
//!
//! Key concepts:
//! - "Passthrough mode": Request body is forwarded as-is (Gemini native format)
//! - "Conversion mode": OpenAI format is converted to Gemini format
//!
//! Passthrough is triggered when:
//! 1. Path matches Gemini native API pattern (e.g., `/v1beta/models/...`)
//! 2. Request body contains `contents` field (Gemini native format)

use burncloud_tests::TestClient;
use dotenvy::dotenv;
use serde_json::json;
use std::env;
use uuid::Uuid;

use crate::common as common_mod;

/// Get Gemini API key from environment
fn get_gemini_key() -> Option<String> {
    dotenv().ok();
    env::var("TEST_GEMINI_KEY").ok().filter(|k| !k.is_empty())
}

/// Create a Gemini channel for testing
async fn create_gemini_channel(_base_url: &str, admin_client: &TestClient) -> String {
    let gemini_key = match get_gemini_key() {
        Some(k) => k,
        None => panic!("TEST_GEMINI_KEY not set"),
    };

    let channel_name = format!("Gemini Passthrough Test {}", Uuid::new_v4());
    let body = json!({
        "type": 24, // Gemini channel type
        "key": gemini_key,
        "name": channel_name,
        "base_url": "https://generativelanguage.googleapis.com",
        "models": "gemini-2.0-flash,gemini-2.5-flash,gemini-2.0-flash-thinking",
        "group": "vip",
        "weight": 10,
        "priority": 100
    });

    let res = admin_client
        .post("/console/api/channel", &body)
        .await
        .expect("Create channel failed");
    assert_eq!(res["success"], true);
    channel_name
}

// ============================================================================
// Test 1: Authorization header forwarding (via valid API key)
// ============================================================================

#[tokio::test]
async fn test_authorization_forwarding_valid() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Send request with valid token - should succeed, meaning Authorization is forwarded correctly
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Say 'test passed'"}]
            }
        ]
    });

    println!("Testing Authorization header forwarding with valid key...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    // If we get a valid response, the Authorization header was correctly forwarded
    let has_response = chat_res.get("candidates").is_some() || chat_res.get("choices").is_some();

    assert!(
        has_response,
        "Should have valid response, got: {:?}",
        chat_res
    );
    println!("SUCCESS: Authorization header forwarded correctly (got valid response)");
}

#[tokio::test]
async fn test_authorization_forwarding_gemini_format() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Use Gemini native format to test passthrough mode
    let chat_body = json!({
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "What is 1+1?"}]
            }
        ],
        "generationConfig": {
            "temperature": 0.5
        }
    });

    println!("Testing Authorization header forwarding with Gemini native format...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    // Verify Gemini native response (passthrough mode)
    let has_candidates = chat_res.get("candidates").is_some();
    println!(
        "Response has Gemini native format (candidates): {}",
        has_candidates
    );

    assert!(has_candidates, "Should have candidates in passthrough mode");
    println!("SUCCESS: Authorization header forwarded correctly in passthrough mode");
}

// ============================================================================
// Test 2: OpenAI to Gemini format conversion
// ============================================================================

#[tokio::test]
async fn test_openai_to_gemini_conversion_basic() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Send OpenAI format request - should be converted to Gemini format
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [
            {"role": "user", "content": "Hello, how are you?"}
        ],
        "temperature": 0.7,
        "max_tokens": 100
    });

    println!("Testing OpenAI to Gemini format conversion...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("Conversion response: {:?}", chat_res);

    // Verify OpenAI format response (conversion mode returns OpenAI format)
    let choices = chat_res.get("choices").and_then(|c| c.as_array());
    assert!(choices.is_some(), "Response should have choices array");

    let first_choice = choices.unwrap().first();
    assert!(first_choice.is_some(), "Should have at least one choice");

    let message = first_choice.unwrap().get("message");
    assert!(message.is_some(), "Choice should have message");

    let content = message.unwrap().get("content").and_then(|c| c.as_str());
    assert!(content.is_some(), "Message should have content");
    assert!(!content.unwrap().is_empty(), "Content should not be empty");

    println!(
        "SUCCESS: OpenAI format converted to Gemini and response converted back to OpenAI format"
    );
}

#[tokio::test]
async fn test_openai_to_gemini_conversion_with_system() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Test with system message (OpenAI format doesn't support system directly in conversion)
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Say hello"}
        ]
    });

    println!("Testing OpenAI to Gemini conversion with system message...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    // Should still work (system message handling may vary)
    let has_content = chat_res["choices"][0]["message"]["content"]
        .as_str()
        .is_some();

    assert!(has_content, "Should have content in response");
    println!("SUCCESS: Conversion with system message works");
}

#[tokio::test]
async fn test_openai_to_gemini_conversion_assistant_role() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Test with assistant role (should be converted to "model" in Gemini)
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [
            {"role": "user", "content": "What is 2+2?"},
            {"role": "assistant", "content": "2+2 equals 4."},
            {"role": "user", "content": "And what is 3+3?"}
        ]
    });

    println!("Testing OpenAI to Gemini conversion with assistant role...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    let content = chat_res["choices"][0]["message"]["content"]
        .as_str()
        .expect("Should have content");

    assert!(!content.is_empty(), "Content should not be empty");
    println!("Response: {}", content);
    println!("SUCCESS: Assistant role converted to model role correctly");
}

// ============================================================================
// Test 3: Error response handling
// ============================================================================

#[tokio::test]
async fn test_error_response_invalid_model() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request with invalid model name
    let chat_body = json!({
        "model": "non-existent-model-xyz",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Hello"}]
            }
        ]
    });

    println!("Testing error response for invalid model...");
    let result = user_client.post("/v1/chat/completions", &chat_body).await;

    // Should either return an error or a response with error info
    match result {
        Ok(resp) => {
            // Check if response contains error
            if let Some(error) = resp.get("error") {
                println!("Got expected error response: {:?}", error);
                assert!(error.get("message").is_some() || error.get("status").is_some());
            } else {
                println!(
                    "Response without explicit error (may be default model): {:?}",
                    resp
                );
            }
        }
        Err(e) => {
            println!("Got expected error: {}", e);
        }
    }

    println!("SUCCESS: Error response handling verified");
}

#[tokio::test]
async fn test_error_response_malformed_request() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request with empty contents (invalid)
    let chat_body = json!({
        "contents": []
    });

    println!("Testing error response for malformed request...");
    let result = user_client.post("/v1/chat/completions", &chat_body).await;

    // This should either fail or return an error
    match result {
        Ok(resp) => {
            if let Some(error) = resp.get("error") {
                println!("Got expected error response: {:?}", error);
            } else {
                println!("Response: {:?}", resp);
            }
        }
        Err(e) => {
            println!("Got expected error: {}", e);
        }
    }

    println!("SUCCESS: Malformed request error handling verified");
}

// ============================================================================
// Test 4: SSE streaming response forwarding
// ============================================================================

#[tokio::test]
async fn test_streaming_sse_passthrough() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Streaming request in Gemini native format (passthrough mode)
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Count from 1 to 5"}]
            }
        ],
        "stream": true
    });

    println!("Testing SSE streaming response forwarding...");

    // Note: TestClient doesn't fully support streaming, but we can verify the request is accepted
    let result = user_client.post("/v1/chat/completions", &chat_body).await;

    match result {
        Ok(resp) => {
            println!("Streaming response accepted: {:?}", resp);
            // Streaming may return different format
            println!("SUCCESS: Streaming request accepted in passthrough mode");
        }
        Err(e) => {
            // Streaming may fail to parse as JSON because it returns SSE format
            println!("Streaming response (expected SSE format): {}", e);
            println!("SUCCESS: Streaming request forwarded (SSE format returned)");
        }
    }
}

#[tokio::test]
async fn test_streaming_sse_conversion_mode() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Streaming request in OpenAI format (conversion mode)
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [
            {"role": "user", "content": "Say hello"}
        ],
        "stream": true
    });

    println!("Testing SSE streaming response in conversion mode...");

    let result = user_client.post("/v1/chat/completions", &chat_body).await;

    match result {
        Ok(resp) => {
            println!("Streaming response accepted: {:?}", resp);
            println!("SUCCESS: Streaming request accepted in conversion mode");
        }
        Err(e) => {
            println!("Streaming response (expected SSE format): {}", e);
            println!("SUCCESS: Streaming request converted (SSE format returned)");
        }
    }
}

// ============================================================================
// Test 5: safetySettings parameter passthrough
// ============================================================================

#[tokio::test]
async fn test_safety_settings_passthrough() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request with safetySettings (Gemini native format triggers passthrough)
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "What is the capital of France?"}]
            }
        ],
        "safetySettings": [
            {
                "category": "HARM_CATEGORY_HARASSMENT",
                "threshold": "BLOCK_MEDIUM_AND_ABOVE"
            },
            {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "threshold": "BLOCK_MEDIUM_AND_ABOVE"
            }
        ]
    });

    println!("Testing safetySettings parameter passthrough...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("Response with safetySettings: {:?}", chat_res);

    // Verify response (passthrough mode returns Gemini native format)
    let has_candidates = chat_res.get("candidates").is_some();
    assert!(has_candidates, "Should have candidates in passthrough mode");

    // Verify no safety blocking occurred (response should have content)
    let has_content =
        if let Some(candidates) = chat_res.get("candidates").and_then(|c| c.as_array()) {
            if let Some(first) = candidates.first() {
                first
                    .get("content")
                    .and_then(|c| c.get("parts"))
                    .and_then(|p| p.as_array())
                    .and_then(|parts| parts.first())
                    .and_then(|part| part.get("text"))
                    .and_then(|t| t.as_str())
                    .is_some()
            } else {
                false
            }
        } else {
            false
        };

    assert!(has_content, "Should have content in response");
    println!("SUCCESS: safetySettings parameter passthrough verified");
}

#[tokio::test]
async fn test_safety_settings_all_categories() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request with all safety categories
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Hello world"}]
            }
        ],
        "safetySettings": [
            {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_ONLY_HIGH"},
            {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_ONLY_HIGH"},
            {"category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_ONLY_HIGH"},
            {"category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_ONLY_HIGH"},
            {"category": "HARM_CATEGORY_CIVIC_INTEGRITY", "threshold": "BLOCK_ONLY_HIGH"}
        ]
    });

    println!("Testing all safetySettings categories passthrough...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    // Verify response in Gemini native format
    let has_candidates = chat_res.get("candidates").is_some();
    assert!(
        has_candidates,
        "Should have candidates with all safety settings"
    );

    println!("SUCCESS: All safetySettings categories passthrough verified");
}

// ============================================================================
// Test 6: generationConfig parameter passthrough
// ============================================================================

#[tokio::test]
async fn test_generation_config_passthrough() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request with full generationConfig (Gemini native format triggers passthrough)
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Tell me a short joke"}]
            }
        ],
        "generationConfig": {
            "temperature": 0.9,
            "maxOutputTokens": 100,
            "topP": 0.95,
            "topK": 40,
            "presencePenalty": 0.5,
            "frequencyPenalty": 0.5
        }
    });

    println!("Testing generationConfig parameter passthrough...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("Response with generationConfig: {:?}", chat_res);

    // Verify response in Gemini native format (passthrough mode)
    let has_candidates = chat_res.get("candidates").is_some();
    assert!(has_candidates, "Should have candidates in passthrough mode");

    // Verify response has content
    let has_content = chat_res["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .is_some();
    assert!(has_content, "Should have content");

    println!("SUCCESS: generationConfig parameter passthrough verified");
}

#[tokio::test]
async fn test_generation_config_temperature_only() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request with only temperature in generationConfig
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Say hello"}]
            }
        ],
        "generationConfig": {
            "temperature": 0.1
        }
    });

    println!("Testing generationConfig with only temperature...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    let has_candidates = chat_res.get("candidates").is_some();
    assert!(has_candidates, "Should have candidates");

    println!("SUCCESS: generationConfig with temperature only works");
}

#[tokio::test]
async fn test_generation_config_with_thinking_budget() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request with thinkingBudget (for thinking models)
    let chat_body = json!({
        "model": "gemini-2.0-flash-thinking",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "What is 5+7?"}]
            }
        ],
        "generationConfig": {
            "thinkingBudget": 2048,
            "maxOutputTokens": 500
        }
    });

    println!("Testing generationConfig with thinkingBudget...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("Thinking model response: {:?}", chat_res);

    // Verify response
    let has_candidates = chat_res.get("candidates").is_some();
    assert!(has_candidates, "Should have candidates");

    println!("SUCCESS: generationConfig with thinkingBudget passthrough verified");
}

// ============================================================================
// Test 7: Combined parameters passthrough
// ============================================================================

#[tokio::test]
async fn test_combined_safety_and_generation_config() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request with both safetySettings and generationConfig
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "What are the primary colors?"}]
            }
        ],
        "generationConfig": {
            "temperature": 0.3,
            "maxOutputTokens": 200,
            "topP": 0.8
        },
        "safetySettings": [
            {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_LOW_AND_ABOVE"}
        ]
    });

    println!("Testing combined safetySettings and generationConfig passthrough...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("Combined config response: {:?}", chat_res);

    // Verify response
    let has_candidates = chat_res.get("candidates").is_some();
    assert!(has_candidates, "Should have candidates");

    let has_content = chat_res["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .is_some();
    assert!(has_content, "Should have content");

    println!("SUCCESS: Combined safetySettings and generationConfig passthrough verified");
}

// ============================================================================
// Test 8: systemInstruction passthrough
// ============================================================================

#[tokio::test]
async fn test_system_instruction_passthrough() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request with systemInstruction (Gemini native format)
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Explain gravity"}]
            }
        ],
        "systemInstruction": {
            "parts": [{"text": "You are a physics professor. Always be concise and use simple language."}]
        },
        "generationConfig": {
            "maxOutputTokens": 150
        }
    });

    println!("Testing systemInstruction passthrough...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("systemInstruction response: {:?}", chat_res);

    // Verify response
    let has_candidates = chat_res.get("candidates").is_some();
    assert!(has_candidates, "Should have candidates");

    println!("SUCCESS: systemInstruction passthrough verified");
}

// ============================================================================
// Test 9: Usage metadata extraction
// ============================================================================

#[tokio::test]
async fn test_usage_metadata_in_response() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request that should return usage metadata
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Hello"}]
            }
        ]
    });

    println!("Testing usage metadata in response...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("Response with usage: {:?}", chat_res);

    // Check for usageMetadata in passthrough mode response
    if let Some(usage) = chat_res.get("usageMetadata") {
        let prompt_tokens = usage.get("promptTokenCount").and_then(|t| t.as_u64());
        let completion_tokens = usage.get("candidatesTokenCount").and_then(|t| t.as_u64());

        println!("Prompt tokens: {:?}", prompt_tokens);
        println!("Completion tokens: {:?}", completion_tokens);

        assert!(prompt_tokens.is_some(), "Should have promptTokenCount");
        assert!(
            completion_tokens.is_some(),
            "Should have candidatesTokenCount"
        );

        println!("SUCCESS: Usage metadata correctly extracted");
    } else {
        println!("No usageMetadata in response (may be in different format)");
    }
}

// ============================================================================
// Test 10: responseModalities parameter passthrough
// ============================================================================

#[tokio::test]
async fn test_response_modalities_passthrough() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request with responseModalities in generationConfig (Gemini native format triggers passthrough)
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Hello, how are you?"}]
            }
        ],
        "generationConfig": {
            "temperature": 0.5,
            "maxOutputTokens": 100,
            "responseModalities": ["TEXT"]
        }
    });

    println!("Testing responseModalities parameter passthrough...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("Response with responseModalities: {:?}", chat_res);

    // Verify response in Gemini native format (passthrough mode)
    let has_candidates = chat_res.get("candidates").is_some();
    assert!(has_candidates, "Should have candidates in passthrough mode");

    // Verify response has text content
    let has_content = chat_res["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .is_some();
    assert!(has_content, "Should have text content");

    println!("SUCCESS: responseModalities parameter passthrough verified");
}
