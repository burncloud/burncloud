use crate::types::Config;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn load_config(path: &str) -> Result<Config> {
    if Path::new(path).exists() {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    } else {
        let config = Config::default();
        save_config(path, &config)?;
        Ok(config)
    }
}

pub fn save_config(path: &str, config: &Config) -> Result<()> {
    let content = serde_json::to_string_pretty(config)?;
    fs::write(path, content)?;
    Ok(())
}
