#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
//! Aliyun ECS API Client
//!
//! Direct HTTP API calls to Aliyun ECS without external CLI dependency

use anyhow::{Context, Result};
use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::blocking::Client;
use serde::Deserialize;
use sha1::Sha1;
use std::collections::BTreeMap;

type HmacSha1 = Hmac<Sha1>;

/// Aliyun ECS Client Configuration
#[derive(Debug, Clone)]
pub struct AliyunConfig {
    pub access_key_id: String,
    pub access_key_secret: String,
    pub region_id: String,
}

impl AliyunConfig {
    /// Load configuration from ~/.aliyun/config.json
    pub fn from_config_file() -> Result<Self> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .context("Cannot determine home directory")?;

        let config_path = format!("{}/.aliyun/config.json", home);
        let content = std::fs::read_to_string(&config_path)
            .context(format!("Cannot read config file: {}", config_path))?;

        let config: AliyunConfigFile =
            serde_json::from_str(&content).context("Invalid config file format")?;

        let current_profile = config.current.as_deref().unwrap_or("default");

        for profile in &config.profiles {
            if profile.name == current_profile {
                return Ok(AliyunConfig {
                    access_key_id: profile
                        .access_key_id
                        .clone()
                        .context("Missing access_key_id")?,
                    access_key_secret: profile
                        .access_key_secret
                        .clone()
                        .context("Missing access_key_secret")?,
                    region_id: profile
                        .region_id
                        .clone()
                        .unwrap_or_else(|| "cn-shenzhen".to_string()),
                });
            }
        }

        anyhow::bail!("Profile '{}' not found in config", current_profile)
    }
}

#[derive(Debug, Deserialize)]
struct AliyunConfigFile {
    current: Option<String>,
    profiles: Vec<Profile>,
}

#[derive(Debug, Deserialize)]
struct Profile {
    name: String,
    access_key_id: Option<String>,
    access_key_secret: Option<String>,
    region_id: Option<String>,
}

/// Aliyun ECS Instance Info
#[derive(Debug, Clone, Deserialize)]
pub struct InstanceInfo {
    #[serde(rename = "InstanceId")]
    pub instance_id: String,
    #[serde(rename = "InstanceName")]
    pub instance_name: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "PublicIpAddress")]
    pub public_ip_address: Option<PublicIpAddress>,
    #[serde(rename = "OSName")]
    pub os_name: String,
    #[serde(rename = "InstanceType")]
    pub instance_type: String,
    #[serde(rename = "ZoneId")]
    pub zone_id: String,
    #[serde(rename = "CreationTime")]
    pub creation_time: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PublicIpAddress {
    pub ip_address: Vec<String>,
}

/// Aliyun ECS API Client
pub struct AliyunECS {
    config: AliyunConfig,
    client: Client,
    endpoint: String,
}

impl AliyunECS {
    pub fn new(config: AliyunConfig) -> Self {
        let endpoint = format!("ecs.{}.aliyuncs.com", &config.region_id);
        Self {
            config,
            client: Client::new(),
            endpoint,
        }
    }

    /// Create client from config file
    pub fn from_config_file() -> Result<Self> {
        let config = AliyunConfig::from_config_file()?;
        Ok(Self::new(config))
    }

    /// Create client from config file with region override
    pub fn from_config_file_with_region(region_id: &str) -> Result<Self> {
        let mut config = AliyunConfig::from_config_file()?;
        config.region_id = region_id.to_string();
        Ok(Self::new(config))
    }

    /// Generate API signature
    fn sign(&self, params: &BTreeMap<String, String>) -> String {
        // Build canonical query string
        let query_string: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        // String to sign
        let string_to_sign = format!("GET&%2F&{}", urlencoding::encode(&query_string));

        // HMAC-SHA1 signature
        let key = format!("{}&", self.config.access_key_secret);
        let mut mac = HmacSha1::new_from_slice(key.as_bytes()).expect("HMAC key creation failed");
        mac.update(string_to_sign.as_bytes());
        let signature =
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

        signature
    }

    /// Call ECS API
    fn call_api(
        &self,
        action: &str,
        params: BTreeMap<String, String>,
    ) -> Result<serde_json::Value> {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        let nonce = format!("{}-{}", timestamp, uuid::Uuid::new_v4());

        // Public parameters
        let mut all_params = BTreeMap::new();
        all_params.insert("Format".to_string(), "JSON".to_string());
        all_params.insert("Version".to_string(), "2014-05-26".to_string());
        all_params.insert("AccessKeyId".to_string(), self.config.access_key_id.clone());
        all_params.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
        all_params.insert("Timestamp".to_string(), timestamp.to_string());
        all_params.insert("SignatureVersion".to_string(), "1.0".to_string());
        all_params.insert("SignatureNonce".to_string(), nonce);
        all_params.insert("Action".to_string(), action.to_string());
        all_params.insert("RegionId".to_string(), self.config.region_id.clone());

        // Merge action-specific params
        for (k, v) in params {
            all_params.insert(k, v);
        }

        // Sign and add signature
        let signature = self.sign(&all_params);
        all_params.insert("Signature".to_string(), signature);

        // Build URL
        let query = all_params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!("https://{}?{}", self.endpoint, query);

        // Make request
        let response = self
            .client
            .get(&url)
            .send()
            .context(format!("API call failed: {}", action))?;

        let status = response.status();
        let body = response.text().context("Failed to read response body")?;

        let json: serde_json::Value = if body.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(&body).context(format!("Invalid JSON response: {}", body))?
        };

