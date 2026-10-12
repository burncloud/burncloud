use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) enum AuthType {
    Bearer,         // Authorization: Bearer <key>
    Header(String), // <custom-header>: <key>
    Query(String),  // ?<param>=<key>
    AwsSigV4,       // AWS Signature Version 4
    Azure,          // Azure OpenAI (api-key header)
    GoogleAI,       // Google AI Studio (x-goog-api-key header)
    Vertex,         // Google Vertex AI (Bearer token, usually short-lived)
    DeepSeek,       // DeepSeek API (Bearer token)
    Qwen,           // Alibaba Cloud Qwen (Bearer token)
    Claude,         // Anthropic Claude (x-api-key header)
}

impl From<&str> for AuthType {
    fn from(s: &str) -> Self {
        match s {
            "Bearer" => AuthType::Bearer,
            "XApiKey" => AuthType::Header("x-api-key".to_string()), // Alias for backward compatibility
            "AwsSigV4" => AuthType::AwsSigV4,
            "Azure" => AuthType::Azure,
            "GoogleAI" => AuthType::GoogleAI,
            "Vertex" => AuthType::Vertex,
            "DeepSeek" => AuthType::DeepSeek,
            "Qwen" => AuthType::Qwen,
            "Claude" => AuthType::Claude,
            s if s.starts_with("Header:") => {
                let header_name = s.trim_start_matches("Header:").trim();
                AuthType::Header(header_name.to_string())
            }
            s if s.starts_with("Query:") => {
                let param = s.trim_start_matches("Query:").trim();
                AuthType::Query(param.to_string())
            }
            _ => AuthType::Bearer, // Default
        }
    }
}

/// Internal representation of an upstream channel for request routing.
/// Populated from `Channel` objects returned by `ModelRouter`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Upstream {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub match_path: String,
    pub auth_type: AuthType,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub param_override: Option<String>,
    #[serde(default)]
    pub header_override: Option<String>,
    #[serde(default)]
    pub api_version: Option<String>,
    #[serde(default)]
    pub pricing_region: Option<String>,
}

#[cfg(test)]
mod tests {
    //! Contract for the provider auth-scheme parser.
    //!
    //! `AuthType::from` decides where a stored credential is placed on the outgoing request: a
    //! bearer header, a named header, or a query parameter. That makes an unrecognised string a
    //! security-relevant fallback, and the crate had no test for any branch of it.
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "unit tests: a failed assertion is the intended failure signal, and the workspace denies unwrap_used even in test code"
    )]
    use super::*;

    #[test]
    fn the_canonical_spellings_map_to_their_schemes() {
        assert_eq!(AuthType::from("Bearer"), AuthType::Bearer);
        assert_eq!(AuthType::from("AwsSigV4"), AuthType::AwsSigV4);
        assert_eq!(AuthType::from("Azure"), AuthType::Azure);
        assert_eq!(AuthType::from("GoogleAI"), AuthType::GoogleAI);
        assert_eq!(AuthType::from("Vertex"), AuthType::Vertex);
        assert_eq!(AuthType::from("DeepSeek"), AuthType::DeepSeek);
        assert_eq!(AuthType::from("Qwen"), AuthType::Qwen);
        assert_eq!(AuthType::from("Claude"), AuthType::Claude);
    }

    /// `XApiKey` is a backward-compatibility alias, not a scheme of its own: it must keep
    /// resolving to the same named header as a client that sends `Header:x-api-key`. If it ever
    /// stopped, existing channels would start sending credentials in the wrong place.
    #[test]
    fn the_x_api_key_alias_agrees_with_the_explicit_header_form() {
        assert_eq!(
            AuthType::from("XApiKey"),
            AuthType::from("Header:x-api-key")
        );
        assert_eq!(
            AuthType::from("XApiKey"),
            AuthType::Header("x-api-key".to_string())
        );
    }

    /// The `Header:` / `Query:` prefixes carry a caller-supplied name, and surrounding
    /// whitespace must not leak into it: a header name with a trailing space is a *different*
    /// header, and the credential would then be sent where the upstream is not looking.
    #[test]
    fn prefixed_forms_capture_the_name_and_trim_whitespace() {
        assert_eq!(
            AuthType::from("Header:x-custom-key"),
            AuthType::Header("x-custom-key".to_string())
        );
        assert_eq!(
            AuthType::from("Header:  x-custom-key  "),
            AuthType::Header("x-custom-key".to_string())
        );
        assert_eq!(
            AuthType::from("Query:api_key"),
            AuthType::Query("api_key".to_string())
        );
        assert_eq!(
            AuthType::from("Query:   api_key   "),
            AuthType::Query("api_key".to_string())
        );
    }

    /// An empty name after the prefix is preserved rather than rejected. Pinning this makes the
    /// tolerance explicit: it is pre-existing behaviour, so a future change to reject empty
    /// names has to update this test deliberately instead of altering it silently.
    #[test]
    fn a_prefix_with_an_empty_name_is_preserved_as_empty() {
        assert_eq!(AuthType::from("Header:"), AuthType::Header(String::new()));
        assert_eq!(AuthType::from("Query:"), AuthType::Query(String::new()));
        assert_eq!(
            AuthType::from("Header:   "),
            AuthType::Header(String::new())
        );
    }

    /// An unrecognised string falls back to `Bearer`. This is the risky branch: a typo in a
    /// stored channel credential scheme silently becomes a bearer token instead of failing.
    /// The behaviour is pinned here so it is a visible, deliberate contract rather than an
    /// accident, and changing it requires changing this test.
    #[test]
    fn an_unrecognised_scheme_falls_back_to_bearer() {
        assert_eq!(AuthType::from(""), AuthType::Bearer);
        assert_eq!(AuthType::from("nonsense"), AuthType::Bearer);
        assert_eq!(AuthType::from("bearer"), AuthType::Bearer);
        assert_eq!(AuthType::from("BEARER"), AuthType::Bearer);
        assert_eq!(AuthType::from("AzureOpenAI"), AuthType::Bearer);
        assert_eq!(AuthType::from("Header"), AuthType::Bearer);
        assert_eq!(AuthType::from("Query"), AuthType::Bearer);
    }

    /// Matching is exact and case-sensitive, so a differently-cased spelling does not reach the
    /// scheme-specific path; it lands on the `Bearer` fallback instead.
    #[test]
    fn scheme_matching_is_case_sensitive() {
        for misspelled in [
            "awssigv4", "AWSsigV4", "azure", "googleai", "claude", "vertex",
        ] {
            assert_eq!(
                AuthType::from(misspelled),
                AuthType::Bearer,
                "{misspelled} must not match a scheme case-insensitively"
            );
        }
        // Only the exact spelling reaches the scheme.
        assert_eq!(AuthType::from("Claude"), AuthType::Claude);
    }

    /// The parse-then-serialise round trip is the wire contract for a stored channel, so a
    /// scheme that cannot survive it would be silently rewritten the next time it is loaded.
    #[test]
    fn a_parsed_scheme_survives_a_serde_round_trip() {
        for raw in [
            "Bearer",
            "XApiKey",
            "AwsSigV4",
            "Azure",
            "GoogleAI",
            "Vertex",
            "DeepSeek",
            "Qwen",
            "Claude",
            "Header:x-custom-key",
            "Query:api_key",
        ] {
            let parsed = AuthType::from(raw);
            let json = serde_json::to_string(&parsed).expect("AuthType must serialise");
            let back: AuthType = serde_json::from_str(&json).expect("AuthType must deserialise");
            assert_eq!(back, parsed, "{raw} did not survive a serde round trip");
        }
    }
}
