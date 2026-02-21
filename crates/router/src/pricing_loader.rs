//! Pricing configuration loader module.
//!
//! This module provides functionality for loading pricing configuration from local files.
//! Supports both main configuration and override files for flexible deployment.

use std::path::{Path, PathBuf};

use burncloud_common::{PricingConfig, ValidationWarning};
use thiserror::Error;

/// Default paths for pricing configuration files
pub const DEFAULT_CONFIG_PATH: &str = "config/pricing.json";
pub const DEFAULT_OVERRIDE_PATH: &str = "config/pricing.override.json";

/// Errors that can occur when loading pricing configuration
#[derive(Debug, Error)]
pub enum PricingLoaderError {
    #[error("IO error reading {path}: {source}")]
    IoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("JSON parse error in {path}: {source}")]
    JsonParseError {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Configuration validation failed: {0}")]
    ValidationError(String),
}

/// Configuration for the pricing loader
#[derive(Debug, Clone)]
pub struct PricingLoaderConfig {
    /// Path to the main pricing configuration file
    pub config_path: PathBuf,
    /// Path to the override configuration file (higher priority)
    pub override_path: PathBuf,
}

impl Default for PricingLoaderConfig {
    fn default() -> Self {
        Self {
            config_path: PathBuf::from(DEFAULT_CONFIG_PATH),
            override_path: PathBuf::from(DEFAULT_OVERRIDE_PATH),
        }
    }
}

/// Service for loading pricing configuration from files
pub struct PricingLoader {
    config: PricingLoaderConfig,
}

impl PricingLoader {
    /// Create a new PricingLoader with default paths
    pub fn new() -> Self {
        Self {
            config: PricingLoaderConfig::default(),
        }
    }

    /// Create a new PricingLoader with custom paths
    pub fn with_config(config: PricingLoaderConfig) -> Self {
        Self { config }
    }

    /// Load pricing configuration from the override file if it exists
    /// This has the highest priority and is intended for local development
    pub fn load_local_override(&self) -> Result<Option<PricingConfig>, PricingLoaderError> {
        self.load_from_path(&self.config.override_path)
    }

    /// Load pricing configuration from the main config file if it exists
    pub fn load_local_config(&self) -> Result<Option<PricingConfig>, PricingLoaderError> {
        self.load_from_path(&self.config.config_path)
    }

    /// Load pricing configuration from a specific path
    pub fn load_from_path(
        &self,
        path: &Path,
    ) -> Result<Option<PricingConfig>, PricingLoaderError> {
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(path).map_err(|e| PricingLoaderError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let config: PricingConfig =
            serde_json::from_str(&content).map_err(|e| PricingLoaderError::JsonParseError {
                path: path.to_path_buf(),
                source: e,
            })?;

        Ok(Some(config))
    }

    /// Validate a pricing configuration
    pub fn validate_config(
        &self,
        config: &PricingConfig,
    ) -> Result<Vec<ValidationWarning>, PricingLoaderError> {
        config.validate().map_err(|e| {
            PricingLoaderError::ValidationError(format!("Configuration validation failed: {}", e))
        })
    }

    /// Load configuration with priority: override > main > None
    pub fn load_with_priority(&self) -> Result<Option<PricingConfig>, PricingLoaderError> {
        // First try the override file (highest priority)
        if let Some(config) = self.load_local_override()? {
            println!("Loaded pricing configuration from override file");
            return Ok(Some(config));
        }

        // Then try the main config file
        if let Some(config) = self.load_local_config()? {
            println!("Loaded pricing configuration from main file");
            return Ok(Some(config));
        }

        // No configuration found
        Ok(None)
    }

    /// Get the path to the main configuration file
    pub fn config_path(&self) -> &Path {
        &self.config.config_path
    }

    /// Get the path to the override configuration file
    pub fn override_path(&self) -> &Path {
        &self.config.override_path
    }
}

impl Default for PricingLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_nonexistent_file() {
        let loader = PricingLoader::new();
        let result = loader.load_from_path(Path::new("/nonexistent/path.json")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_valid_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("pricing.json");

        let config_content = r#"{
            "version": "1.0",
            "updated_at": "2024-01-15T10:00:00Z",
            "source": "test",
            "models": {}
        }"#;

        std::fs::write(&config_path, config_content).unwrap();

        let loader_config = PricingLoaderConfig {
            config_path: config_path.clone(),
            override_path: dir.path().join("override.json"),
        };
        let loader = PricingLoader::with_config(loader_config);

        let result = loader.load_local_config().unwrap();
        assert!(result.is_some());

        let config = result.unwrap();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.source, "test");
    }

    #[test]
    fn test_load_override_priority() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("pricing.json");
        let override_path = dir.path().join("pricing.override.json");

        // Write main config
        let main_content = r#"{
            "version": "1.0",
            "updated_at": "2024-01-15T10:00:00Z",
            "source": "main",
            "models": {}
        }"#;
        std::fs::write(&config_path, main_content).unwrap();

        // Write override config
        let override_content = r#"{
            "version": "1.0",
            "updated_at": "2024-01-16T10:00:00Z",
            "source": "override",
            "models": {}
        }"#;
        std::fs::write(&override_path, override_content).unwrap();

        let loader_config = PricingLoaderConfig {
            config_path,
            override_path,
        };
        let loader = PricingLoader::with_config(loader_config);

        let result = loader.load_with_priority().unwrap();
        assert!(result.is_some());

        let config = result.unwrap();
        // Override should have priority
        assert_eq!(config.source, "override");
    }

    #[test]
    fn test_validate_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("pricing.json");

        // Invalid config with negative price
        let invalid_content = r#"{
            "version": "1.0",
            "updated_at": "2024-01-15T10:00:00Z",
            "source": "test",
            "models": {
                "test-model": {
                    "pricing": {
                        "USD": {
                            "input_price": -10.0,
                            "output_price": 30.0
                        }
                    }
                }
            }
        }"#;
        std::fs::write(&config_path, invalid_content).unwrap();

        let loader_config = PricingLoaderConfig {
            config_path,
            override_path: dir.path().join("override.json"),
        };
        let loader = PricingLoader::with_config(loader_config);

        let config = loader.load_local_config().unwrap().unwrap();
        let result = loader.validate_config(&config);

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_json() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("pricing.json");

        std::fs::write(&config_path, "not valid json").unwrap();

        let loader_config = PricingLoaderConfig {
            config_path,
            override_path: dir.path().join("override.json"),
        };
        let loader = PricingLoader::with_config(loader_config);

        let result = loader.load_local_config();
        assert!(result.is_err());
    }
}
