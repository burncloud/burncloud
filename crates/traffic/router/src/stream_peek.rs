//! Stream Peek Module
//! 
//! This module provides functionality to peek the first chunk of a streaming response
//! to detect errors before sending the response to the user.

use axum::body::Bytes;
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use std::time::Duration;

/// Type alias for boxed error stream
pub type BoxedErrorStream = Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>;

/// Result of peeking the first chunk from a stream.
pub enum PeekResult {
    /// First chunk was successfully read.
    /// Contains the first chunk and the remaining stream.
    HasFirstChunk {
        first_chunk: Bytes,
        remaining_stream: BoxedErrorStream,
    },
    /// Stream ended immediately (empty response).
    Empty,
    /// Error reading first chunk.
    Error(reqwest::Error),
    /// Timeout waiting for first chunk.
    /// The stream is returned so it can still be used.
    Timeout { stream: BoxedErrorStream },
}

/// Peek the first chunk from a stream with timeout.
pub async fn peek_first_chunk<S>(stream: S, timeout: Duration) -> PeekResult
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
{
    let mut stream = Box::pin(stream);
    
    match tokio::time::timeout(timeout, stream.next()).await {
        Ok(Some(Ok(chunk))) => PeekResult::HasFirstChunk {
            first_chunk: chunk,
            remaining_stream: stream,
        },
        Ok(Some(Err(e))) => PeekResult::Error(e),
        Ok(None) => PeekResult::Empty,
        Err(_) => PeekResult::Timeout { stream },
    }
}

/// Check if a chunk contains an SSE error.
/// Returns Some((error_code, error_message, is_auth_error)) if error found.
pub fn check_sse_error_in_chunk(chunk: &[u8]) -> Option<(u16, String, bool)> {
    let text = String::from_utf8_lossy(chunk);
    
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with("data: ") {
            continue;
        }
        let data = &line[6..];
        if data.trim() == "[DONE]" {
            continue;
        }
        
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
            if let Some(error) = json.get("error") {
                let error_msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown SSE error")
                    .to_string();
                let error_code = error
                    .get("code")
                    .and_then(|c| c.as_u64())
                    .unwrap_or(400) as u16;
                
                // Check if this is an auth error
                let msg_lower = error_msg.to_lowercase();
                let is_auth_error = msg_lower.contains("auth")
                    || msg_lower.contains("appid")
                    || msg_lower.contains("unauthorized")
                    || msg_lower.contains("invalid key")
                    || error_code == 401;
                
                return Some((error_code, error_msg, is_auth_error));
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_sse_error_auth() {
        let chunk = b"data: {\"error\":{\"code\":11200,\"message\":\"AppIdNoAuthError\"}}\n\n";
        let result = check_sse_error_in_chunk(chunk);
        assert!(result.is_some());
        let (code, msg, is_auth) = result.unwrap();
        assert_eq!(code, 11200);
        assert!(msg.contains("AppIdNoAuthError"));
        assert!(is_auth);
    }

    #[test]
    fn test_check_sse_error_normal() {
        let chunk = b"data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n";
        let result = check_sse_error_in_chunk(chunk);
        assert!(result.is_none());
    }
}
