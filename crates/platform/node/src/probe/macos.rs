#![expect(
    clippy::disallowed_types,
    reason = "此模块是 system_profiler 的 JSON 协议适配边界，需要处理平台返回的可变字段和结构。"
)]

use std::process::Command;

use serde_json::Value;

use crate::{
    error::HardwareProbeError,
    types::{AcceleratorKind, AcceleratorProfile, HardwareProfile},
};

use super::common::collect_common;

/// 使用 macOS 系统接口读取本机硬件信息。
pub struct SystemHardwareProbe;

impl SystemHardwareProbe {
    /// 创建 macOS 硬件探测器。
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

    pub(crate) async fn inspect(&self) -> Result<HardwareProfile, HardwareProbeError> {
        tokio::task::spawn_blocking(|| {
            let common = collect_common()?;
            // system_profiler 缺失或失败时只关闭加速器报告。
            let accelerators = detect_accelerators(common.memory_bytes).unwrap_or_default();
            Ok(common.with_accelerators(accelerators))
        })
        .await
        .map_err(|error| HardwareProbeError::DetectionFailed(error.to_string()))?
    }
}

fn detect_accelerators(
    total_memory_bytes: u64,
) -> Result<Vec<AcceleratorProfile>, HardwareProbeError> {
    let output = Command::new("system_profiler")
        .args(["SPDisplaysDataType", "-json"])
        .output()
        .map_err(|error| HardwareProbeError::GpuDriverUnavailable(error.to_string()))?;
    if !output.status.success() {
        return Err(HardwareProbeError::GpuDriverUnavailable(format!(
            "system_profiler exited with {}",
            output.status
        )));
    }
    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| HardwareProbeError::GpuDriverUnavailable(error.to_string()))?;
    Ok(parse_displays(&value, total_memory_bytes))
}

fn parse_displays(value: &Value, total_memory_bytes: u64) -> Vec<AcceleratorProfile> {
    let Some(displays) = value.get("SPDisplaysDataType").and_then(Value::as_array) else {
        return Vec::new();
    };

    displays
        .iter()
        .filter_map(|display| {
            let vendor = first_string(
                display,
                &["spdisplays_vendor", "sppci_vendor", "spdisplays_gpu_vendor"],
            )
            .unwrap_or_else(|| "Unknown Vendor".to_owned());
            let kind = classify_vendor(&vendor);
            let name = first_string(display, &["sppci_model", "_name", "spdisplays_name"])
                .unwrap_or_else(|| format!("{vendor} GPU"));

            // Apple Silicon 使用统一内存，因此返回系统物理内存总量。
            let memory_bytes = if kind == AcceleratorKind::Apple {
                Some(total_memory_bytes)
            } else {
                display.get("spdisplays_vram").and_then(parse_byte_count)
            };

            Some(AcceleratorProfile {
                kind,
                name,
                memory_bytes,
            })
        })
        .collect()
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn classify_vendor(vendor: &str) -> AcceleratorKind {
    let vendor = vendor.to_ascii_lowercase();
    if vendor.contains("apple") || vendor.contains("0x106b") {
        AcceleratorKind::Apple
    } else if vendor.contains("amd") || vendor.contains("ati") || vendor.contains("0x1002") {
        AcceleratorKind::Amd
    } else if vendor.contains("intel") || vendor.contains("0x8086") {
        AcceleratorKind::Intel
    } else if vendor.contains("nvidia") || vendor.contains("0x10de") {
        AcceleratorKind::Nvidia
    } else {
        AcceleratorKind::Other
    }
}

fn parse_byte_count(value: &Value) -> Option<u64> {
    if let Some(bytes) = value.as_u64() {
        return Some(bytes);
    }

    let text = value.as_str()?.trim();
    let mut parts = text.split_whitespace();
    let number = parts.next()?.replace(',', "").parse::<f64>().ok()?;
    let unit = parts.next()?.to_ascii_lowercase();
    let multiplier = match unit.as_str() {
        "b" | "byte" | "bytes" => 1_u64,
        "kb" | "kib" => 1_u64 << 10,
        "mb" | "mib" => 1_u64 << 20,
        "gb" | "gib" => 1_u64 << 30,
        "tb" | "tib" => 1_u64 << 40,
        _ => return None,
    };
    let bytes = number * multiplier as f64;
    (bytes.is_finite() && bytes >= 0.0 && bytes <= u64::MAX as f64).then_some(bytes.round() as u64)
}

#[cfg(test)]
#[path = "tests/macos.rs"]
mod tests;
