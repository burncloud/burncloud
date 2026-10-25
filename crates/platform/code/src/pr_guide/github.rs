use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Client, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time::sleep;

use super::types::{CheckFact, LinkedIssue, PullRequest, COMMENT_MARKER};

const PAGE_SIZE: u64 = 100;
const MAX_PAGES: u64 = 30;

pub(crate) struct GitHub {
    client: Client,
    token: String,
    owner: String,
    repo: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChangedFile {
    pub(crate) filename: String,
    pub(crate) previous_filename: Option<String>,
    pub(crate) status: String,
    pub(crate) additions: u64,
    pub(crate) deletions: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct IssueComment {
    pub(crate) id: u64,
    pub(crate) body: String,
    pub(crate) html_url: String,
    pub(crate) user: CommentUser,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CommentUser {
    pub(crate) login: String,
    #[serde(rename = "type")]
    pub(crate) kind: String,
}

impl GitHub {
    pub(crate) fn from_env(repository: &str) -> Result<Self> {
        let token = std::env::var("GITHUB_TOKEN").context("GITHUB_TOKEN is required")?;
        let (owner, repo) = repository
            .split_once('/')
            .filter(|(owner, repo)| !owner.is_empty() && !repo.is_empty())
            .context("GITHUB_REPOSITORY must use owner/name form")?;
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("burncloud-pr-guide/1")
            .build()
            .context("failed to construct GitHub HTTP client")?;
        Ok(Self {
            client,
            token,
            owner: owner.to_owned(),
            repo: repo.to_owned(),
        })
    }

    pub(crate) async fn pull_request(&self, number: u64) -> Result<PullRequest> {
        let value: Value = self
            .request_json(
                Method::GET,
                &format!("/repos/{}/{}/pulls/{number}", self.owner, self.repo),
                None,
            )
            .await?;
        Ok(PullRequest {
            number,
            title: string_at(&value, "/title")?,
            body: value
                .pointer("/body")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            state: string_at(&value, "/state")?,
            draft: value
                .pointer("/draft")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            html_url: string_at(&value, "/html_url")?,
            changed_files: value
                .pointer("/changed_files")
                .and_then(Value::as_u64)
                .context("pull request changed_files is missing")?,
            base_ref: string_at(&value, "/base/ref")?,
            base_sha: validated_sha(&string_at(&value, "/base/sha")?)?,
            head_sha: validated_sha(&string_at(&value, "/head/sha")?)?,
            head_label: string_at(&value, "/head/label")?,
        })
    }

    pub(crate) async fn changed_files(&self, number: u64) -> Result<Vec<ChangedFile>> {
        let mut files = Vec::new();
        for page in 1..=MAX_PAGES {
            let batch: Vec<ChangedFile> = self
                .request_json(
                    Method::GET,
                    &format!(
                        "/repos/{}/{}/pulls/{number}/files?per_page={PAGE_SIZE}&page={page}",
                        self.owner, self.repo
                    ),
                    None,
                )
                .await?;
            let count = batch.len();
            files.extend(batch);
            if count < PAGE_SIZE as usize {
                return Ok(files);
            }
        }
        // GitHub 的 PR 文件接口最多可靠返回 3,000 项；本地 Git 差异仍负责完整枚举。
        Ok(files)
    }

    pub(crate) async fn checks(&self, head_sha: &str) -> Result<Vec<CheckFact>> {
        let mut checks = Vec::new();
        for page in 1..=MAX_PAGES {
            let value: Value = self
                .request_json(
                    Method::GET,
                    &format!(
                        "/repos/{}/{}/commits/{head_sha}/check-runs?per_page={PAGE_SIZE}&page={page}",
                        self.owner, self.repo
                    ),
                    None,
                )
                .await?;
            let batch = value
                .pointer("/check_runs")
                .and_then(Value::as_array)
                .context("GitHub check-runs response is missing check_runs")?;
            for check in batch {
                checks.push(CheckFact {
                    name: string_at(check, "/name")?,
                    status: string_at(check, "/status")?,
                    conclusion: optional_string_at(check, "/conclusion"),
                    details_url: optional_string_at(check, "/details_url"),
                    head_sha: validated_sha(&string_at(check, "/head_sha")?)?,
                    completed_at: optional_string_at(check, "/completed_at"),
                });
            }
            if batch.len() < PAGE_SIZE as usize {
                checks.sort_by(|left, right| left.name.cmp(&right.name));
                return Ok(checks);
            }
        }
        anyhow::bail!("GitHub check-run pagination reached the {MAX_PAGES}-page safety limit")
    }

    pub(crate) async fn linked_issues(&self, body: &str) -> Result<Vec<LinkedIssue>> {
        let mut issues = Vec::new();
        for number in referenced_numbers(body).into_iter().take(10) {
            let path = format!("/repos/{}/{}/issues/{number}", self.owner, self.repo);
            let value = match self.request_json::<Value>(Method::GET, &path, None).await {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("Skipping linked issue #{number}: {error:#}");
                    continue;
                }
            };
            if value.pointer("/pull_request").is_some() {
                continue;
            }
            issues.push(LinkedIssue {
                number,
                title: string_at(&value, "/title")?,
                state: string_at(&value, "/state")?,
                html_url: string_at(&value, "/html_url")?,
            });
        }
        Ok(issues)
    }

    pub(crate) async fn guide_comment(&self, number: u64) -> Result<Option<IssueComment>> {
        let mut found = None;
        for page in 1..=MAX_PAGES {
            let batch: Vec<IssueComment> = self
                .request_json(
                    Method::GET,
                    &format!(
                        "/repos/{}/{}/issues/{number}/comments?per_page={PAGE_SIZE}&page={page}",
                        self.owner, self.repo
                    ),
                    None,
                )
                .await?;
            for comment in &batch {
                if comment.body.contains(COMMENT_MARKER)
                    && comment.user.kind == "Bot"
                    && comment.user.login == "github-actions[bot]"
                {
                    found = Some(comment.clone());
                }
            }
            if batch.len() < PAGE_SIZE as usize {
                return Ok(found);
            }
        }
        anyhow::bail!("GitHub comment pagination reached the {MAX_PAGES}-page safety limit")
    }

    pub(crate) async fn upsert_guide_comment(
        &self,
        number: u64,
        comment_id: Option<u64>,
        body: &str,
    ) -> Result<IssueComment> {
        if body.len() > 65_000 {
            anyhow::bail!("refusing to publish a PR comment larger than 65,000 bytes");
        }
        let payload = json!({ "body": body });
        if let Some(comment_id) = comment_id {
            self.request_json(
                Method::PATCH,
                &format!(
                    "/repos/{}/{}/issues/comments/{comment_id}",
                    self.owner, self.repo
                ),
                Some(&payload),
            )
            .await
        } else {
            self.request_json(
                Method::POST,
                &format!(
                    "/repos/{}/{}/issues/{number}/comments",
                    self.owner, self.repo
                ),
                Some(&payload),
            )
            .await
        }
    }

    async fn request_json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<T> {
        let url = format!("https://api.github.com{path}");
        for attempt in 0..3_u64 {
            let mut request = self
                .client
                .request(method.clone(), &url)
                .bearer_auth(&self.token)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28");
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = request
                .send()
                .await
                .with_context(|| format!("GitHub request failed: {method} {path}"))?;
            let status = response.status();
            let retryable = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
            if retryable && attempt < 2 {
                sleep(Duration::from_secs(2_u64.pow(attempt as u32))).await;
                continue;
            }
            let bytes = response
                .bytes()
                .await
                .context("failed to read GitHub response")?;
            if !status.is_success() {
                let message = String::from_utf8_lossy(&bytes);
                anyhow::bail!(
                    "GitHub request {method} {path} returned {status}: {}",
                    truncate(&message, 1_000)
                );
            }
            return serde_json::from_slice(&bytes)
                .with_context(|| format!("GitHub returned invalid JSON for {method} {path}"));
        }
        anyhow::bail!("GitHub request retry loop ended unexpectedly")
    }
}

fn string_at(value: &Value, pointer: &str) -> Result<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("GitHub response is missing string field {pointer}"))
}

fn optional_string_at(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn validated_sha(value: &str) -> Result<String> {
    if (40..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(value.to_ascii_lowercase())
    } else {
        anyhow::bail!("GitHub returned an invalid commit SHA")
    }
}

fn referenced_numbers(body: &str) -> Vec<u64> {
    let mut numbers = Vec::new();
    let bytes = body.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'#' {
            index += 1;
            continue;
        }
        let start = index + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end].is_ascii_digit() && end - start < 10 {
            end += 1;
        }
        if end > start {
            if let Ok(number) = body[start..end].parse::<u64>() {
                if number > 0 && !numbers.contains(&number) {
                    numbers.push(number);
                }
            }
        }
        index = end.max(index + 1);
    }
    numbers
}

fn truncate(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}
