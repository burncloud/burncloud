//! 模型站点请求、外部响应解析与去重；只向本模块暴露必要契约。

#![allow(
    clippy::disallowed_types,
    reason = "上游 ModelScope/Hugging Face 协议解析边界需要读取异构字段"
)]

use reqwest::header::ACCEPT;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Config;

const PAGE_SIZE: &str = "50";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Source {
    #[serde(rename = "modelscope")]
    ModelScope,
    #[serde(rename = "hf-mirror")]
    HfMirror,
    #[serde(rename = "huggingface")]
    HuggingFace,
}

impl Source {
    // 同名型号在不同站点出现时，优先选择国内来源，而不改变到达顺序。
    fn priority(self) -> u8 {
        match self {
            Source::ModelScope => 0,
            Source::HfMirror => 1,
            Source::HuggingFace => 2,
        }
    }

    fn base(self, config: &Config) -> &str {
        match self {
            Source::ModelScope => &config.modelscope,
            Source::HfMirror => &config.hf_mirror,
            Source::HuggingFace => &config.huggingface,
        }
    }

    /// 返回站点默认修订号。
    ///
    /// # Examples
    /// ModelScope 使用 `master`，其他站点使用 `main`。
    ///
    /// # Notes
    /// 仅在上游未给出安全修订号时使用。
    pub(super) fn default_revision(self) -> &'static str {
        match self {
            Source::ModelScope => "master",
            _ => "main",
        }
    }
}

/// 模块内部的仓库资源快照，来源只能使用固定站点枚举。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct ModelResource {
    pub(super) model: String,
    pub(super) source: Source,
    pub(super) revision: String,
}

impl ModelResource {
    /// 校验外部仓库参数并构造资源页面地址，不接受用户提供的任意网址。
    ///
    /// # Examples
    /// `Qwen/Qwen3-8B` 配合 ModelScope 的 `master` 修订号定位仓库页面。
    ///
    /// # Notes
    /// 页面地址不是可部署的模型文件。
    pub(super) fn url(&self, config: &Config) -> Option<Url> {
        if !is_valid_model_id(&self.model) || !is_valid_segment(&self.revision) {
            return None;
        }
        let (org, name) = self.model.split_once('/')?;
        let mut url = Url::parse(self.source.base(config)).ok()?;
        {
            let mut segments = url.path_segments_mut().ok()?;
            segments.pop_if_empty();
            match self.source {
                Source::ModelScope => {
                    segments.extend(["models", org, name, "files"]);
                }
                _ => {
                    segments.extend([org, name, "tree", self.revision.as_str()]);
                }
            }
        }
        if self.source == Source::ModelScope {
            url.query_pairs_mut()
                .append_pair("Revision", &self.revision);
        }
        Some(url)
    }
}

/// 与具体仓库绑定的搜索命中项。
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(super) struct SearchItem {
    id: String,
    name: String,
    #[serde(flatten)]
    resource: ModelResource,
}

fn is_valid_segment(part: &str) -> bool {
    !part.is_empty()
        && part.len() <= 128
        && part != "."
        && part != ".."
        && part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

// 仅允许安全的两段式仓库名，避免查询参数变成路径跳转。
fn is_valid_model_id(id: &str) -> bool {
    match id.split_once('/') {
        Some((org, name)) => is_valid_segment(org) && is_valid_segment(name),
        None => false,
    }
}

/// 跨站点去重使用小写仓库短名称，与 v0.2 契约保持一致。
///
/// # Examples
/// `Qwen/Qwen3-8B` 对应 `qwen3-8b`。
///
/// # Notes
/// 去重标识不替代来源、仓库与修订号组成的资源身份。
pub(super) fn version_id(model: &str) -> String {
    model
        .rsplit('/')
        .next()
        .unwrap_or(model)
        .to_ascii_lowercase()
}

// 上游缺少展示名称时，只从真实仓库名生成可读标题。
fn readable_name(model: &str) -> String {
    model
        .rsplit('/')
        .next()
        .unwrap_or(model)
        .replace(['-', '_'], " ")
}

fn text<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .filter_map(|key| value.get(*key)?.as_str())
        .map(str::trim)
        .find(|s| !s.is_empty())
}

fn to_item(
    source: Source,
    model: &str,
    title: Option<&str>,
    revision: Option<&str>,
) -> Option<SearchItem> {
    if !is_valid_model_id(model) {
        return None;
    }
    let name = title
        .filter(|title| *title != model)
        .map(str::to_string)
        .unwrap_or_else(|| readable_name(model));
    let revision = revision
        .filter(|r| is_valid_segment(r))
        .unwrap_or(source.default_revision());
    Some(SearchItem {
        id: version_id(model),
        name,
        resource: ModelResource {
            model: model.to_string(),
            source,
            revision: revision.to_string(),
        },
    })
}

