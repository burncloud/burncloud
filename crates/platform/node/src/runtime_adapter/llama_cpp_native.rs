use async_trait::async_trait;

use crate::{
    PreparedArtifact, PreparedRuntime, ProcessPlan, ProcessSpec, ReadinessTarget, RuntimeAdapter,
    RuntimeAdapterError,
};

use super::{local_http_endpoint, plan_error, validate_common_inputs};

/// Configuration for one native llama.cpp server launch profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaCppNativeConfig {
    host: String,
    port: u16,
    extra_args: Vec<String>,
}

impl LlamaCppNativeConfig {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            extra_args: Vec::new(),
        }
    }

    pub fn with_extra_args(mut self, extra_args: Vec<String>) -> Self {
        self.extra_args = extra_args;
        self
    }
}

/// Pure launch-plan adapter for a native llama.cpp server executable.
///
/// It performs no installation, file access, port binding, or process spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaCppNativeAdapter {
    config: LlamaCppNativeConfig,
}

impl LlamaCppNativeAdapter {
    pub fn new(config: LlamaCppNativeConfig) -> Self {
        Self { config }
    }

    fn validate_extra_args(&self) -> Result<(), RuntimeAdapterError> {
        const RESERVED: [&str; 4] = ["-m", "--model", "--host", "--port"];

        if self.config.extra_args.iter().any(|arg| {
            let arg = arg.trim();
            RESERVED.contains(&arg)
                || arg.starts_with("--model=")
                || arg.starts_with("--host=")
                || arg.starts_with("--port=")
        }) {
            return Err(plan_error(
                "extra llama.cpp arguments must not override model, host, or port",
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl RuntimeAdapter for LlamaCppNativeAdapter {
    async fn plan(
        &self,
        runtime: &PreparedRuntime,
        artifact: &PreparedArtifact,
    ) -> Result<ProcessPlan, RuntimeAdapterError> {
        let (executable, artifact_path, host) = validate_common_inputs(
            runtime,
            artifact,
            &self.config.host,
            self.config.port,
        )?;
        self.validate_extra_args()?;

        let mut args = vec![
            "--model".to_string(),
            artifact_path.to_string(),
            "--host".to_string(),
            host.to_string(),
            "--port".to_string(),
            self.config.port.to_string(),
        ];
        args.extend(self.config.extra_args.clone());

        let local_endpoint = local_http_endpoint(host, self.config.port);
        Ok(ProcessPlan {
            process: ProcessSpec {
                program: executable.to_string(),
                args,
            },
            readiness: ReadinessTarget {
                endpoint: format!("{local_endpoint}/health"),
            },
            local_endpoint,
        })
    }
}