        if !status.is_success() {
            let message = json
                .get("Message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            anyhow::bail!("API error ({}): {}", action, message);
        }

        Ok(json)
    }

    /// List all instances
    pub fn list_instances(&self) -> Result<Vec<InstanceInfo>> {
        let mut params = BTreeMap::new();
        params.insert("PageSize".to_string(), "50".to_string());

        let result = self.call_api("DescribeInstances", params)?;

        let instances = result
            .get("Instances")
            .and_then(|i| i.get("Instance"))
            .and_then(|i| serde_json::from_value(i.clone()).ok())
            .unwrap_or_default();

        Ok(instances)
    }

    /// Get instance by ID
    pub fn get_instance(&self, instance_id: &str) -> Result<Option<InstanceInfo>> {
        let mut params = BTreeMap::new();
        params.insert("InstanceIds".to_string(), format!("[\"{}\"]", instance_id));

        let result = self.call_api("DescribeInstances", params)?;

        let instances: Vec<InstanceInfo> = result
            .get("Instances")
            .and_then(|i| i.get("Instance"))
            .and_then(|i| serde_json::from_value(i.clone()).ok())
            .unwrap_or_default();

        Ok(instances.into_iter().next())
    }

    /// Create a Windows instance
    pub fn create_windows_instance(
        &self,
        password: &str,
        instance_name: Option<&str>,
        instance_type: Option<&str>,
    ) -> Result<String> {
        // Get available resources
        let resources = self.get_available_resources()?;

        let zone_id = resources.zone_id.clone();
        let vswitch_id = resources.vswitch_id.clone();
        let security_group_id = resources.security_group_id.clone();

        println!("Creating Windows ECS instance...");
        println!("  Zone: {}", zone_id);
        println!("  Type: {}", instance_type.unwrap_or("ecs.g7.large"));
        println!("  VSwitch: {}", vswitch_id);
        println!("  SecurityGroup: {}", security_group_id);

        // Windows Server 2025 image
        let image_id = "win2025_24H2_x64_dtc_zh-cn_40G_alibase_20260211.vhd";

        let mut params = BTreeMap::new();
        params.insert("ZoneId".to_string(), zone_id);
        params.insert(
            "InstanceType".to_string(),
            instance_type.unwrap_or("ecs.g7.large").to_string(),
        );
        params.insert("ImageId".to_string(), image_id.to_string());
        params.insert("VSwitchId".to_string(), vswitch_id);
        params.insert("SecurityGroupId".to_string(), security_group_id);
        params.insert(
            "InstanceName".to_string(),
            instance_name.unwrap_or("burncloud-test").to_string(),
        );
        params.insert("Password".to_string(), password.to_string());
        params.insert("InternetMaxBandwidthOut".to_string(), "5".to_string());
        params.insert(
            "InternetChargeType".to_string(),
            "PayByBandwidth".to_string(),
        );
        params.insert("SystemDisk.Category".to_string(), "cloud_essd".to_string());
        params.insert("SystemDisk.Size".to_string(), "40".to_string());
        params.insert("Amount".to_string(), "1".to_string());

        let result = self.call_api("RunInstances", params)?;

        let instance_id = result
            .get("InstanceIdSets")
            .and_then(|s| s.get("InstanceIdSet"))
            .and_then(|s| s.as_array())
            .and_then(|a| a.first())
            .and_then(|i| i.as_str())
            .context("Failed to get instance ID from response")?
            .to_string();

        println!("Instance created: {}", instance_id);
        Ok(instance_id)
    }

