#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
//! End-to-End Bundle Installation Test (Full Pipeline)
//!
//! This test runs the complete installation flow in one go.
//! For debugging individual steps, use the step-by-step tests instead.
//!
//! Run: cargo test -p burncloud-tests --test aliyun_e2e_test -- --ignored 06_e2e

use super::aliyun_api::AliyunECS;
use anyhow::{Context, Result};
use ssh2::Session;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

/// Environment file path for storing test instance info
const TEST_ENV_FILE: &str = ".env";

/// Default test server configuration
pub struct TestServerConfig {
    pub region: String,
    pub password: String,
    pub instance_name: String,
    pub instance_type: String,
}

impl Default for TestServerConfig {
    fn default() -> Self {
        Self {
            region: "cn-shenzhen".to_string(),
            password: "Burncloud@Test123".to_string(),
            instance_name: "burncloud-e2e-test".to_string(),
            instance_type: "ecs.g7.large".to_string(),
        }
    }
}

/// Saved test instance info
#[derive(Debug, Clone)]
pub struct SavedInstanceInfo {
    pub instance_id: String,
    pub public_ip: String,
    pub password: String,
}

/// Bundle E2E Test runner
pub struct BundleE2ETest {
    ecs: AliyunECS,
    config: TestServerConfig,
    instance_id: Option<String>,
    public_ip: Option<String>,
}

impl BundleE2ETest {
    /// Create new E2E test runner with default config
    pub fn new(ecs: AliyunECS) -> Self {
        Self {
            ecs,
            config: TestServerConfig::default(),
            instance_id: None,
            public_ip: None,
        }
    }

    /// Create new E2E test runner from config file with region override
    pub fn from_config_file_with_region(region: &str) -> Result<Self> {
        let ecs = AliyunECS::from_config_file_with_region(region)?;
        Ok(Self::new(ecs))
    }

    /// Save instance info to .env.test file for reuse
    pub fn save_instance_info(&self) -> Result<()> {
        let instance_id = self.instance_id.as_ref().context("No instance ID")?;
        let public_ip = self.public_ip.as_ref().context("No public IP")?;

        let content = format!(
            "# BurnCloud Test Instance Info\n\
             # Generated automatically - DO NOT DELETE until testing is complete\n\
             TEST_INSTANCE_ID={}\n\
             TEST_PUBLIC_IP={}\n\
             TEST_PASSWORD={}\n\
             TEST_REGION={}\n",
            instance_id, public_ip, self.config.password, self.config.region
        );

        fs::write(TEST_ENV_FILE, &content)?;
        println!("Instance info saved to {}", TEST_ENV_FILE);
        println!("  Instance ID: {}", instance_id);
        println!("  Public IP: {}", public_ip);
        println!("  Password: {}", self.config.password);

        Ok(())
    }

    /// Load saved instance info from .env.test file
    pub fn load_saved_instance() -> Option<SavedInstanceInfo> {
        let content = fs::read_to_string(TEST_ENV_FILE).ok()?;
        let mut instance_id = None;
        let mut public_ip = None;
        let mut password = None;

        for line in content.lines() {
            if line.starts_with("TEST_INSTANCE_ID=") {
                instance_id = Some(line.split('=').nth(1)?.to_string());
            } else if line.starts_with("TEST_PUBLIC_IP=") {
                public_ip = Some(line.split('=').nth(1)?.to_string());
            } else if line.starts_with("TEST_PASSWORD=") {
                password = Some(line.split('=').nth(1)?.to_string());
            }
        }

        Some(SavedInstanceInfo {
            instance_id: instance_id?,
            public_ip: public_ip?,
            password: password?,
        })
    }

    /// Clear saved instance info
    pub fn clear_saved_instance() {
        let _ = fs::remove_file(TEST_ENV_FILE);
    }

    /// Create test server and wait for it to be ready (or reuse existing)
    pub fn setup(&mut self) -> Result<()> {
        println!("=== Setting up test server ===");

        // Check for saved instance first
        if let Some(saved) = Self::load_saved_instance() {
            println!(
                "Found saved instance: {} ({})",
                saved.instance_id, saved.public_ip
            );

            // Verify instance still exists and is running
            if let Ok(instances) = self.ecs.list_instances() {
                if let Some(instance) = instances
                    .iter()
                    .find(|i| i.instance_id == saved.instance_id)
                {
                    if instance.status == "Running" {
                        println!("Reusing existing running instance: {}", saved.instance_id);
                        self.instance_id = Some(saved.instance_id);
                        self.public_ip = Some(saved.public_ip);
                        return Ok(());
                    } else {
                        println!("Instance exists but status is: {}", instance.status);
                    }
                }
            }
            println!("Saved instance not found or not running, creating new one...");
        }

        // Cleanup any existing test instances
        println!("Cleaning up any existing test instances...");
        let deleted = self
            .ecs
            .delete_instances_by_prefix(&self.config.instance_name, true)?;
        if deleted > 0 {
            println!("Deleted {} stale instances", deleted);
        }

        // Create instance
        let instance_id = self.ecs.create_windows_instance(
            &self.config.password,
            Some(&self.config.instance_name),
            Some(&self.config.instance_type),
        )?;

        self.instance_id = Some(instance_id.clone());

        // Wait for instance to be ready (600 seconds for Windows instances)
        let public_ip = self
            .ecs
            .wait_for_instance_ready(&instance_id, 600)
            .context("Failed to wait for instance")?;

        self.public_ip = Some(public_ip.clone());

        // Install SSH
        self.ecs.install_ssh(&instance_id)?;

        // Wait for SSH to be available (poll for up to 5 minutes for slower cloud instances)
        self.wait_for_ssh_ready(300)?;

        println!("Server ready: {}", public_ip);

        // Save instance info for reuse
        self.save_instance_info()?;

        Ok(())
    }

