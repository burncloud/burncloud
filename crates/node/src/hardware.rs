use crate::{NodeError, Result};
use burncloud_service_monitor::SystemMonitorService;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::process::Stdio;
use tokio::process::Command;

const BYTES_PER_MB: u64 = 1024 * 1024;

/// Unified hardware description consumed by local model resolution and runtime selection.
///
/// Static identity and dynamic capacity live in one intentionally small structure for Node v0.1.
/// Dynamic values such as available RAM, VRAM and disk space should be refreshed before model
/// preparation rather than treated as permanent startup facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HardwareProfile {
    pub os: String,
    pub cpu_arch: String,
    pub cpu_cores: usize,
    pub cpu_brand: String,
    pub ram_mb: u64,
    pub ram_available_mb: u64,
    pub gpu: Vec<GpuDevice>,
    pub disk_free_mb: u64,
    pub gpu_probe: GpuProbeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GpuDevice {
    pub vendor: String,
    pub model: String,
    pub vram_mb: u64,
    pub vram_available_mb: u64,
    pub driver_version: Option<String>,
}

/// GPU discovery is diagnostic state, not a hard requirement for a valid Node.
/// CPU-only machines therefore still produce a usable HardwareProfile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GpuProbeStatus {
    pub state: GpuProbeState,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GpuProbeState {
    Available,
    NotAvailable,
    Failed,
}

#[derive(Debug, Clone)]
struct GpuProbeResult {
    devices: Vec<GpuDevice>,
    status: GpuProbeStatus,
}

/// Single entry point for BurnCloud Node hardware discovery.
///
/// Existing monitor collectors remain the source for CPU, RAM and disk metrics. GPU discovery is
/// additive and currently probes NVIDIA through nvidia-smi; other vendors can be added without
/// changing HardwareProfile consumers.
pub struct HardwareDetector {
    monitor: SystemMonitorService,
}

impl HardwareDetector {
    pub fn new() -> Self {
        Self {
            monitor: SystemMonitorService::new(),
        }
    }

    pub async fn detect(&self) -> Result<HardwareProfile> {
        let metrics = self
            .monitor
            .refresh_metrics()
            .await
            .map_err(|error| NodeError::SystemMetrics(error.to_string()))?;

        let gpu_probe = probe_nvidia().await;
        let disk_free_bytes = metrics
            .disks
            .iter()
            .map(|disk| disk.available)
            .max()
            .unwrap_or(0);

        Ok(HardwareProfile {
            os: std::env::consts::OS.to_string(),
            cpu_arch: std::env::consts::ARCH.to_string(),
            cpu_cores: metrics.cpu.core_count,
            cpu_brand: metrics.cpu.brand,
            ram_mb: bytes_to_mb(metrics.memory.total),
            ram_available_mb: bytes_to_mb(metrics.memory.available),
            gpu: gpu_probe.devices,
            disk_free_mb: bytes_to_mb(disk_free_bytes),
            gpu_probe: gpu_probe.status,
        })
    }
}

impl Default for HardwareDetector {
    fn default() -> Self {
        Self::new()
    }
}

fn bytes_to_mb(bytes: u64) -> u64 {
    bytes / BYTES_PER_MB
}

async fn probe_nvidia() -> GpuProbeResult {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,memory.free,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return GpuProbeResult {
                devices: Vec::new(),
                status: GpuProbeStatus {
                    state: GpuProbeState::NotAvailable,
                    detail: Some("nvidia-smi was not found".to_string()),
                },
            };
        }
        Err(error) => {
            return GpuProbeResult {
                devices: Vec::new(),
                status: GpuProbeStatus {
                    state: GpuProbeState::Failed,
                    detail: Some(format!("failed to execute nvidia-smi: {error}")),
                },
            };
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            format!("nvidia-smi exited with status {}", output.status)
        } else {
            stderr
        };
        return GpuProbeResult {
            devices: Vec::new(),
            status: GpuProbeStatus {
                state: GpuProbeState::Failed,
                detail: Some(detail),
            },
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    match parse_nvidia_smi_csv(&stdout) {
        Ok(devices) => GpuProbeResult {
            devices,
            status: GpuProbeStatus {
                state: GpuProbeState::Available,
                detail: None,
            },
        },
        Err(error) => GpuProbeResult {
            devices: Vec::new(),
            status: GpuProbeStatus {
                state: GpuProbeState::Failed,
                detail: Some(error),
            },
        },
    }
}

fn parse_nvidia_smi_csv(input: &str) -> std::result::Result<Vec<GpuDevice>, String> {
    let mut devices = Vec::new();

    for (index, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() != 4 {
            return Err(format!(
                "unexpected nvidia-smi row {}: expected 4 fields, got {}",
                index + 1,
                fields.len()
            ));
        }

        let vram_mb = fields[1].parse::<u64>().map_err(|error| {
            format!(
                "invalid total VRAM in nvidia-smi row {}: {error}",
                index + 1
            )
        })?;
        let vram_available_mb = fields[2].parse::<u64>().map_err(|error| {
            format!(
                "invalid free VRAM in nvidia-smi row {}: {error}",
                index + 1
            )
        })?;

        devices.push(GpuDevice {
            vendor: "nvidia".to_string(),
            model: fields[0].to_string(),
            vram_mb,
            vram_available_mb,
            driver_version: Some(fields[3].to_string()),
        });
    }

    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::{bytes_to_mb, parse_nvidia_smi_csv};

    #[test]
    fn parses_multiple_nvidia_devices() {
        let input = "NVIDIA RTX 5090, 32607, 31000, 590.00\nNVIDIA H100, 81559, 80000, 590.00\n";
        let parsed = parse_nvidia_smi_csv(input);

        assert!(parsed.is_ok());
        let devices = match parsed {
            Ok(devices) => devices,
            Err(error) => panic!("unexpected parse failure: {error}"),
        };
        assert_eq!(devices.len(), 2);
        assert_eq!(devices.first().map(|gpu| gpu.model.as_str()), Some("NVIDIA RTX 5090"));
        assert_eq!(devices.first().map(|gpu| gpu.vram_mb), Some(32607));
        assert_eq!(devices.get(1).map(|gpu| gpu.vram_available_mb), Some(80000));
    }

    #[test]
    fn rejects_malformed_nvidia_rows() {
        let parsed = parse_nvidia_smi_csv("NVIDIA RTX 5090, 32607, missing\n");
        assert!(parsed.is_err());
    }

    #[test]
    fn converts_bytes_to_mebibytes() {
        assert_eq!(bytes_to_mb(64 * 1024 * 1024), 64);
    }
}
