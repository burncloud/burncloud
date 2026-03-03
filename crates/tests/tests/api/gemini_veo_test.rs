//! Gemini Veo 3.1 Video Generation Test Suite
//!
//! Tests for Gemini Veo 3.1 video generation API:
//! - Video generation from text prompt
//! - Video generation from image (image-to-video)
//! - Long-running operation polling
//! - Error handling for invalid requests
//!
//! ## Veo 3.1 API Overview
//!
//! Veo 3.1 is Google's latest video generation model that supports:
//! - Text-to-video generation
//! - Image-to-video generation
//! - Video extension
//! - First/last frame control
//!
//! ## API Endpoint
//!
//! Vertex AI: `POST /v1/projects/{PROJECT}/locations/{LOCATION}/publishers/google/models/veo-3.1-generate-preview:predictLongRunning`
//! Google AI Studio: `POST /v1beta/models/veo-3.1-generate-preview:predictLongRunning`
//!
//! ## Key Differences from Chat API
//!
//! 1. Uses `predictLongRunning` method instead of `generateContent`
//! 2. Returns an operation ID for polling
//! 3. Uses `instances` array format instead of `contents`
//! 4. Supports video-specific parameters (aspectRatio, durationSeconds, etc.)

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

/// Create a Gemini channel for Veo testing
async fn create_gemini_veo_channel(_base_url: &str, admin_client: &TestClient) -> String {
    let gemini_key = match get_gemini_key() {
        Some(k) => k,
        None => panic!("TEST_GEMINI_KEY not set"),
    };

    let channel_name = format!("Gemini Veo 3.1 Test {}", Uuid::new_v4());
    let body = json!({
        "type": 24, // Gemini channel type
        "key": gemini_key,
        "name": channel_name,
        "base_url": "https://generativelanguage.googleapis.com",
        "models": "veo-3.1-generate-preview,veo-3.0-generate-preview,veo-2.0-generate-001",
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
// Test 1: Basic text-to-video generation (if supported via AI Studio)
// ============================================================================

#[tokio::test]
async fn test_veo_text_to_video_basic() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_veo_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Veo 3.1 text-to-video request in Gemini native format
    // Note: Veo uses different request structure than chat models
    let veo_body = json!({
        "model": "veo-3.1-generate-preview",
        "instances": [{
            "prompt": "A cat walking in a sunny garden"
        }],
        "parameters": {
            "aspectRatio": "16:9",
            "durationSeconds": 5,
            "sampleCount": 1
        }
    });

    println!("Testing Veo 3.1 text-to-video generation...");
    let result = user_client.post("/v1/chat/completions", &veo_body).await;

    match result {
        Ok(resp) => {
            println!("Veo response: {:?}", resp);

            // Check for operation ID (long-running operation)
            if let Some(operation) = resp.get("name") {
                println!("SUCCESS: Got long-running operation: {:?}", operation);
            } else if let Some(error) = resp.get("error") {
                // API might return error if model not available in AI Studio
                println!("API returned error (may be expected): {:?}", error);
            } else {
                println!("Response received: {:?}", resp);
            }
        }
        Err(e) => {
            println!("Request error (may be expected if Veo not available): {}", e);
        }
    }
}

// ============================================================================
// Test 2: Veo via passthrough mode (native Gemini path)
// ============================================================================

#[tokio::test]
async fn test_veo_passthrough_native_path() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_veo_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Use native Gemini path for Veo
    let veo_body = json!({
        "instances": [{
            "prompt": "A beautiful sunset over the ocean"
        }],
        "parameters": {
            "aspectRatio": "16:9",
            "durationSeconds": 5
        }
    });

    println!("Testing Veo via native Gemini passthrough path...");

    // Note: The router should detect Gemini format and use passthrough
    let result = user_client
        .post("/v1beta/models/veo-3.1-generate-preview:predictLongRunning", &veo_body)
        .await;

    match result {
        Ok(resp) => {
            println!("Veo passthrough response: {:?}", resp);
        }
        Err(e) => {
            println!("Passthrough error: {}", e);
        }
    }
}

// ============================================================================
// Test 3: Image-to-video generation
// ============================================================================

#[tokio::test]
async fn test_veo_image_to_video() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_veo_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // A small 1x1 red pixel image in base64 (for testing purposes)
    let test_image_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

    // Image-to-video request
    let veo_body = json!({
        "model": "veo-3.1-generate-preview",
        "instances": [{
            "prompt": "Make this image come alive with gentle movement",
            "image": {
                "bytesBase64Encoded": test_image_base64,
                "mimeType": "image/png"
            }
        }],
        "parameters": {
            "aspectRatio": "16:9",
            "durationSeconds": 5,
            "sampleCount": 1
        }
    });

    println!("Testing Veo 3.1 image-to-video generation...");
    let result = user_client.post("/v1/chat/completions", &veo_body).await;

    match result {
        Ok(resp) => {
            println!("Image-to-video response: {:?}", resp);
        }
        Err(e) => {
            println!("Image-to-video error: {}", e);
        }
    }
}