fn parse_modelscope(payload: &Value) -> Option<Vec<SearchItem>> {
    if payload.get("success").and_then(Value::as_bool) == Some(false) {
        return None;
    }
    let data = payload.get("data").unwrap_or(payload);
    let models = data.get("models")?.as_array()?;
    Some(
        models
            .iter()
            .filter_map(|m| {
                to_item(
                    Source::ModelScope,
                    text(m, &["id", "model_id", "name"])?,
                    text(m, &["display_name", "title"]),
                    text(m, &["default_branch", "revision"]),
                )
            })
            .collect(),
    )
}

fn parse_hf(payload: &Value, source: Source) -> Option<Vec<SearchItem>> {
    let models = payload.as_array()?;
    Some(
        models
            .iter()
            .filter_map(|m| {
                let card = m.get("cardData").unwrap_or(&Value::Null);
                let title = text(m, &["title", "display_name"])
                    .or_else(|| text(card, &["model_name", "pretty_name", "title"]));
                to_item(
                    source,
                    text(m, &["id", "modelId"])?,
                    title,
                    text(m, &["default_branch", "revision"]),
                )
            })
            .collect(),
    )
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum FetchError {
    Timeout,
    Failed,
}

fn classify(error: reqwest::Error) -> FetchError {
    tracing::warn!(%error, "模型上游请求失败");
    if error.is_timeout() {
        FetchError::Timeout
    } else {
        FetchError::Failed
    }
}

fn search_url(config: &Config, source: Source, query: &str) -> Option<Url> {
    let mut url = Url::parse(source.base(config)).ok()?;
    {
        let mut segments = url.path_segments_mut().ok()?;
        segments.pop_if_empty();
        match source {
            Source::ModelScope => {
                segments.extend(["openapi", "v1", "models"]);
            }
            _ => {
                segments.extend(["api", "models"]);
            }
        }
    }
    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("search", query)
            .append_pair("sort", "downloads");
        match source {
            Source::ModelScope => {
                pairs
                    .append_pair("page_size", PAGE_SIZE)
                    .append_pair("page_number", "1");
            }
            _ => {
                pairs
                    .append_pair("limit", PAGE_SIZE)
                    .append_pair("direction", "-1")
                    .append_pair("cardData", "true");
            }
        }
    }
    Some(url)
}

/// 查询单个站点并在外部响应边界过滤无效仓库。
///
/// # Examples
/// 使用 `Source::ModelScope` 搜索 `qwen`。
///
/// # Notes
/// 超时单独分类，以便编排器只在国内站点超时后启动回退。
pub(super) async fn search_source(
    client: &reqwest::Client,
    config: &Config,
    source: Source,
    query: &str,
) -> Result<Vec<SearchItem>, FetchError> {
    let url = search_url(config, source, query).ok_or(FetchError::Failed)?;
    let response = client
        .get(url)
        .header(ACCEPT, "application/json")
        .timeout(config.search_timeout)
        .send()
        .await
        .map_err(classify)?;
    if !response.status().is_success() {
        tracing::warn!(?source, status = %response.status(), "模型搜索上游返回错误状态");
        return Err(FetchError::Failed);
    }
    let payload: Value = response.json().await.map_err(classify)?;
    match source {
        Source::ModelScope => parse_modelscope(&payload),
        _ => parse_hf(&payload, source),
    }
    .ok_or_else(|| {
        tracing::warn!(?source, "模型搜索上游响应格式无效");
        FetchError::Failed
    })
}

/// 对资源页面执行 GET；调用方负责记录错误并映射为不可下载状态。
///
/// # Examples
/// HTTP 200 可以显示下载入口，其他状态不能作为下载凭据。
///
/// # Notes
/// 不读取页面内容，也不把页面当作模型工件。
pub(super) async fn probe(
    client: &reqwest::Client,
    config: &Config,
    url: Url,
) -> Result<u16, reqwest::Error> {
    client
        .get(url)
        .header(ACCEPT, "text/html,application/json")
        .timeout(config.probe_timeout)
        .send()
        .await
        .map(|response| response.status().as_u16())
}

/// 按版本标识去重；高优先级来源原位替换，保持到达顺序。
#[derive(Default)]
pub(super) struct Merged {
    items: Vec<SearchItem>,
}

impl Merged {
    /// 合并真实命中项，返回列表是否改变。
    ///
    /// # Examples
    /// ModelScope 命中可以原位替换 hf-mirror 命中。
    ///
    /// # Notes
    /// 同优先级命中不会反复覆盖。
    pub(super) fn add(&mut self, incoming: Vec<SearchItem>) -> bool {
        let mut changed = false;
        for item in incoming {
            match self
                .items
                .iter_mut()
                .find(|existing| existing.id == item.id)
            {
                Some(existing) => {
                    if item.resource.source.priority() < existing.resource.source.priority() {
                        *existing = item;
                        changed = true;
                    }
                }
                None => {
                    self.items.push(item);
                    changed = true;
                }
            }
        }
        changed
    }

    /// 借用累计快照，避免发送前复制完整列表。
    ///
    /// # Examples
    /// 序列化最终响应时使用当前快照。
    ///
    /// # Notes
    /// 调用方不应改变返回项的顺序。
    pub(super) fn items(&self) -> &[SearchItem] {
        &self.items
    }
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;
