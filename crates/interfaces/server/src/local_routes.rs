use burncloud_router::local_attachment::LocalRouteAttachment;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfiguredAttachment {
    model: String,
    base_url: String,
}

// 部署元数据不是健康凭证，只提取现有挂载入口真正需要的目标字段。
#[derive(Deserialize)]
struct DeploymentResult {
    model_id: String,
    status: String,
    state: String,
    endpoint: String,
    error: Option<serde::de::IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DeploymentInput {
    One(DeploymentResult),
    Many(Vec<DeploymentResult>),
}

pub(super) fn configured_attachments() -> anyhow::Result<Vec<LocalRouteAttachment>> {
    let read = |name| match std::env::var(name) {
        Ok(raw) => Ok(Some(raw)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => anyhow::bail!("{name} must be valid Unicode"),
    };
    parse_sources(
        read("BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS")?.as_deref(),
        read("BURNCLOUD_LOCAL_ROUTE_DEPLOYMENTS")?.as_deref(),
    )
}

fn parse_sources(
    attachments: Option<&str>,
    deployments: Option<&str>,
) -> anyhow::Result<Vec<LocalRouteAttachment>> {
    match (attachments, deployments) {
        (Some(_), Some(_)) => anyhow::bail!(
            "BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS and BURNCLOUD_LOCAL_ROUTE_DEPLOYMENTS are mutually exclusive"
        ),
        (None, Some(raw)) => parse_deployments(raw),
        (raw, None) => parse_attachments(raw),
    }
}

fn parse_deployments(raw: &str) -> anyhow::Result<Vec<LocalRouteAttachment>> {
    let input: DeploymentInput = serde_json::from_str(raw)
        .map_err(|error| anyhow::anyhow!("invalid BURNCLOUD_LOCAL_ROUTE_DEPLOYMENTS: {error}"))?;
    let deployments = match input {
        DeploymentInput::One(result) => vec![result],
        DeploymentInput::Many(results) => results,
    };
    deployments
        .into_iter()
        .enumerate()
        .map(|(index, result)| {
            anyhow::ensure!(
                result.status == "ready" && result.state == "Ready" && result.error.is_none(),
                "BURNCLOUD_LOCAL_ROUTE_DEPLOYMENTS entry {index} must have status=ready, state=Ready and no error"
            );
            // Ready 只允许进入首检，不采纳外部 PID、事件或时间戳作为准入证据。
            Ok(LocalRouteAttachment {
                model: result.model_id,
                base_url: result.endpoint,
            })
        })
        .collect()
}

fn parse_attachments(raw: Option<&str>) -> anyhow::Result<Vec<LocalRouteAttachment>> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let configured: Vec<ConfiguredAttachment> = serde_json::from_str(raw)
        .map_err(|error| anyhow::anyhow!("invalid BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS: {error}"))?;
    // 此处只解码形状，模型与地址合法域由 Router 公有构造入口统一校验。
    Ok(configured
        .into_iter()
        .map(|entry| LocalRouteAttachment {
            model: entry.model,
            base_url: entry.base_url,
        })
        .collect())
}

#[cfg(test)]
#[path = "local_routes_tests.rs"]
mod tests;
