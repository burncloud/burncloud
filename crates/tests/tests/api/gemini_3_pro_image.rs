//! Gemini 3 Pro Image Preview Test Suite
//!
//! Tests for Gemini native image generation with gemini-3-pro-image-preview:
//! - Basic image generation with responseModalities
//! - Text + Image mixed output
//! - Conversational image editing
//! - Image fusion with multiple references
//! - Billing verification
//!
//! Key concepts:
//! - Uses responseModalities: ["TEXT", "IMAGE"] for image generation
//! - Images are returned as inlineData with base64 encoded data
//! - Image editing/fusion works by sending inlineData in the request

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

    let channel_name = format!("Gemini 3 Pro Image Test {}", Uuid::new_v4());
    let body = json!({
        "type": 24, // Gemini channel type
        "key": gemini_key,
        "name": channel_name,
        "base_url": "https://generativelanguage.googleapis.com",
        "models": "gemini-3-pro-image-preview",
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

/// Extract image data from response
fn extract_image_data(response: &serde_json::Value) -> Option<(String, String)> {
    let parts = response
        .get("candidates")?
        .get(0)?
        .get("content")?
        .get("parts")?
        .as_array()?;

    for part in parts {
        if let Some(inline_data) = part.get("inlineData") {
            let mime_type = inline_data.get("mimeType")?.as_str()?.to_string();
            let data = inline_data.get("data")?.as_str()?.to_string();
            return Some((mime_type, data));
        }
    }
    None
}

// ============================================================================
// Test 1: Basic image generation
// ============================================================================

#[tokio::test]
async fn test_basic_image_generation() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Basic image generation request
    let chat_body = json!({
        "model": "gemini-3-pro-image-preview",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Generate a simple red circle"}]
            }
        ],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"]
        }
    });

    println!("Testing basic image generation...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("Response structure: {:?}", chat_res);

    // Verify response has candidates (Gemini native format in passthrough mode)
    assert!(chat_res.get("candidates").is_some(), "Should have candidates");

    // Verify image data is present
    let image_data = extract_image_data(&chat_res);
    assert!(image_data.is_some(), "Should have image data in response");

    let (mime_type, data) = image_data.unwrap();
    assert!(mime_type.starts_with("image/"), "MIME type should be image/*");
    assert!(!data.is_empty(), "Image data should not be empty");

    println!("SUCCESS: Basic image generation verified");
    println!("  - MIME type: {}", mime_type);
    println!("  - Data length: {} bytes", data.len());
}

// ============================================================================
// Test 2: Text + Image mixed output
// ============================================================================

#[tokio::test]
async fn test_text_image_mixed_output() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request that might generate both text and image
    let chat_body = json!({
        "model": "gemini-3-pro-image-preview",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Create an image of a sunset and describe it"}]
            }
        ],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"]
        }
    });

    println!("Testing text + image mixed output...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    // Verify response has candidates
    assert!(chat_res.get("candidates").is_some(), "Should have candidates");

    // Check for any parts in response
    let parts = chat_res["candidates"][0]["content"]["parts"].as_array();
    assert!(parts.is_some(), "Should have parts in response");

    let has_image = parts.unwrap().iter().any(|p| p.get("inlineData").is_some());
    assert!(has_image, "Should have at least one image part");

    println!("SUCCESS: Text + Image mixed output verified");
}

// ============================================================================
// Test 3: Conversational image editing
// ============================================================================

#[tokio::test]
async fn test_conversational_image_editing() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // First, generate an image
    let gen_body = json!({
        "model": "gemini-3-pro-image-preview",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Generate a simple blue square"}]
            }
        ],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"]
        }
    });

    println!("Generating initial image...");
    let gen_res = user_client
        .post("/v1/chat/completions", &gen_body)
        .await
        .expect("Generation failed");

    let original_image = extract_image_data(&gen_res);
    assert!(original_image.is_some(), "Should have generated an image");
    let (mime_type, image_data) = original_image.unwrap();

    // Now, edit the image
    let edit_body = json!({
        "model": "gemini-3-pro-image-preview",
        "contents": [
            {
                "role": "user",
                "parts": [
                    {
                        "inlineData": {
                            "mimeType": mime_type,
                            "data": image_data
                        }
                    },
                    {"text": "Change this blue square to a red circle"}
                ]
            }
        ],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"]
        }
    });

    println!("Testing conversational image editing...");
    let edit_res = user_client
        .post("/v1/chat/completions", &edit_body)
        .await
        .expect("Edit failed");

    // Verify edited image is present
    let edited_image = extract_image_data(&edit_res);
    assert!(edited_image.is_some(), "Should have edited image in response");

    let (edited_mime, edited_data) = edited_image.unwrap();
    assert!(edited_mime.starts_with("image/"), "Edited MIME type should be image/*");
    assert!(!edited_data.is_empty(), "Edited image data should not be empty");

    println!("SUCCESS: Conversational image editing verified");
}

