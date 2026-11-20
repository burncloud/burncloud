use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use burncloud_node_runtime::{
    current_arch, detect_platform, AcceleratorKind, HardwareProbe, HardwareProbeError,
    HardwareProfile, Platform, SystemHardwareProbe,
};
use serde::Serialize;

use crate::AppState;

/// 构建公开的 Node 硬件快照接口。
pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/api/node/hardware", get(get_hardware))
}

#[derive(Debug, Serialize)]
pub(super) struct HardwareResponse {
    success: bool,
    data: HardwareSnapshot,
}

#[derive(Debug, Serialize)]
pub(super) struct HardwareSnapshot {
    platform: &'static str,
    arch: &'static str,
    cpu: String,
    memory_bytes: u64,
    disk_available_bytes: u64,
    accelerators: Vec<AcceleratorSnapshot>,
}

#[derive(Debug, Serialize)]
pub(super) struct AcceleratorSnapshot {
    kind: &'static str,
    name: String,
    memory_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
struct HardwareErrorResponse {
    success: bool,
    error: HardwareErrorBody,
}

#[derive(Debug, Serialize)]
struct HardwareErrorBody {
    code: &'static str,
    message: &'static str,
}

impl From<HardwareProfile> for HardwareSnapshot {
    fn from(profile: HardwareProfile) -> Self {
        Self {
            platform: platform_name(detect_platform()),
            arch: current_arch(),
            cpu: profile.cpu,
            memory_bytes: profile.memory_bytes,
            disk_available_bytes: profile.disk_available_bytes,
            accelerators: profile
                .accelerators
                .into_iter()
                .map(AcceleratorSnapshot::from)
                .collect(),
        }
    }
}

impl From<burncloud_node_runtime::AcceleratorProfile> for AcceleratorSnapshot {
    fn from(profile: burncloud_node_runtime::AcceleratorProfile) -> Self {
        Self {
            kind: accelerator_kind_name(profile.kind),
            name: profile.name,
            memory_bytes: profile.memory_bytes,
        }
    }
}

async fn get_hardware() -> Response {
    match SystemHardwareProbe::new().inspect().await {
        Ok(profile) => {
            let response = HardwareResponse {
                success: true,
                data: profile.into(),
            };
            let mut response = Json(response).into_response();
            // 硬件快照会随运行环境变化，禁止浏览器和代理保存旧结果。
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        Err(error) => hardware_error_response(error),
    }
}

fn hardware_error_response(error: HardwareProbeError) -> Response {
    let (status, code, message) = match error {
        HardwareProbeError::UnsupportedPlatform(_) => (
            StatusCode::NOT_IMPLEMENTED,
            "unsupported_platform",
            "当前平台暂不支持硬件检测",
        ),
        error => {
            tracing::error!(%error, "硬件快照检测失败");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "hardware_probe_failed",
                "硬件检测失败",
            )
        }
    };

    let mut response = (
        status,
        Json(HardwareErrorResponse {
            success: false,
            error: HardwareErrorBody { code, message },
        }),
    )
        .into_response();
    // 错误也不能被缓存，否则平台或磁盘状态恢复后前端仍可能看到旧错误。
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

const fn platform_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Linux => "linux",
        Platform::MacOS => "macos",
        Platform::Windows => "windows",
        Platform::Other => "other",
    }
}

const fn accelerator_kind_name(kind: AcceleratorKind) -> &'static str {
    match kind {
        AcceleratorKind::Nvidia => "nvidia",
        AcceleratorKind::Amd => "amd",
        AcceleratorKind::Intel => "intel",
        AcceleratorKind::Apple => "apple",
        AcceleratorKind::Other => "other",
    }
}

#[cfg(test)]
#[path = "hardware_tests.rs"]
mod tests;
