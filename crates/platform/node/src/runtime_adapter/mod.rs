mod llama_cpp_native;
mod sglang_docker;

pub use llama_cpp_native::{LlamaCppNativeAdapter, LlamaCppNativeConfig};
pub use sglang_docker::{SglangDockerAdapter, SglangDockerConfig};

use crate::{PreparedArtifact, PreparedRuntime, RuntimeAdapterError};

pub(super) fn validate_common_inputs<'a>(
    runtime: &'a PreparedRuntime,
    artifact: &'a PreparedArtifact,
    host: &'a str,
    port: u16,
) -> Result<(&'a str, &'a str, &'a str), RuntimeAdapterError> {
    let executable = runtime.executable.trim();
    if executable.is_empty() {
        return Err(plan_error("prepared runtime executable is empty"));
    }

    let artifact_path = artifact.local_path.trim();
    if artifact_path.is_empty() {
        return Err(plan_error("prepared artifact path is empty"));
    }
    if !artifact.verified {
        return Err(plan_error("prepared artifact is not verified"));
    }

    let host = host.trim();
    if host.is_empty()
        || host.chars().any(char::is_whitespace)
        || host.contains('/')
        || host.contains("://")
    {
        return Err(plan_error("runtime host is invalid"));
    }
    if port == 0 {
        return Err(plan_error("runtime port must be greater than zero"));
    }

    Ok((executable, artifact_path, host))
}

pub(super) fn local_http_endpoint(host: &str, port: u16) -> String {
    let authority = if host.contains(':') && !(host.starts_with('[') && host.ends_with(']')) {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    format!("http://{authority}")
}

pub(super) fn plan_error(message: impl Into<String>) -> RuntimeAdapterError {
    RuntimeAdapterError::PlanFailed(message.into())
}