    /// Wait for SSH to be available
    fn wait_for_ssh_ready(&self, timeout_secs: u64) -> Result<()> {
        println!("Waiting for SSH to be available...");
        let start = std::time::Instant::now();

        loop {
            let elapsed = start.elapsed().as_secs();
            if elapsed > timeout_secs {
                anyhow::bail!("Timeout waiting for SSH to be available");
            }

            if let Ok(_) = TcpStream::connect(format!("{}:22", self.public_ip.as_ref().unwrap())) {
                println!("  SSH port is open after {} seconds", elapsed);
                // Give SSH service a few more seconds to fully initialize
                std::thread::sleep(Duration::from_secs(5));
                return Ok(());
            }

            if elapsed % 10 == 0 {
                println!("  [{}s] SSH not yet available, waiting...", elapsed);
            }
            std::thread::sleep(Duration::from_secs(5));
        }
    }

    /// Connect via SSH
    pub fn ssh_connect(&self) -> Result<Session> {
        let ip = self.public_ip.as_ref().context("Server not ready")?;

        println!("Connecting to {} via SSH...", ip);

        let tcp = TcpStream::connect(format!("{}:22", ip)).context("Failed to connect via TCP")?;

        let mut sess = Session::new().context("Failed to create SSH session")?;
        sess.set_tcp_stream(tcp);
        sess.handshake().context("SSH handshake failed")?;

        sess.userauth_password("Administrator", &self.config.password)
            .context("SSH authentication failed")?;

        println!("SSH connected!");
        Ok(sess)
    }

    /// Upload BurnCloud CLI and bundle to server
    pub fn upload_files(&self, local_cli_path: &str, local_bundle_path: &str) -> Result<()> {
        let sess = self.ssh_connect()?;

        println!("Uploading files...");

        // Create remote directory (Windows command)
        self.ssh_exec(
            &sess,
            "cmd /c \"if not exist C:\\burncloud-test mkdir C:\\burncloud-test\"",
        )?;

        // Upload CLI via SFTP
        let sftp = sess.sftp().context("Failed to open SFTP")?;

        let local_cli = Path::new(local_cli_path);
        if local_cli.exists() {
            println!("  Uploading burncloud.exe...");
            // Use Windows path format for SFTP
            let remote_path = std::path::Path::new("C:/burncloud-test/burncloud.exe");
            let mut local_file =
                std::fs::File::open(local_cli).context("Failed to open local CLI")?;
            let mut remote_file = sftp
                .create(remote_path)
                .context("Failed to create remote file")?;
            let mut buffer = Vec::new();
            local_file.read_to_end(&mut buffer)?;
            remote_file.write_all(&buffer)?;
            println!("  CLI uploaded");
        } else {
            anyhow::bail!("Local CLI not found: {}", local_cli_path);
        }

        // Upload bundle directory
        let local_bundle = Path::new(local_bundle_path);
        if local_bundle.exists() {
            println!("  Uploading bundle...");
            self.upload_directory(&sftp, local_bundle, "C:/burncloud-test/openclaw-bundle")?;
            println!("  Bundle uploaded");
        } else {
            anyhow::bail!("Local bundle not found: {}", local_bundle_path);
        }

        println!("Files uploaded successfully!");
        Ok(())
    }

    /// Upload directory recursively via SFTP
    fn upload_directory(&self, sftp: &ssh2::Sftp, local: &Path, remote: &str) -> Result<()> {
        // Create remote directory (mode 0755 = rwxr-xr-x)
        let _ = sftp.mkdir(Path::new(remote), 0o755); // Ignore error if exists

        for entry in std::fs::read_dir(local).context("Failed to read local directory")? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            // Use forward slashes for SFTP on Windows
            let remote_path = format!("{}/{}", remote, name);

            if path.is_dir() {
                self.upload_directory(sftp, &path, &remote_path)?;
            } else {
                let mut local_file = std::fs::File::open(&path)?;
                let mut remote_file = sftp.create(Path::new(&remote_path))?;
                let mut buffer = Vec::new();
                local_file.read_to_end(&mut buffer)?;
                remote_file.write_all(&buffer)?;
            }
        }

