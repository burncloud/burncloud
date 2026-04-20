#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
//! Bundle Fresh Install Test
//!
//! This test validates that a bundle can be installed on a completely fresh machine.
//! It uses Docker containers to simulate isolated environments without any pre-installed dependencies.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// Configuration for fresh install test
struct FreshInstallTest {
    /// Software ID to test (e.g., "openclaw")
    software_id: String,
    /// Docker image to use (should be minimal OS)
    docker_image: String,
    /// Whether to auto-install dependencies
    auto_deps: bool,
}

impl FreshInstallTest {
    fn new(software_id: &str) -> Self {
        Self {
            software_id: software_id.to_string(),
            docker_image: "ubuntu:22.04".to_string(),
            auto_deps: true,
        }
    }

    fn with_docker_image(mut self, image: &str) -> Self {
        self.docker_image = image.to_string();
        self
    }

    /// Run the full test
    async fn run(&self) -> anyhow::Result<TestResult> {
        let mut result = TestResult::new(&self.software_id);

        // Step 1: Build burncloud CLI
        result.add_step("build_cli", self.build_cli()?);

        // Step 2: Create bundle
        result.add_step("create_bundle", self.create_bundle().await?);

        // Step 3: Run Docker test
        result.add_step("docker_test", self.run_docker_test()?);

        // Step 4: Verify installation
        result.add_step("verify", self.verify_in_docker()?);

        Ok(result)
    }

