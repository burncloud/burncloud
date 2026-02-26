//! Protocol Mapping Module
//!
//! This module provides mapping utilities for protocol adaptation:
//! - Request mapping: Transform request fields between protocols
//! - Response mapping: Extract content from various response formats

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Request mapping configuration for protocol adaptation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestMapping {
    /// Field mappings: "target_field" => "source_field"
    /// Example: {"input": "messages"} means copy "messages" field to "input"
    #[serde(default)]
    pub field_map: HashMap<String, String>,

    /// Field renames: "old_name" => "new_name"
    /// Example: {"messages": "input"} means rename "messages" to "input"
    #[serde(default)]
    pub rename: HashMap<String, String>,

    /// Fields to add to the request
    /// Example: {"api-version": "2025-01-01"}
    #[serde(default)]
    pub add_fields: HashMap<String, Value>,
}

impl RequestMapping {
    /// Create an empty request mapping
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a request mapping with field map
    #[allow(dead_code)]
    pub fn with_field_map(field_map: HashMap<String, String>) -> Self {
        Self {
            field_map,
            ..Default::default()
        }
    }

    /// Add a field mapping
    #[allow(dead_code)]
    pub fn add_field_mapping(
        mut self,
        target: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        self.field_map.insert(target.into(), source.into());
        self
    }

    /// Add a field rename
    #[allow(dead_code)]
    pub fn add_rename(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.rename.insert(from.into(), to.into());
        self
    }

    /// Add a fixed field
    #[allow(dead_code)]
    pub fn add_fixed_field(mut self, key: impl Into<String>, value: Value) -> Self {
        self.add_fields.insert(key.into(), value);
        self
    }
}

/// Response mapping configuration for protocol adaptation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseMapping {
    /// Path to extract content from response
    /// Example: "choices[0].message.content" or "output.text"
    pub content_path: Option<String>,

    /// Path to extract token usage
    /// Example: "usage" or "usage.total_tokens"
    pub usage_path: Option<String>,

    /// Path to extract error message
    /// Example: "error.message" or "error.error.message"
    pub error_path: Option<String>,
}

impl ResponseMapping {
    /// Create an empty response mapping
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a response mapping with content path
    #[allow(dead_code)]
    pub fn with_content_path(path: impl Into<String>) -> Self {
        Self {
            content_path: Some(path.into()),
            ..Default::default()
        }
    }

    /// Set content path
    #[allow(dead_code)]
    pub fn content_path(mut self, path: impl Into<String>) -> Self {
        self.content_path = Some(path.into());
        self
    }

    /// Set usage path
    #[allow(dead_code)]
    pub fn usage_path(mut self, path: impl Into<String>) -> Self {
        self.usage_path = Some(path.into());
        self
    }

    /// Set error path
    #[allow(dead_code)]
    pub fn error_path(mut self, path: impl Into<String>) -> Self {
        self.error_path = Some(path.into());
        self
    }
}

