use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub downloaded: bool,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub models_dir: String,
    pub server_port: u16,
    pub max_memory: u64,
    pub gpu_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            models_dir: "models".to_string(),
            server_port: 8080,
            max_memory: 8192,
            gpu_enabled: false,
        }
    }
}
