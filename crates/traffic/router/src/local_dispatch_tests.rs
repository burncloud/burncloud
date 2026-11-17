#![allow(clippy::unwrap_used, reason = "测试固定输入与断言失败应中止")]
use super::*;

#[test]
fn request_errors_do_not_lock_but_endpoint_and_structured_model_errors_do() {
    let headers = reqwest::header::HeaderMap::new();
    for status in [400, 422, 409] {
        assert!(classify_local_error(
            StatusCode::from_u16(status).unwrap(),
            Some(&serde_json::json!({"error":{"code":"invalid_request"}})),
            &headers
        )
        .is_none());
    }
    for status in [401, 402, 403, 404, 405, 415, 500, 503, 302] {
        assert!(
            classify_local_error(StatusCode::from_u16(status).unwrap(), None, &headers).is_some()
        );
    }
    assert!(matches!(
        classify_local_error(
            StatusCode::BAD_REQUEST,
            Some(&serde_json::json!({"error":{"code":"model_not_found"}})),
            &headers
        ),
        Some(FailureType::ModelNotFound)
    ));
    assert!(matches!(
        classify_local_error(StatusCode::TOO_MANY_REQUESTS, None, &headers),
        Some(FailureType::RateLimited { .. })
    ));
}

#[test]
fn completion_accepts_tool_calls_and_refusal_without_usage_but_not_invalid_json_shape() {
    assert!(valid_completion(
        &serde_json::json!({"choices":[{"message":{"tool_calls":[{"id":"a"}]}}]})
    ));
    assert!(valid_completion(
        &serde_json::json!({"choices":[{"message":{"refusal":"no"}}]})
    ));
    for invalid in [
        serde_json::json!({}),
        serde_json::json!({"choices":[]}),
        serde_json::json!({"choices":[{"message":null}]}),
    ] {
        assert!(!valid_completion(&invalid));
    }
}

#[test]
fn sse_handles_every_chunk_split_and_error_after_text() {
    let content = b"data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\r\n\r\ndata: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
    for split in 0..=content.len() {
        let mut decoder = SseDecoder::default();
        let counter = UnifiedTokenCounter::new();
        decoder.push(&content[..split], &counter).unwrap();
        decoder.push(&content[split..], &counter).unwrap();
        decoder.finish(&counter).unwrap();
    }
    let mut decoder = SseDecoder::default();
    let counter = UnifiedTokenCounter::new();
    decoder
        .push(
            b"data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: {\"err",
            &counter,
        )
        .unwrap();
    assert!(decoder
        .push(b"or\":{\"message\":\"failed\"}}\n\n", &counter)
        .is_err());
}

#[test]
fn sse_tool_refusal_and_utf8_chunks_work_and_premature_eof_fails() {
    let mut decoder = SseDecoder::default();
    let counter = UnifiedTokenCounter::new();
    for byte in
        "data: {\"choices\":[{\"delta\":{\"refusal\":\"拒绝\"},\"finish_reason\":\"stop\"}]}\n\n"
            .as_bytes()
    {
        decoder.push(&[*byte], &counter).unwrap();
    }
    decoder.finish(&counter).unwrap();
    let mut decoder = SseDecoder::default();
    decoder
        .push(b"data: {\"choices\":[{\"delta\":{}}]}\n\n", &counter)
        .unwrap();
    assert!(decoder.finish(&counter).is_err());
    assert!(SseDecoder::default().finish(&counter).is_err());
}
