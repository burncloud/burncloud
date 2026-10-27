use std::time::Duration;

use async_trait::async_trait;
use tokio::time::sleep;

use crate::{
    HealthError, HealthProbe, ReadinessError, ReadinessProbe, ReadinessTarget,
};

/// 提供共享 HTTP client 的 readiness 与 health 探测器。
///
/// 每次请求最多等待 10 秒；readiness 在非 2xx 或请求失败后每秒重试，
/// health 则把非 2xx 作为不健康结果返回。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::deployment::HttpProbe;
///
/// let _probe = HttpProbe::default();
/// ```
#[derive(Clone)]
pub struct HttpProbe {
    client: reqwest::Client,
}

impl Default for HttpProbe {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|error| panic!("failed to create HTTP probe client: {error}"));

        Self { client }
    }
}

#[async_trait]
impl ReadinessProbe for HttpProbe {
    async fn wait_ready(&self, target: ReadinessTarget) -> Result<(), ReadinessError> {
        loop {
            match self.client.get(target.endpoint.as_str()).send().await {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(_) => {}
                Err(error) if error.is_builder() => {
                    return Err(ReadinessError::CheckFailed(error.to_string()));
                }
                Err(_) => {}
            }

            sleep(Duration::from_secs(1)).await;
        }
    }
}

#[async_trait]
impl HealthProbe for HttpProbe {
    async fn is_healthy(&self, target: ReadinessTarget) -> Result<bool, HealthError> {
        let response = self
            .client
            .get(target.endpoint.as_str())
            .send()
            .await
            .map_err(|error| HealthError::CheckFailed(error.to_string()))?;

        Ok(response.status().is_success())
    }
}