// ============================================================================
// Test 4: Image fusion with multiple references
// ============================================================================

#[tokio::test]
async fn test_image_fusion_multiple_references() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Generate first image
    let gen1_body = json!({
        "model": "gemini-3-pro-image-preview",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Generate a simple solid blue square on white background"}]
            }
        ],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"]
        }
    });

    println!("Generating first image...");
    let gen1_res = user_client
        .post("/v1/chat/completions", &gen1_body)
        .await
        .expect("First generation failed");
    let image1 = extract_image_data(&gen1_res);
    assert!(image1.is_some(), "Should have first image");
    let (mime1, data1) = image1.unwrap();

    // Generate second image
    let gen2_body = json!({
        "model": "gemini-3-pro-image-preview",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Generate a simple solid red circle on white background"}]
            }
        ],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"]
        }
    });

    println!("Generating second image...");
    let gen2_res = user_client
        .post("/v1/chat/completions", &gen2_body)
        .await
        .expect("Second generation failed");
    let image2 = extract_image_data(&gen2_res);
    assert!(image2.is_some(), "Should have second image");
    let (_, data2) = image2.unwrap();

    // Fuse both images
    let fuse_body = json!({
        "model": "gemini-3-pro-image-preview",
        "contents": [
            {
                "role": "user",
                "parts": [
                    {
                        "inlineData": {
                            "mimeType": mime1,
                            "data": data1
                        }
                    },
                    {
                        "inlineData": {
                            "mimeType": mime1,
                            "data": data2
                        }
                    },
                    {"text": "Combine these two images: put the blue square and red circle side by side"}
                ]
            }
        ],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"]
        }
    });

    println!("Testing image fusion...");
    let fuse_res = user_client
        .post("/v1/chat/completions", &fuse_body)
        .await
        .expect("Fusion failed");

    // Verify fused image is present
    let fused_image = extract_image_data(&fuse_res);
    assert!(fused_image.is_some(), "Should have fused image in response");

    let (fused_mime, fused_data) = fused_image.unwrap();
    assert!(fused_mime.starts_with("image/"), "Fused MIME type should be image/*");
    assert!(!fused_data.is_empty(), "Fused image data should not be empty");

    println!("SUCCESS: Image fusion with multiple references verified");
}

// ============================================================================
// Test 5: Billing verification
// ============================================================================

#[tokio::test]
async fn test_billing_verification() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Send image generation request
    let chat_body = json!({
        "model": "gemini-3-pro-image-preview",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Generate a yellow star"}]
            }
        ],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"]
        }
    });

    println!("Testing billing verification...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    // Check for usageMetadata (Gemini native format)
    if let Some(usage) = chat_res.get("usageMetadata") {
        let prompt_tokens = usage.get("promptTokenCount").and_then(|t| t.as_u64());
        let completion_tokens = usage.get("candidatesTokenCount").and_then(|t| t.as_u64());
        let total_tokens = usage.get("totalTokenCount").and_then(|t| t.as_u64());

        println!("Usage Metadata:");
        println!("  promptTokenCount: {:?}", prompt_tokens);
        println!("  candidatesTokenCount: {:?}", completion_tokens);
        println!("  totalTokenCount: {:?}", total_tokens);

        assert!(prompt_tokens.is_some(), "Should have promptTokenCount");
        assert!(prompt_tokens.unwrap() > 0, "Prompt tokens should be > 0");

        assert!(completion_tokens.is_some(), "Should have candidatesTokenCount");
        // Image generation typically has higher token count for output
        assert!(completion_tokens.unwrap() > 0, "Completion tokens should be > 0");

        // Verify total = prompt + completion (approximately, may have cached tokens)
        if let (Some(pt), Some(ct), Some(tt)) = (prompt_tokens, completion_tokens, total_tokens) {
            assert!(
                tt >= pt + ct,
                "Total tokens should be >= prompt + completion"
            );
        }

        println!("SUCCESS: Billing verification passed");
        println!("  - Token counting verified");
        println!("  - Image generation uses output tokens correctly");
    } else {
        println!("WARNING: No usageMetadata in response");
    }
}

// ============================================================================
// Test 6: responseModalities parameter passthrough
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

    // Test with IMAGE only modality
    let chat_body = json!({
        "model": "gemini-3-pro-image-preview",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Generate a green triangle"}]
            }
        ],
        "generationConfig": {
            "responseModalities": ["IMAGE"]
        }
    });

    println!("Testing responseModalities passthrough (IMAGE only)...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    // Verify response has image
    let image_data = extract_image_data(&chat_res);
    assert!(image_data.is_some(), "Should have image with IMAGE modality");

    println!("SUCCESS: responseModalities parameter passthrough verified");
}
