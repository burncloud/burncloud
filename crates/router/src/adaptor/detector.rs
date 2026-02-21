use burncloud_database::{sqlx, Database};
use regex::Regex;
use std::sync::Arc;

/// Detector for API version deprecation errors from upstream providers
///
/// This detector parses error responses from upstream providers to detect
/// when an API version has been deprecated and a new version is available.
/// It can automatically update the channel's API version configuration.
pub struct ApiVersionDetector {
    db: Arc<Database>,
}

impl ApiVersionDetector {
    /// Create a new ApiVersionDetector with database connection
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Parse deprecation error message to extract new API version
    ///
    /// Supports formats from various providers:
    /// - Azure OpenAI: "This model version 'old-version' has been deprecated. Please use 'new-version' instead."
    /// - OpenAI: "The model 'old' has been deprecated, use 'new' instead"
    /// - Generic: "deprecated... use 'new-version'"
    pub fn parse_deprecation_error(error_message: &str) -> Option<String> {
        // Pattern 1: "Please use 'new-version' instead" or "use 'new-version' instead"
        // This is the most specific pattern for recommended replacement
        let re1 = Regex::new(r#"use\s+['"]([a-zA-Z0-9_.-]+)['"]\s+instead"#).ok()?;
        if let Some(caps) = re1.captures(error_message) {
            if let Some(new_version) = caps.get(1) {
                return Some(new_version.as_str().to_string());
            }
        }

        // Pattern 2: "Please use version 'new-version'" or "use version 'new-version'"
        let re2 = Regex::new(r#"use\s+(?:version\s+)?['"]([a-zA-Z0-9_.-]+)['"]"#).ok()?;
        if let Some(caps) = re2.captures(error_message) {
            if let Some(new_version) = caps.get(1) {
                return Some(new_version.as_str().to_string());
            }
        }

        // Pattern 3: Look for quoted string after "deprecated" keyword
        // This handles cases like "X is deprecated. Use 'Y'" but we need to skip the old version
        if error_message.to_lowercase().contains("deprecated") {
            // Find all quoted strings and return the last one (which is usually the new version)
            let re3 = Regex::new(r#"['"]([a-zA-Z0-9_.-]+)['"]"#).ok()?;
            let captures: Vec<_> = re3.captures_iter(error_message).collect();
            if captures.len() >= 2 {
                // Return the last captured group (new version)
                if let Some(last) = captures.last() {
                    if let Some(new_version) = last.get(1) {
                        return Some(new_version.as_str().to_string());
                    }
                }
            }
        }

        None
    }

    /// Check if an error message indicates API version deprecation
    pub fn is_deprecation_error(error_message: &str) -> bool {
        let lower = error_message.to_lowercase();
        lower.contains("deprecated")
            || lower.contains("no longer available")
            || lower.contains("has been retired")
    }

    /// Detect and update API version for a channel based on deprecation error
    ///
    /// Returns Some(new_version) if a new version was detected and updated,
    /// None if no deprecation was detected or no new version could be extracted
    pub async fn detect_and_update(
        &self,
        channel_id: i32,
        error_message: &str,
        adaptor_factory: &super::factory::DynamicAdaptorFactory,
    ) -> anyhow::Result<Option<String>> {
        // Check if this is a deprecation error
        if !Self::is_deprecation_error(error_message) {
            return Ok(None);
        }

        // Try to extract new version
        let new_version = match Self::parse_deprecation_error(error_message) {
            Some(v) => v,
            None => return Ok(None),
        };

        // Update channel's api_version in database
        self.update_channel_api_version(channel_id, &new_version)
            .await?;

        // Invalidate adaptor cache for this channel
        // Note: We invalidate all cached adaptors for this channel type
        // In a more sophisticated implementation, we'd track the channel_type
        adaptor_factory.clear_cache();

        Ok(Some(new_version))
    }

    /// Update the API version for a channel in the database
    async fn update_channel_api_version(
        &self,
        channel_id: i32,
        new_version: &str,
    ) -> anyhow::Result<()> {
        let conn = self.db.get_connection()?;
        let db_kind = self.db.kind();

        // Use appropriate syntax for the database type
        let sql = match db_kind.as_str() {
            "sqlite" => "UPDATE channels SET api_version = ? WHERE id = ?",
            "postgres" => "UPDATE channels SET api_version = $1 WHERE id = $2",
            _ => "UPDATE channels SET api_version = ? WHERE id = ?",
        };

        sqlx::query(sql)
            .bind(new_version)
            .bind(channel_id)
            .execute(conn.pool())
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_deprecation_error_azure_style() {
        let error = "This model version 'gpt-35-turbo' has been deprecated. Please use 'gpt-35-turbo-0125' instead.";
        let result = ApiVersionDetector::parse_deprecation_error(error);
        assert_eq!(result, Some("gpt-35-turbo-0125".to_string()));
    }

    #[test]
    fn test_parse_deprecation_error_openai_style() {
        let error = "The model 'gpt-4-0314' has been deprecated. Use 'gpt-4-turbo' instead.";
        let result = ApiVersionDetector::parse_deprecation_error(error);
        assert_eq!(result, Some("gpt-4-turbo".to_string()));
    }

    #[test]
    fn test_parse_deprecation_error_version_keyword() {
        let error = "Model version deprecated. Please use version '2024-02-15-preview'.";
        let result = ApiVersionDetector::parse_deprecation_error(error);
        assert_eq!(result, Some("2024-02-15-preview".to_string()));
    }

    #[test]
    fn test_parse_deprecation_error_no_match() {
        let error = "Internal server error";
        let result = ApiVersionDetector::parse_deprecation_error(error);
        assert_eq!(result, None);
    }

    #[test]
    fn test_is_deprecation_error() {
        assert!(ApiVersionDetector::is_deprecation_error(
            "This model has been deprecated"
        ));
        assert!(ApiVersionDetector::is_deprecation_error(
            "Model no longer available"
        ));
        assert!(ApiVersionDetector::is_deprecation_error(
            "Model has been retired"
        ));
        assert!(!ApiVersionDetector::is_deprecation_error(
            "Internal server error"
        ));
        assert!(!ApiVersionDetector::is_deprecation_error(
            "Rate limit exceeded"
        ));
    }
}
