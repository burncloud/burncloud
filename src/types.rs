use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemStatus {
    pub cpu_usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub gpu_memory_used: u64,
    pub gpu_memory_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub temperature: f32,
    pub active_models: u32,
}

impl Default for SystemStatus {
    fn default() -> Self {
        Self {
            cpu_usage: 45.2,
            memory_used: 8_589_934_592, // 8GB
            memory_total: 17_179_869_184, // 16GB
            gpu_memory_used: 7_516_192_768, // 7.2GB
            gpu_memory_total: 12_884_901_888, // 12GB
            disk_used: 167_503_724_544, // 156GB
            disk_total: 536_870_912_000, // 500GB
            temperature: 52.0,
            active_models: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub size: u64,
    pub status: ModelStatus,
    pub port: Option<u16>,
    pub memory_usage: Option<u64>,
    pub requests_count: u64,
    pub avg_response_time: f32,
    pub description: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeployConfig {
    pub model_id: String,
    pub port: u16,
    pub bind_address: String,
    pub api_key: String,
    pub max_concurrent: u32,
    pub memory_limit: u64,
    pub cpu_cores: u32,
    pub quantization: QuantizationLevel,
    pub context_length: u32,
    pub temperature: f32,
    pub log_level: LogLevel,
    pub enable_gpu: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuantizationLevel {
    FP16,
    INT8,
    INT4,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: Theme,
    pub language: String,
    pub font_size: FontSize,
    pub auto_start: bool,
    pub minimize_to_tray: bool,
    pub auto_update: bool,
    pub send_analytics: bool,
    pub data_directory: String,
    pub api_rate_limit: u32,
    pub allow_remote_access: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FontSize {
    Small,
    Medium,
    Large,
}