        Ok(())
    }

    /// Execute SSH command and return output
    pub fn ssh_exec(&self, sess: &Session, cmd: &str) -> Result<String> {
        let mut channel = sess
            .channel_session()
            .context("Failed to open SSH channel")?;

        channel.exec(cmd).context("Failed to execute SSH command")?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .context("Failed to read SSH output")?;

        channel.wait_close().context("Failed to close channel")?;

        Ok(output)
    }

    /// Run BurnCloud installation
    pub fn run_installation(&self) -> Result<String> {
        let sess = self.ssh_connect()?;

        println!("Running BurnCloud installation...");

        let cmd = "set RUST_LOG=info && C:\\burncloud-test\\burncloud.exe install openclaw --bundle C:\\burncloud-test\\openclaw-bundle --auto-deps";

        println!("  Command: {}", cmd);

        let output = self.ssh_exec(&sess, cmd)?;

        println!("{}", output);
        Ok(output)
    }

    /// Verify installation
    pub fn verify_installation(&self) -> Result<bool> {
        let sess = self.ssh_connect()?;

        println!("Verifying installation...");

        // Check status
        let status_output = self.ssh_exec(
            &sess,
            "C:\\burncloud-test\\burncloud.exe install openclaw --status",
        )?;

        println!("Status: {}", status_output);

        // Find Node.js installation directory dynamically
        let nodejs_base = "C:\\Users\\Administrator\\AppData\\Local\\burncloud\\nodejs";
        let nodejs_dir =
            self.ssh_exec(&sess, &format!("dir /b {} | findstr node-v", nodejs_base))?;
        let nodejs_dir = nodejs_dir.trim();
        let nodejs_path = format!("{}\\{}", nodejs_base, nodejs_dir);

        println!("Node.js path: {}", nodejs_path);

        // Check Node.js
        let node_output = self.ssh_exec(
            &sess,
            &format!("set PATH={};%PATH% && node --version", nodejs_path),
        )?;

        println!("Node.js: {}", node_output.trim());

        // Check OpenClaw (npm global packages are in %APPDATA%\npm)
        let openclaw_output =
            self.ssh_exec(&sess, &format!("set PATH={};C:\\Users\\Administrator\\AppData\\Roaming\\npm;%PATH% && openclaw --version", nodejs_path))?;

        println!("OpenClaw: {}", openclaw_output.trim());

        // Check Git
        let git_output =
            self.ssh_exec(&sess, "\"C:\\Program Files\\Git\\cmd\\git.exe\" --version")?;
        println!("Git: {}", git_output.trim());

        // Check if PATH was correctly updated by running from fresh cmd
        // This tests if setx worked - the PATH should include our paths
        let path_check = self.ssh_exec(&sess, "reg query \"HKCU\\Environment\" /v PATH")?;
        println!("User PATH: {}", path_check.trim());

        let success = status_output.contains("Installed")
            && node_output.starts_with("v")
            && openclaw_output.contains("OpenClaw");

        Ok(success)
    }

    /// Cleanup test resources
    pub fn cleanup(&self, force: bool) -> Result<()> {
        if let Some(instance_id) = &self.instance_id {
            println!("Cleaning up instance {}...", instance_id);
            self.ecs.delete_instance(instance_id, force)?;
        }
        Ok(())
    }

    /// Get public IP
    pub fn public_ip(&self) -> Option<&String> {
        self.public_ip.as_ref()
    }

    /// Get instance ID
    pub fn instance_id(&self) -> Option<&String> {
        self.instance_id.as_ref()
    }
}

// Keep internal tests for module verification
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires Aliyun credentials
    fn test_e2e_bundle_installation() {
        let mut test = BundleE2ETest::from_config_file_with_region("cn-shenzhen")
            .expect("Failed to create test runner");

        // Setup server
        test.setup().expect("Setup failed");

        // Upload files - use CARGO_MANIFEST_DIR to get project root
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let project_root = std::path::Path::new(&manifest_dir)
            .parent()
            .and_then(|p| p.parent())
            .expect("Failed to get project root");

        let cli_path = project_root.join("target/release/burncloud.exe");
        let bundle_path = project_root.join("target/release/openclaw-bundle");

        test.upload_files(cli_path.to_str().unwrap(), bundle_path.to_str().unwrap())
            .expect("Upload failed");

        // Run installation
        test.run_installation().expect("Installation failed");

        // Verify
        let success = test.verify_installation().expect("Verification failed");
        assert!(success, "Installation verification failed");

        // NOTE: Instance is preserved for reuse. Check .env.test for instance info.
        // To cleanup manually, run: test.cleanup(true)
        println!("=== Test completed successfully ===");
        println!("Instance preserved for reuse. Info saved to .env.test");
    }
}