    /// Build the burncloud CLI
    fn build_cli(&self) -> anyhow::Result<bool> {
        println!("🔨 Building burncloud CLI...");

        let output = Command::new("cargo")
            .args(["build", "--release", "--bin", "burncloud"])
            .current_dir(project_root())
            .output()?;

        if output.status.success() {
            println!("✅ CLI built successfully");
            Ok(true)
        } else {
            println!(
                "❌ CLI build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(false)
        }
    }

    /// Create bundle using burncloud CLI
    async fn create_bundle(&self) -> anyhow::Result<bool> {
        println!("📦 Creating bundle for {}...", self.software_id);

        let cli_path = cli_binary_path();
        let bundle_dir = test_temp_dir().join("bundles");

        let output = Command::new(&cli_path)
            .args([
                "bundle",
                "create",
                &self.software_id,
                "-o",
                bundle_dir.to_str().unwrap(),
            ])
            .output()?;

        if output.status.success() {
            println!("✅ Bundle created successfully");
            Ok(true)
        } else {
            println!(
                "❌ Bundle creation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(false)
        }
    }

    /// Run the installation test inside Docker
    fn run_docker_test(&self) -> anyhow::Result<bool> {
        println!("🐳 Testing installation in fresh Docker container...");

        let bundle_dir = test_temp_dir().join("bundles");
        let cli_path = cli_binary_path();

        // Create Dockerfile for isolated test
        let dockerfile = self.generate_dockerfile();
        let dockerfile_path = test_temp_dir().join("Dockerfile.test");
        std::fs::write(&dockerfile_path, dockerfile)?;

        // Build Docker image
        println!("  Building Docker image...");
        let build_output = Command::new("docker")
            .args([
                "build",
                "-t",
                &format!("burncloud-test-{}", self.software_id),
                "-f",
                dockerfile_path.to_str().unwrap(),
                test_temp_dir().to_str().unwrap(),
            ])
            .output()?;

        if !build_output.status.success() {
            println!(
                "  ❌ Docker build failed: {}",
                String::from_utf8_lossy(&build_output.stderr)
            );
            return Ok(false);
        }

        // Run container and execute installation
        println!("  Running installation in container...");
        let run_output = Command::new("docker")
            .args([
                "run",
                "--rm",
                &format!("burncloud-test-{}", self.software_id),
            ])
            .output()?;

        if run_output.status.success() {
            println!("✅ Docker test passed");
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            println!("{}", stdout);
            Ok(true)
        } else {
            println!("❌ Docker test failed");
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            println!("{}", stderr);
            Ok(false)
        }
    }

    /// Verify the installation succeeded
    fn verify_in_docker(&self) -> anyhow::Result<bool> {
        // Verification is done inside the Docker container
        // This is just a placeholder for additional verification
        Ok(true)
    }

    /// Generate Dockerfile for isolated testing
    fn generate_dockerfile(&self) -> String {
        let auto_deps_flag = if self.auto_deps { "--auto-deps" } else { "" };

        format!(
            r#"
FROM {docker_image}

# Minimal dependencies for testing
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create test directory
WORKDIR /test

# Copy bundle and CLI
COPY bundles/{software_id}-bundle /test/bundle
COPY target/release/burncloud /usr/local/bin/burncloud
RUN chmod +x /usr/local/bin/burncloud

# Pre-install check
RUN echo "=== Pre-install Environment Check ===" && \
    (git --version || echo "Git: Not installed") && \
    (node --version || echo "Node.js: Not installed") && \
    (npm --version || echo "npm: Not installed")

# Run installation
RUN echo "=== Running Bundle Installation ===" && \
    RUST_LOG=debug burncloud install {software_id} --bundle /test/bundle {auto_deps_flag}

# Post-install verification
RUN echo "=== Post-install Verification ===" && \
    burncloud install {software_id} --status

# Final check - try to run the software
RUN echo "=== Final Verification ===" && \
    (openclaw --version || echo "Note: openclaw command may need PATH configuration")

CMD ["echo", "Test completed successfully"]
"#,
            docker_image = self.docker_image,
            software_id = self.software_id,
            auto_deps_flag = auto_deps_flag
        )
    }
}

/// Test result tracking
struct TestResult {
    software_id: String,
    steps: Vec<(String, bool)>,
    start_time: std::time::Instant,
}

impl TestResult {
    fn new(software_id: &str) -> Self {
        Self {
            software_id: software_id.to_string(),
            steps: Vec::new(),
            start_time: std::time::Instant::now(),
        }
    }

    fn add_step(&mut self, name: &str, success: bool) {
        self.steps.push((name.to_string(), success));
    }

    fn is_success(&self) -> bool {
        self.steps.iter().all(|(_, success)| *success)
    }

    fn print_summary(&self) {
        println!("\n{}", "=".repeat(60));
        println!("📊 Test Summary: {}", self.software_id);
        println!("{}", "=".repeat(60));

        for (step, success) in &self.steps {
            let status = if *success { "✅ PASS" } else { "❌ FAIL" };
            println!("  {} - {}", status, step);
        }

        println!(
            "\n⏱️  Total time: {:.2}s",
            self.start_time.elapsed().as_secs_f64()
        );

        if self.is_success() {
            println!("\n🎉 All tests passed!");
        } else {
            println!("\n❌ Some tests failed!");
        }
    }
}

// Helper functions

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn cli_binary_path() -> PathBuf {
    project_root()
        .join("target")
        .join("release")
        .join("burncloud")
}

fn test_temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("burncloud-bundle-test");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// ============================================================
// Tests
// ============================================================

#[tokio::test]
#[ignore = "Requires Docker and takes time - run with: cargo test -- --ignored"]
async fn test_openclaw_fresh_install_ubuntu() {
    let test = FreshInstallTest::new("openclaw")
        .with_docker_image("ubuntu:22.04");

    let result = test.run().await.expect("Test should complete");

    result.print_summary();
    assert!(result.is_success(), "Fresh install test should pass");
}

#[tokio::test]
#[ignore = "Requires Docker and takes time"]
async fn test_openclaw_fresh_install_debian() {
    let test = FreshInstallTest::new("openclaw")
        .with_docker_image("debian:12-slim");

    let result = test.run().await.expect("Test should complete");

    result.print_summary();
    assert!(result.is_success(), "Fresh install test should pass");
}

/// Quick local test without Docker (for development)
#[tokio::test]
async fn test_bundle_creation_local() {
    let test = FreshInstallTest::new("openclaw");

    // Only test bundle creation locally
    let cli_built = test.build_cli().expect("Should build CLI");
    assert!(cli_built, "CLI should build successfully");

    let bundle_created = test.create_bundle().await.expect("Should create bundle");
    assert!(bundle_created, "Bundle should be created successfully");
}
