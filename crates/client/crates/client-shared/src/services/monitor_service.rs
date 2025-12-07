use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub usage_percent: f32,
    pub core_count: usize,
    pub frequency: u64,
    pub brand: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub usage_percent: f32,
    pub mount_point: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub timestamp: u64,
}

pub struct MonitorService;

impl MonitorService {
    pub async fn get_system_metrics() -> Result<SystemMetrics, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/monitor", port);

        let client = reqwest::Client::new();
        let resp = client.get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        
        if let Some(data) = json.get("data") {
             serde_json::from_value(data.clone()).map_err(|e| e.to_string())
        } else {
            Err("No data field in response".to_string())
        }
    }
}
