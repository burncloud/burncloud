use std::{fs, path::Path};

use crate::{
    error::HardwareProbeError,
    types::{AcceleratorKind, AcceleratorProfile},
};
use nvml_wrapper::Nvml;

use super::common::collect_common;

/// 使用 Linux 系统接口读取本机硬件信息。
pub struct SystemHardwareProbe;

impl SystemHardwareProbe {
    /// 创建 Linux 硬件探测器。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::SystemHardwareProbe;
    ///
    /// let _probe = SystemHardwareProbe::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    pub(crate) async fn inspect(
        &self,
    ) -> Result<crate::types::HardwareProfile, HardwareProbeError> {
        tokio::task::spawn_blocking(|| {
            let common = collect_common()?;
            let mut accelerators = detect_nvidia().unwrap_or_default();
            accelerators.extend(detect_drm_accelerators());
            Ok(common.with_accelerators(accelerators))
        })
        .await
        .map_err(|error| HardwareProbeError::DetectionFailed(error.to_string()))?
    }
}

fn detect_nvidia() -> Result<Vec<AcceleratorProfile>, HardwareProbeError> {
    let nvml = Nvml::init()
        .map_err(|error| HardwareProbeError::GpuDriverUnavailable(error.to_string()))?;
    let count = nvml
        .device_count()
        .map_err(|error| HardwareProbeError::GpuDriverUnavailable(error.to_string()))?;

    let mut devices = Vec::new();
    for index in 0..count {
        let Ok(device) = nvml.device_by_index(index) else {
            continue;
        };
        let name = device
            .name()
            .unwrap_or_else(|_| format!("NVIDIA GPU {index}"));
        let memory_bytes = device.memory_info().ok().map(|memory| memory.total);
        devices.push(AcceleratorProfile {
            kind: AcceleratorKind::Nvidia,
            name,
            memory_bytes,
        });
    }
    Ok(devices)
}

fn detect_drm_accelerators() -> Vec<AcceleratorProfile> {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };

    let mut accelerators = Vec::new();
    for entry in entries.flatten() {
        let card_name = entry.file_name().to_string_lossy().into_owned();
        if !is_drm_card(&card_name) {
            continue;
        }

        let device = entry.path().join("device");
        let Some(vendor) = read_vendor_id(&device) else {
            let name =
                read_device_name(&device).unwrap_or_else(|| format!("Unknown GPU ({card_name})"));
            accelerators.push(AcceleratorProfile {
                kind: AcceleratorKind::Other,
                name,
                memory_bytes: read_vram_bytes(&device),
            });
            continue;
        };
        if vendor == "0x10de" {
            continue;
        }
        let kind = classify_vendor(&vendor);
        let name = read_device_name(&device)
            .unwrap_or_else(|| format!("{} GPU ({card_name})", vendor_name(kind)));
        let memory_bytes = read_vram_bytes(&device);
        accelerators.push(AcceleratorProfile {
            kind,
            name,
            memory_bytes,
        });
    }
    accelerators
}

fn is_drm_card(name: &str) -> bool {
    name.strip_prefix("card").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn read_vendor_id(device: &Path) -> Option<String> {
    fs::read_to_string(device.join("vendor"))
        .ok()
        .map(|vendor| vendor.trim().to_ascii_lowercase())
}

fn classify_vendor(vendor: &str) -> AcceleratorKind {
    match vendor {
        "0x1002" => AcceleratorKind::Amd,
        "0x8086" => AcceleratorKind::Intel,
        _ => AcceleratorKind::Other,
    }
}

fn read_device_name(device: &Path) -> Option<String> {
    ["product_name", "label", "model"]
        .into_iter()
        .find_map(|file| fs::read_to_string(device.join(file)).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn read_vram_bytes(device: &Path) -> Option<u64> {
    fs::read_to_string(device.join("mem_info_vram_total"))
        .ok()?
        .trim()
        .parse()
        .ok()
}

const fn vendor_name(kind: AcceleratorKind) -> &'static str {
    match kind {
        AcceleratorKind::Amd => "AMD",
        AcceleratorKind::Intel => "Intel",
        AcceleratorKind::Nvidia => "NVIDIA",
        AcceleratorKind::Apple => "Apple",
        AcceleratorKind::Other => "Unknown",
    }
}

#[cfg(test)]
#[path = "tests/linux.rs"]
mod tests;