/// Apply request mapping rules to a JSON value
///
/// This function transforms a request JSON according to the mapping rules:
/// 1. Apply field mappings (copy fields from source to target)
/// 2. Apply field renames
/// 3. Add fixed fields
pub fn apply_mapping(json: &mut Value, mapping: &RequestMapping) {
    if !mapping.field_map.is_empty() || !mapping.rename.is_empty() || !mapping.add_fields.is_empty()
    {
        if let Some(obj) = json.as_object_mut() {
            // 1. Apply field mappings (copy from source to target)
            for (target, source) in &mapping.field_map {
                if let Some(value) = obj.get(source).cloned() {
                    obj.insert(target.clone(), value);
                }
            }

            // 2. Apply field renames
            // Collect renames to apply (avoid modifying while iterating)
            let renames: Vec<(String, String)> = mapping
                .rename
                .iter()
                .filter_map(|(from, to)| {
                    if obj.contains_key(from) && !obj.contains_key(to) {
                        Some((from.clone(), to.clone()))
                    } else {
                        None
                    }
                })
                .collect();

            for (from, to) in renames {
                if let Some(value) = obj.remove(&from) {
                    obj.insert(to, value);
                }
            }

            // 3. Add fixed fields
            for (key, value) in &mapping.add_fields {
                obj.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Extract a value from JSON using a path expression
///
/// Supports:
/// - Simple field access: "field"
/// - Nested access: "object.field"
/// - Array index access: "array[0]"
/// - Combined: "choices[0].message.content"
pub fn extract_value<'a>(json: &'a Value, path: &str) -> Option<&'a Value> {
    let parts = parse_path(path);
    let mut current = json;

    for part in parts {
        current = match part {
            PathPart::Key(key) => current.get(key)?,
            PathPart::Index(idx) => current.get(idx)?,
        };
    }

    Some(current)
}

/// Extract a value from JSON using a path expression (owned version)
#[allow(dead_code)]
pub fn extract_value_owned(json: &Value, path: &str) -> Option<Value> {
    extract_value(json, path).cloned()
}

/// Parse a path expression into components
fn parse_path(path: &str) -> Vec<PathPart> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '.' => {
                if !current.is_empty() {
                    parts.push(PathPart::Key(current.clone()));
                    current.clear();
                }
            }
            '[' => {
                if !current.is_empty() {
                    parts.push(PathPart::Key(current.clone()));
                    current.clear();
                }
                // Read until ]
                let mut idx_str = String::new();
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                    idx_str.push(c);
                }
                if let Ok(idx) = idx_str.parse::<usize>() {
                    parts.push(PathPart::Index(idx));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        parts.push(PathPart::Key(current));
    }

    parts
}

#[derive(Debug, Clone)]
enum PathPart {
    Key(String),
    Index(usize),
}

/// Extract usage information from a response
#[allow(dead_code)]
pub fn extract_usage(json: &Value, usage_path: &str) -> Option<Value> {
    extract_value_owned(json, usage_path)
}

/// Extract error message from a response
#[allow(dead_code)]
pub fn extract_error(json: &Value, error_path: &str) -> Option<String> {
    extract_value(json, error_path)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_mapping_default() {
        let mapping = RequestMapping::new();
        assert!(mapping.field_map.is_empty());
        assert!(mapping.rename.is_empty());
        assert!(mapping.add_fields.is_empty());
    }

    #[test]
    fn test_request_mapping_builders() {
        let mapping = RequestMapping::new()
            .add_field_mapping("input", "messages")
            .add_rename("model", "deployment_id")
            .add_fixed_field("api-version", json!("2025-01-01"));

        assert_eq!(
            mapping.field_map.get("input"),
            Some(&"messages".to_string())
        );
        assert_eq!(
            mapping.rename.get("model"),
            Some(&"deployment_id".to_string())
        );
        assert_eq!(
            mapping.add_fields.get("api-version"),
            Some(&json!("2025-01-01"))
        );
    }

    #[test]
    fn test_response_mapping_default() {
        let mapping = ResponseMapping::new();
        assert!(mapping.content_path.is_none());
        assert!(mapping.usage_path.is_none());
        assert!(mapping.error_path.is_none());
    }

    #[test]
    fn test_response_mapping_builders() {
        let mapping = ResponseMapping::new()
            .content_path("choices[0].message.content")
            .usage_path("usage")
            .error_path("error.message");

        assert_eq!(
            mapping.content_path,
            Some("choices[0].message.content".to_string())
        );
        assert_eq!(mapping.usage_path, Some("usage".to_string()));
        assert_eq!(mapping.error_path, Some("error.message".to_string()));
    }

    #[test]
    fn test_apply_mapping_field_copy() {
        let mut json = json!({
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let mapping = RequestMapping::new().add_field_mapping("input", "messages");

        apply_mapping(&mut json, &mapping);

        assert!(json.get("input").is_some());
        assert!(json.get("messages").is_some()); // Original is kept
    }

    #[test]
    fn test_apply_mapping_rename() {
        let mut json = json!({
            "model": "gpt-4"
        });

        let mapping = RequestMapping::new().add_rename("model", "deployment_id");

        apply_mapping(&mut json, &mapping);

        assert!(json.get("deployment_id").is_some());
        assert!(json.get("model").is_none()); // Renamed away
        assert_eq!(json.get("deployment_id"), Some(&json!("gpt-4")));
    }

    #[test]
    fn test_apply_mapping_add_fields() {
        let mut json = json!({
            "messages": []
        });

        let mapping = RequestMapping::new()
            .add_fixed_field("api-version", json!("2025-01-01"))
            .add_fixed_field("stream", json!(true));

        apply_mapping(&mut json, &mapping);

        assert_eq!(json.get("api-version"), Some(&json!("2025-01-01")));
        assert_eq!(json.get("stream"), Some(&json!(true)));
    }

    #[test]
    fn test_extract_value_simple() {
        let json = json!({
            "message": "hello"
        });

        let value = extract_value(&json, "message");
        assert_eq!(value, Some(&json!("hello")));
    }

    #[test]
    fn test_extract_value_nested() {
        let json = json!({
            "response": {
                "content": "world"
            }
        });

        let value = extract_value(&json, "response.content");
        assert_eq!(value, Some(&json!("world")));
    }

    #[test]
    fn test_extract_value_array_index() {
        let json = json!({
            "choices": [
                {"message": {"content": "first"}},
                {"message": {"content": "second"}}
            ]
        });

        let value = extract_value(&json, "choices[0].message.content");
        assert_eq!(value, Some(&json!("first")));

        let value = extract_value(&json, "choices[1].message.content");
        assert_eq!(value, Some(&json!("second")));
    }

    #[test]
    fn test_extract_value_not_found() {
        let json = json!({
            "other": "value"
        });

        let value = extract_value(&json, "missing");
        assert!(value.is_none());
    }

    #[test]
    fn test_extract_usage() {
        let json = json!({
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        });

        let usage = extract_usage(&json, "usage");
        assert!(usage.is_some());
        assert_eq!(usage.unwrap()["total_tokens"], 30);
    }

    #[test]
    fn test_extract_error() {
        let json = json!({
            "error": {
                "message": "Rate limit exceeded",
                "type": "rate_limit_error"
            }
        });

        let error = extract_error(&json, "error.message");
        assert_eq!(error, Some("Rate limit exceeded".to_string()));
    }

    #[test]
    fn test_request_mapping_serialization() {
        let mapping = RequestMapping::new()
            .add_field_mapping("input", "messages")
            .add_rename("model", "deployment_id");

        let json = serde_json::to_string(&mapping).unwrap();
        let parsed: RequestMapping = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.field_map.get("input"), Some(&"messages".to_string()));
        assert_eq!(
            parsed.rename.get("model"),
            Some(&"deployment_id".to_string())
        );
    }

    #[test]
    fn test_response_mapping_serialization() {
        let mapping = ResponseMapping::new().content_path("choices[0].message.content");

        let json = serde_json::to_string(&mapping).unwrap();
        let parsed: ResponseMapping = serde_json::from_str(&json).unwrap();

        assert_eq!(
            parsed.content_path,
            Some("choices[0].message.content".to_string())
        );
    }
}