    /// Get available network resources
    fn get_available_resources(&self) -> Result<NetworkResources> {
        // Get VSwitch
        let result = self.call_api("DescribeVSwitches", BTreeMap::new())?;
        let vswitches = result
            .get("VSwitches")
            .and_then(|v| v.get("VSwitch"))
            .and_then(|v| v.as_array())
            .context("No VSwitch found")?;

        let vs = vswitches.first().context("No VSwitch available")?;
        let zone_id = vs
            .get("ZoneId")
            .and_then(|z| z.as_str())
            .unwrap_or("")
            .to_string();
        let vswitch_id = vs
            .get("VSwitchId")
            .and_then(|z| z.as_str())
            .unwrap_or("")
            .to_string();

        // Get Security Group
        let result = self.call_api("DescribeSecurityGroups", BTreeMap::new())?;
        let sgs = result
            .get("SecurityGroups")
            .and_then(|s| s.get("SecurityGroup"))
            .and_then(|s| s.as_array())
            .context("No SecurityGroup found")?;

        let sg = sgs.first().context("No SecurityGroup available")?;
        let security_group_id = sg
            .get("SecurityGroupId")
            .and_then(|z| z.as_str())
            .unwrap_or("")
            .to_string();

        Ok(NetworkResources {
            zone_id,
            vswitch_id,
            security_group_id,
        })
    }

    /// Wait for instance to be ready and return public IP
    pub fn wait_for_instance_ready(&self, instance_id: &str, timeout_secs: u64) -> Result<String> {
        println!("Waiting for instance {} to be ready...", instance_id);
        let start = std::time::Instant::now();

        loop {
            let elapsed = start.elapsed().as_secs();
            if elapsed > timeout_secs {
                anyhow::bail!("Timeout waiting for instance to be ready");
            }

            match self.get_instance(instance_id) {
                Ok(Some(instance)) => {
                    let status = instance.status.clone();
                    let ips = instance
                        .public_ip_address
                        .as_ref()
                        .map(|p| p.ip_address.clone())
                        .unwrap_or_default();

                    let ip = ips.first().map(|s| s.as_str()).unwrap_or("N/A");
                    println!("  [{}s] Status: {}, IP: {}", elapsed, status, ip);

                    if status == "Running" && !ips.is_empty() {
                        return Ok(ips[0].clone());
                    }
                }
                Ok(None) => {
                    println!("  [{}s] Instance not found (None returned)", elapsed);
                }
                Err(e) => {
                    println!("  [{}s] Error getting instance: {}", elapsed, e);
                }
            }

            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    /// Install SSH via Cloud Assistant
    pub fn install_ssh(&self, instance_id: &str) -> Result<String> {
        let script = r#"
Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0 -ErrorAction SilentlyContinue
Start-Service sshd -ErrorAction SilentlyContinue
Set-Service -Name sshd -StartupType 'Automatic' -ErrorAction SilentlyContinue
if (!(Get-NetFirewallRule -Name 'OpenSSH-Server-In-TCP' -ErrorAction SilentlyContinue)) {
    New-NetFirewallRule -Name 'OpenSSH-Server-In-TCP' -DisplayName 'OpenSSH Server' -Enabled True -Direction Inbound -Protocol TCP -Action Allow -LocalPort 22 -ErrorAction SilentlyContinue
}
Write-Host 'SSH Server installed successfully!'
"#;

        let mut params = BTreeMap::new();
        params.insert("Type".to_string(), "RunPowerShellScript".to_string());
        params.insert("InstanceId.1".to_string(), instance_id.to_string());
        params.insert("CommandContent".to_string(), script.to_string());
        params.insert("Timeout".to_string(), "300".to_string());

        let result = self.call_api("RunCommand", params)?;

        let invoke_id = result
            .get("InvokeId")
            .and_then(|i| i.as_str())
            .context("Failed to get InvokeId")?
            .to_string();

        println!("SSH installation command sent: {}", invoke_id);
        Ok(invoke_id)
    }

    /// Delete instance
    pub fn delete_instance(&self, instance_id: &str, force: bool) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("InstanceId".to_string(), instance_id.to_string());
        params.insert(
            "Force".to_string(),
            if force { "true" } else { "false" }.to_string(),
        );

        self.call_api("DeleteInstance", params)?;
        println!("Instance {} deleted", instance_id);
        Ok(())
    }

    /// Delete all instances with a specific name prefix
    pub fn delete_instances_by_prefix(&self, prefix: &str, force: bool) -> Result<usize> {
        let instances = self.list_instances()?;
        let to_delete: Vec<_> = instances
            .iter()
            .filter(|i| i.instance_name.starts_with(prefix))
            .collect();

        let count = to_delete.len();
        for inst in to_delete {
            println!(
                "Deleting instance {} ({})...",
                inst.instance_id, inst.instance_name
            );
            self.delete_instance(&inst.instance_id, force)?;
        }

        Ok(count)
    }
}

#[derive(Debug, Clone)]
struct NetworkResources {
    zone_id: String,
    vswitch_id: String,
    security_group_id: String,
}

// Simple URL encoding module
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for c in s.chars() {
            match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => encoded.push(c),
                _ => {
                    for byte in c.to_string().as_bytes() {
                        encoded.push_str(&format!("%{:02X}", byte));
                    }
                }
            }
        }
        encoded
    }
}