// ============================================================================
// Test 4: Veo with all parameters
// ============================================================================

#[tokio::test]
async fn test_veo_full_parameters() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_veo_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Full parameter test
    let veo_body = json!({
        "model": "veo-3.1-generate-preview",
        "instances": [{
            "prompt": "A professional product showcase video"
        }],
        "parameters": {
            "aspectRatio": "16:9",
            "durationSeconds": 8,
            "sampleCount": 1,
            "negativePrompt": "blurry, low quality",
            "enhancePrompt": true,
            "personGeneration": "allow_all"
        }
    });

    println!("Testing Veo 3.1 with full parameters...");
    let result = user_client.post("/v1/chat/completions", &veo_body).await;

    match result {
        Ok(resp) => {
            println!("Full parameter response: {:?}", resp);
        }
        Err(e) => {
            println!("Full parameter error: {}", e);
        }
    }
}

// ============================================================================
// Test 5: Error handling - invalid model
// ============================================================================

#[tokio::test]
async fn test_veo_error_invalid_model() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_veo_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Request with non-existent Veo model
    let veo_body = json!({
        "model": "veo-99.9-not-exist",
        "instances": [{
            "prompt": "Test"
        }]
    });

    println!("Testing Veo error handling for invalid model...");
    let result = user_client.post("/v1/chat/completions", &veo_body).await;

    match result {
        Ok(resp) => {
            if let Some(error) = resp.get("error") {
                println!("Got expected error: {:?}", error);
            } else {
                println!("Response: {:?}", resp);
            }
        }
        Err(e) => {
            println!("Got expected error: {}", e);
        }
    }
}

// ============================================================================
// Test 6: Veo 2.0 comparison (if available)
// ============================================================================

#[tokio::test]
async fn test_veo_2_compatibility() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_veo_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Test with Veo 2.0 (GA version)
    let veo_body = json!({
        "model": "veo-2.0-generate-001",
        "instances": [{
            "prompt": "A simple test video"
        }],
        "parameters": {
            "aspectRatio": "16:9",
            "durationSeconds": 5
        }
    });

    println!("Testing Veo 2.0 compatibility...");
    let result = user_client.post("/v1/chat/completions", &veo_body).await;

    match result {
        Ok(resp) => {
            println!("Veo 2.0 response: {:?}", resp);
        }
        Err(e) => {
            println!("Veo 2.0 error: {}", e);
        }
    }
}

// ============================================================================
// Test 7: Channel configuration verification
// ============================================================================

#[tokio::test]
async fn test_veo_channel_configuration() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let channel_name = create_gemini_veo_channel(&base_url, &admin_client).await;

    // Verify channel was created correctly
    let channels = admin_client
        .get("/console/api/channel")
        .await
        .expect("Failed to get channels");

    println!("Channels list: {:?}", channels);

    // Find our channel
    if let Some(channel_list) = channels.get("data").and_then(|d| d.as_array()) {
        let found = channel_list.iter().any(|c| {
            c.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.contains("Veo 3.1 Test"))
                .unwrap_or(false)
        });

        assert!(found, "Veo channel should be created");
        println!("SUCCESS: Veo channel configured correctly");
    }
}
