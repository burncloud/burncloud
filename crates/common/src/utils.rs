use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn ensure_dir_exists(path: &str) -> Result<()> {
    if !Path::new(path).exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn get_home_dir() -> String {
    dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .to_string_lossy()
        .to_string()
}
