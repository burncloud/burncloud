use async_trait::async_trait;

use crate::{
    PreparedArtifact, PreparedRuntime, ProcessPlan, ProcessSpec, ReadinessTarget, RuntimeAdapter,
    RuntimeAdapterError,
};

use super::{local_http_endpoint, plan_error, validate_common_inputs};

const CONTAINER_MODEL_PATH: &str = "/models/burncloud-artifact";

/// Configuration for one Linux Docker SGLang server launch profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SglangDockerConfig {
    image: String,
    host: String,
    port: u16,
}

impl SglangDockerConfig {
    pub fn new(image: impl Into<String>, host: impl Into<String>, port: u16) -> Self {
        Self {
            image: image.into(),
            host: host.into(),
            port,
        }
    }
}

/// Pure launch-plan adapter for running SGLang inside Docker.
///
/// It only produces a Docker CLI argument vector. It never contacts Docker,
/// reads the artifact, pulls an image, or starts a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SglangDockerAdapter {
    config: SglangDockerConfig,
}

impl SglangDockerAdapter {
    pub fn new(config: SglangDockerConfig) -> Self {
        Self { config }
    }

    fn image(&self) -> Result<&str, RuntimeAdapterError> {
        let image = self.config.image.trim();
        if image.is_empty() || image.chars().any(char::is_whitespace) {
            return Err(plan_error("SGLang Docker image is invalid"));
        }
        Ok(image)
    }
}

#[async_trait]
impl RuntimeAdapter for SglangDockerAdapter {
    async fn plan(
        &self,
        runtime: &PreparedRuntime,
        artifact: &PreparedArtifact,
    ) -> Result<ProcessPlan, RuntimeAdapterError> {
        let (docker_executable, artifact_path, host) =
            validate_common_inputs(runtime, artifact, &self.config.host, self.config.port)?;
        let image = self.image()?;
        let port = self.config.port.to_string();

        let args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "--gpus".to_string(),
            "all".to_string(),
            "--publish".to_string(),
            format!("{host}:{}:{}", self.config.port, self.config.port),
            "--volume".to_string(),
            format!("{artifact_path}:{CONTAINER_MODEL_PATH}:ro"),
            image.to_string(),
            "python3".to_string(),
            "-m".to_string(),
            "sglang.launch_server".to_string(),
            "--model-path".to_string(),
            CONTAINER_MODEL_PATH.to_string(),
            "--host".to_string(),
            "0.0.0.0".to_string(),
            "--port".to_string(),
            port,
        ];

        let local_endpoint = local_http_endpoint(host, self.config.port);
        Ok(ProcessPlan {
            process: ProcessSpec {
                program: docker_executable.to_string(),
                args,
            },
            readiness: ReadinessTarget {
                endpoint: format!("{local_endpoint}/health"),
            },
            local_endpoint,
        })
    }
}
