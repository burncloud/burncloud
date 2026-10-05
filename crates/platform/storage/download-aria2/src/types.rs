use std::path::PathBuf;
use std::process::Child;

use serde::{Deserialize, Serialize};

use crate::constants::{get_burncloud_dir, DEFAULT_PORT};
use crate::error::{Aria2Error, Aria2Result};

// ============================================================================
// 数据结构定义
// ============================================================================

/// Configuration for an aria2 process.
///
/// **`Debug` is written by hand so the secret cannot be printed.** The derived implementation would have
/// included `secret`, and `Aria2Config` is held by `Aria2Instance`, which the daemon stores -- so a single
/// `println!("{:?}", ...)` anywhere on that path would put the RPC token into a log. The plan lists "token 不写
/// 日志" and this is the one place where the type invites it.
///
/// The substitution is a fixed marker rather than a masked prefix: a partial token is still a token, and the
/// useful information in a log line is *whether* a secret is set, not what it is.
#[derive(Clone)]
pub struct Aria2Config {
    pub port: u16,
    pub secret: Option<String>,
    pub download_dir: PathBuf,
    pub max_connections: u8,
    pub split_size: String,
    pub aria2_path: PathBuf,
}

impl Default for Aria2Config {
    // 输入：无。
    // 功能：创建带默认端口、下载目录和 aria2 路径的配置。
    // 错误：无，当前工作目录读取失败时使用空路径。
    fn default() -> Self {
        // 组装默认配置字段
        Self {
            port: DEFAULT_PORT,
            secret: None,
            download_dir: std::env::current_dir()
                .unwrap_or_default()
                .join("downloads"),
            max_connections: 16,
            split_size: "1M".to_string(),
            aria2_path: get_burncloud_dir().join("aria2c.exe"),
        }
    }
}

impl std::fmt::Debug for Aria2Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Aria2Config")
            .field("port", &self.port)
            // Whether a secret exists, never the secret itself.
            .field(
                "secret",
                &match &self.secret {
                    Some(_) => "<redacted>",
                    None => "<none>",
                },
            )
            .field("download_dir", &self.download_dir)
            .field("max_connections", &self.max_connections)
            .field("split_size", &self.split_size)
            .field("aria2_path", &self.aria2_path)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split: Option<u8>,
    #[serde(
        rename = "max-connection-per-server",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_connection_per_server: Option<u8>,
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_download: Option<bool>,
    #[serde(rename = "allow-overwrite", skip_serializing_if = "Option::is_none")]
    pub allow_overwrite: Option<bool>,
    #[serde(rename = "auto-file-renaming", skip_serializing_if = "Option::is_none")]
    pub auto_file_renaming: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadStatus {
    pub gid: String,
    pub status: String,
    #[serde(rename = "totalLength")]
    pub total_length: String,
    #[serde(rename = "completedLength")]
    pub completed_length: String,
    #[serde(rename = "downloadSpeed")]
    pub download_speed: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalStat {
    #[serde(rename = "downloadSpeed")]
    pub download_speed: String,
    #[serde(rename = "numActive")]
    pub num_active: String,
    #[serde(rename = "numWaiting")]
    pub num_waiting: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub uris: Vec<UriInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UriInfo {
    pub uri: String,
    pub status: String,
}

pub struct Aria2Instance {
    pub process: Child,
    pub port: u16,
    pub config: Aria2Config,
}

impl Aria2Instance {
    // 输入：可变的 aria2 进程实例。
    // 功能：探测关联的 aria2 子进程是否仍在运行。
    // 错误：进程状态查询失败时返回 false。
    pub fn is_running(&mut self) -> bool {
        // 查询子进程的退出状态
        matches!(self.process.try_wait(), Ok(None))
    }

    // 输入：可变的 aria2 进程实例。
    // 功能：终止并等待关联的 aria2 子进程退出。
    // 错误：进程终止或等待失败时返回 ProcessError。
    pub fn kill(&mut self) -> Aria2Result<()> {
        // 请求终止子进程
        self.process
            .kill()
            .map_err(|e| Aria2Error::ProcessError(e.to_string()))?;
        // 等待子进程完成退出
        self.process
            .wait()
            .map_err(|e| Aria2Error::ProcessError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Aria2Config;

    /// **The token must not appear in the `Debug` output**, which is what a log line uses.
    ///
    /// The derived `Debug` printed `secret: Some("s3cr3t")`, and `Aria2Config` is reachable from `Aria2Instance`,
    /// which the daemon stores -- so any `{:?}` on that path would write the RPC token to a log. The plan lists
    /// "token 不写日志" and this is where the type invites the mistake.
    ///
    /// The assertion is on the **whole rendered string** rather than on a field, because a partial leak is a
    /// leak: `format!("{:?}", config)` is what a caller writes.
    #[test]
    fn the_debug_output_does_not_contain_the_secret() {
        let secret = "s3cr3t-token-value";
        let config = Aria2Config {
            secret: Some(secret.to_string()),
            ..Aria2Config::default()
        };

        let rendered = format!("{config:?}");
        println!("{rendered}");

        assert!(
            !rendered.contains(secret),
            "the secret must not be in the Debug output: {rendered}"
        );
        // No prefix either: a partial token is still a token.
        for length in [3, 6, 8] {
            let prefix = &secret[..length];
            assert!(
                !rendered.contains(prefix),
                "not even the first {length} characters ({prefix:?}): {rendered}"
            );
        }

        // The useful information survives: that a secret is set at all.
        assert!(
            rendered.contains("redacted"),
            "the output says a secret is present: {rendered}"
        );
        // And nothing is lost from the other fields, or the substitution would have broken diagnosis.
        for field in [
            "port",
            "download_dir",
            "max_connections",
            "split_size",
            "aria2_path",
        ] {
            assert!(
                rendered.contains(field),
                "{field} is still printed: {rendered}"
            );
        }
    }

    /// With no secret configured the output says so, which is different from hiding one.
    #[test]
    fn the_debug_output_distinguishes_no_secret_from_a_hidden_one() {
        let without = format!("{:?}", Aria2Config::default());
        let with = format!(
            "{:?}",
            Aria2Config {
                secret: Some("x".to_string()),
                ..Aria2Config::default()
            }
        );

        println!("without: {without}");
        println!("with:    {with}");

        assert!(
            without.contains("<none>"),
            "an unset secret reads as absent"
        );
        assert!(with.contains("<redacted>"), "a set secret reads as hidden");
        assert_ne!(
            without, with,
            "the two states must be distinguishable, or a log cannot tell whether auth is configured"
        );
    }
}
