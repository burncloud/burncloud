//! Real Git + the real Rust command, with only Cargo check execution stubbed.
#![allow(
    clippy::panic_in_result_fn,
    reason = "Regression tests return setup errors with ? and use assertions for behavioral expectations"
)]
use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tempfile::{tempdir, TempDir};

const BIN: &str = env!("CARGO_BIN_EXE_burncloud-code");
const HOOK: &str = include_str!("../../../../.github/hooks/pre-commit");
const MESSAGE_HOOK: &str = include_str!("../../../../.github/hooks/commit-msg");
const PREVIOUS_HOOK: &str = r#"#!/bin/sh
# BurnCloud managed pre-commit hook. Reinstall with: cargo run -- code init
set -eu

root=$(git rev-parse --show-toplevel)
cd "$root"
cargo run --quiet -- code test --staged

# Keep the previous hook's interpreter, arguments and exit status intact.
hooks=$(git rev-parse --git-path hooks)
if [ -x "$hooks/pre-commit.burncloud-original" ]; then
    exec "$hooks/pre-commit.burncloud-original" "$@"
fi
"#;
const FULL: &str = "fmt --all -- --check\ntest --workspace --no-default-features\nclippy --workspace --all-targets --no-default-features\ndeny check\n";

#[derive(Deserialize)]
struct CheckSummary {
    status: String,
    unit: Option<CheckCounts>,
    integration: Option<CheckCounts>,
    steps: Vec<CheckStep>,
}

#[derive(Deserialize)]
struct CheckCounts {
    passed: u64,
}

#[derive(Deserialize)]
struct CheckStep {
    log: PathBuf,
}

#[derive(Deserialize, Serialize)]
struct FixtureMetadata {
    workspace_members: Vec<String>,
    packages: Vec<FixturePackage>,
}

#[derive(Deserialize, Serialize)]
struct FixturePackage {
    id: String,
    name: String,
    manifest_path: PathBuf,
    dependencies: Vec<FixtureDependency>,
}

#[derive(Default, Deserialize, Serialize)]
struct FixtureDependency {
    path: PathBuf,
    #[serde(default)]
    rename: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    target: Option<String>,
}

fn stub() -> Result<&'static PathBuf> {
    static STUB: OnceLock<Result<(TempDir, PathBuf), String>> = OnceLock::new();
    match STUB.get_or_init(|| {
        (|| -> Result<_> {
            let dir = tempdir()?;
            let binary = dir
                .path()
                .join(if cfg!(windows) { "cargo.exe" } else { "cargo" });
            let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cargo_stub.rs");
            let status = Command::new("rustc")
                .args(["--edition=2021", "-Dwarnings"])
                .arg(source)
                .arg("-o")
                .arg(&binary)
                .status()?;
            anyhow::ensure!(status.success(), "Cannot compile native Cargo stub");
            fs::copy(
                &binary,
                dir.path().join(if cfg!(windows) {
                    "rustup.exe"
                } else {
                    "rustup"
                }),
            )?;
            Ok((dir, binary))
        })()
        .map_err(|e| e.to_string())
    }) {
        Ok((_, binary)) => Ok(binary),
        Err(error) => anyhow::bail!("{error}"),
    }
}

struct Fixture {
    _temp: TempDir,
    repo: PathBuf,
    base: PathBuf,
    path: OsString,
}
impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempdir()?;
        let base = temp.path().to_path_buf();
        let repo = base.join("repo with spaces");
        fs::create_dir_all(&repo)?;
        fs::write(base.join("gitconfig"), "")?;
        let mut paths = vec![stub()?.parent().context("Stub parent")?.to_path_buf()];
        paths.extend(env::split_paths(&env::var_os("PATH").context("PATH")?));
        let fixture = Self {
            _temp: temp,
            repo,
            base,
            path: env::join_paths(paths)?,
        };
        fixture.git(&["init", "-q"])?;
        fixture.git(&["config", "core.autocrlf", "false"])?;
        fixture.git(&["config", "user.name", "Hook Test"])?;
        fixture.git(&["config", "user.email", "hook-test@example.invalid"])?;
        for file in [
            "Cargo.toml",
            "deny.toml",
            "crates/a/Cargo.toml",
            "crates/b/Cargo.toml",
            "crates/c/Cargo.toml",
            "crates/a/src/lib.rs",
            "crates/b/src/lib.rs",
            "crates/c/src/lib.rs",
        ] {
            fixture.write(file, "// fixture\n")?;
        }
        fixture.write(".github/hooks/pre-commit", HOOK)?;
        fixture.write(".github/hooks/commit-msg", MESSAGE_HOOK)?;
        let package = |name: &str, directory: &str, dependencies: &[&str]| {
            json!({
                "id": name, "name": name, "manifest_path": fixture.repo.join(directory).join("Cargo.toml"),
                "dependencies": dependencies.iter().map(|path| json!({"path":fixture.repo.join(path)})).collect::<Vec<_>>()
            })
        };
        let metadata = json!({"workspace_members":["burncloud","a","b","c"],"packages":[
            package("burncloud", "", &["crates/b"]), package("a", "crates/a", &[]),
            package("b", "crates/b", &["crates/a"]), package("c", "crates/c", &[])
        ]});
        fs::write(
            fixture.base.join("metadata.json"),
            serde_json::to_vec(&metadata)?,
        )?;
        fixture.git(&["add", "."])?;
        Ok(fixture)
    }
    fn command(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut cmd = Command::new(program);
        cmd.current_dir(&self.repo)
            .env("PATH", &self.path)
            .env("GIT_CONFIG_GLOBAL", self.base.join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("CHECK_LOG", self.base.join("checks.log"))
            .env("INSTALL_LOG", self.base.join("install.log"))
            .env("TOOL_STATE_DIR", self.base.join("tools"))
            .env("METADATA_FILE", self.base.join("metadata.json"))
            .env("CODE_TEST_BIN", BIN);
        cmd
    }
    fn git(&self, args: &[&str]) -> Result<Output> {
        success(self.command("git").args(args).output()?)
    }
    fn run(&self, args: &[&str]) -> Result<Output> {
        success(self.command(BIN).args(args).output()?)
    }
    fn write(&self, path: &str, content: &str) -> Result<()> {
        let target = self.repo.join(path);
        fs::create_dir_all(target.parent().context("File parent")?)?;
        Ok(fs::write(target, content)?)
    }
    fn log(&self) -> Result<String> {
        Ok(fs::read_to_string(self.base.join("checks.log"))?)
    }
    fn hook(&self) -> PathBuf {
        self.repo.join(".git/hooks/pre-commit")
    }
    fn saved(&self) -> PathBuf {
        self.repo.join(".git/hooks/pre-commit.burncloud-original")
    }
    fn latest(&self) -> Result<CheckSummary> {
        Ok(serde_json::from_slice(&fs::read(
            self.repo.join(".git/burncloud/checks/latest.json"),
        )?)?)
    }
    fn seed(&self) -> Result<()> {
        self.git(&["commit", "-qm", "seed"])?;
        Ok(())
    }
}
fn success(output: Output) -> Result<Output> {
    anyhow::ensure!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}
fn failed(output: Output, message: &str) {
    assert!(!output.status.success(), "command unexpectedly succeeded");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(text.contains(message), "missing {message:?}: {text}");
}
fn executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[test]
fn install_repeat_commit_and_legacy_upgrade() -> Result<()> {
    let f = Fixture::new()?;
    f.run(&["init"])?;
    f.run(&["init"])?;
    assert!(!f.saved().exists());
    f.seed()?;
    assert_eq!(f.log()?, FULL);
    let message = String::from_utf8(f.git(&["log", "-1", "--format=%B"])?.stdout)?;
    assert!(message.contains("BurnCloud-Checks: ✅ PASS"));
    assert!(
        message.contains("BurnCloud-Tests: ✅ PASS (3 passed, 0 failed, 1 ignored; unit 2 passed")
    );
    assert!(message.contains("BurnCloud-Clippy: ✅ PASS"));
    assert!(message.contains("BurnCloud-Deny: ✅ PASS"));
    assert_eq!(f.latest()?.status, "passed");
    assert!(!f.repo.join(".env").exists());
    fs::write(f.hook(), PREVIOUS_HOOK)?;
    fs::write(f.saved(), "keep me")?;
    f.run(&["init"])?;
    assert_eq!(fs::read_to_string(f.hook())?, HOOK);
    assert_eq!(fs::read_to_string(f.saved())?, "keep me");
    Ok(())
}

#[test]
fn init_installs_missing_tools_once() -> Result<()> {
    let f = Fixture::new()?;
    success(
        f.command(BIN)
            .arg("init")
            .env("MISSING_TOOLS", "fmt,clippy,deny")
            .output()?,
    )?;
    let installed = fs::read_to_string(f.base.join("install.log"))?;
    assert_eq!(
        installed,
        "rustup component add rustfmt\nrustup component add clippy\ncargo install --locked cargo-deny\n"
    );
    success(
        f.command(BIN)
            .arg("init")
            .env("MISSING_TOOLS", "fmt,clippy,deny")
            .output()?,
    )?;
    assert_eq!(fs::read_to_string(f.base.join("install.log"))?, installed);
    assert!(f.hook().exists());
    assert!(f.repo.join(".git/hooks/commit-msg").exists());
    Ok(())
}

#[test]
fn failed_tool_install_leaves_hooks_unchanged_and_can_retry() -> Result<()> {
    let f = Fixture::new()?;
    failed(
        f.command(BIN)
            .arg("init")
            .env("MISSING_TOOL", "deny")
            .env("FAIL_INSTALL", "deny")
            .output()?,
        "Could not install deny",
    );
    assert!(!f.hook().exists());
    assert!(!f.repo.join(".git/hooks/commit-msg").exists());
    success(
        f.command(BIN)
            .arg("init")
            .env("MISSING_TOOL", "deny")
            .output()?,
    )?;
    assert!(f.hook().exists());
    Ok(())
}

#[test]
fn receipt_uses_matching_index_and_replaces_old_footer_on_amend() -> Result<()> {
    let f = Fixture::new()?;
    f.run(&["init"])?;
    f.run(&["test", "--staged"])?;
    let log = String::from_utf8(f.run(&["test", "--last"])?.stdout)?;
    assert!(log.contains("✅ test: passed"));
    assert!(log.contains("Tests: 3 passed, 0 failed, 1 ignored"));
    let report = f.latest()?;
    assert_eq!(report.unit.context("unit counts")?.passed, 2);
    assert_eq!(report.integration.context("integration counts")?.passed, 1);
    assert!(report.steps[0].log.exists());
    f.write("crates/a/src/lib.rs", "changed after checks")?;
    let message_file = f.base.join("message.txt");
    fs::write(&message_file, "subject\n")?;
    failed(
        f.command(BIN).arg("stamp").arg(&message_file).output()?,
        "Working tree changed after checks",
    );
    f.git(&["add", "."])?;
    failed(
        f.command(BIN).arg("stamp").arg(&message_file).output()?,
        "Staged tree or HEAD changed",
    );
    f.git(&["commit", "-qm", "subject"])?;
    let message = String::from_utf8(f.git(&["log", "-1", "--format=%B"])?.stdout)?;
    assert_eq!(message.matches("BurnCloud-Checks: ✅ PASS").count(), 1);
    f.write("crates/a/src/lib.rs", "amended")?;
    f.git(&["add", "."])?;
    f.git(&["commit", "--amend", "--no-edit", "-q"])?;
    let amended = String::from_utf8(f.git(&["log", "-1", "--format=%B"])?.stdout)?;
    assert_eq!(amended.matches("BurnCloud-Checks: ✅ PASS").count(), 1);
    assert_eq!(amended.matches("BurnCloud-Checks-Tree:").count(), 1);
    Ok(())
}

#[test]
fn existing_message_hook_runs_and_docs_only_receipt_is_skipped() -> Result<()> {
    let f = Fixture::new()?;
    let hook = f.repo.join(".git/hooks/commit-msg");
    fs::write(
        &hook,
        "#!/bin/sh\necho original-message >> \"$CHECK_LOG\"\n",
    )?;
    executable(&hook)?;
    f.run(&["init"])?;
    assert!(f
        .repo
        .join(".git/hooks/commit-msg.burncloud-original")
        .exists());
    f.seed()?;
    assert!(f.log()?.contains("original-message\n"));
    f.write("README.md", "documentation")?;
    f.git(&["add", "."])?;
    f.git(&["commit", "-qm", "docs"])?;
    let message = String::from_utf8(f.git(&["log", "-1", "--format=%B"])?.stdout)?;
    assert!(message.contains("BurnCloud-Checks: ➖ SKIP (no code checks executed)"));
    assert!(message.contains("BurnCloud-Tests: ➖ SKIP"));
    assert_eq!(f.latest()?.status, "skipped");
    Ok(())
}

#[test]
fn each_failed_pre_commit_gate_stops_and_blocks_commit() -> Result<()> {
    let f = Fixture::new()?;
    f.run(&["init"])?;
    for (index, check) in ["fmt", "test", "clippy", "deny"].iter().enumerate() {
        fs::write(f.base.join("checks.log"), "")?;
        let output = f
            .command("git")
            .args(["commit", "-qm", "blocked"])
            .env("FAIL_CHECK", check)
            .output()?;
        assert!(!output.status.success(), "{check} unexpectedly allowed commit");
        if *check == "test" {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(text.contains("Check failed"), "missing code-test failure: {text}");
        }
        assert_eq!(
            f.log()?,
            FULL.lines().take(index + 1).collect::<Vec<_>>().join("\n") + "\n"
        );
        if *check == "fmt" {
            assert!(!f.repo.join(".git/burncloud/checks/latest.json").exists());
        } else if *check == "test" {
            assert_eq!(f.latest()?.status, "failed");
        } else {
            assert_eq!(f.latest()?.status, "passed");
        }
        assert!(!f
            .command("git")
            .args(["rev-parse", "--verify", "HEAD"])
            .output()?
            .status
            .success());
    }
    Ok(())
}

#[test]
fn code_test_does_not_bootstrap_unrelated_quality_tools() -> Result<()> {
    let f = Fixture::new()?;
    success(
        f.command(BIN)
            .args(["test", "--staged"])
            .env("MISSING_TOOLS", "fmt,clippy,deny")
            .output()?,
    )?;
    assert!(!f.base.join("install.log").exists());
    assert_eq!(f.log()?, "test --workspace --no-default-features\n");
    Ok(())
}

#[test]
fn existing_hook_failure_propagates_after_checks() -> Result<()> {
    let f = Fixture::new()?;
    let original = "#!/bin/sh\necho original >> \"$CHECK_LOG\"\nexit 7\n";
    fs::write(f.hook(), original)?;
    executable(&f.hook())?;
    f.run(&["init"])?;
    f.run(&["init"])?;
    assert!(!f
        .command("git")
        .args(["commit", "-qm", "blocked"])
        .output()?
        .status
        .success());
    assert_eq!(f.log()?, FULL.to_owned() + "original\n");
    assert_eq!(fs::read_to_string(f.saved())?, original);
    fs::write(f.base.join("checks.log"), "")?;
    failed(
        f.command("git")
            .args(["commit", "-qm", "blocked"])
            .env("FAIL_CHECK", "fmt")
            .output()?,
        "Check failed",
    );
    assert_eq!(f.log()?, "fmt --all -- --check\n");
    Ok(())
}

#[test]
fn staged_mode_rejects_partial_staging_and_untracked_files() -> Result<()> {
    let f = Fixture::new()?;
    f.write("Cargo.toml", "unstaged")?;
    failed(
        f.command(BIN)
            .args(["test", "--staged", "--plan"])
            .output()?,
        "stage or stash",
    );
    f.git(&["add", "."])?;
    f.write("new.rs", "untracked")?;
    failed(
        f.command(BIN).args(["test", "--staged"]).output()?,
        "stage or stash",
    );
    assert!(!f.base.join("checks.log").exists());
    Ok(())
}

#[test]
fn refuses_custom_hooks_backup_and_install_lock() -> Result<()> {
    let f = Fixture::new()?;
    f.git(&["config", "core.hooksPath", "shared-hooks"])?;
    failed(f.command(BIN).arg("init").output()?, "core.hooksPath");
    assert!(!f.hook().exists());
    f.git(&["config", "--unset", "core.hooksPath"])?;
    fs::write(f.hook(), "original")?;
    fs::write(f.saved(), "saved")?;
    failed(f.command(BIN).arg("init").output()?, "already exists");
    assert_eq!(fs::read_to_string(f.hook())?, "original");
    assert_eq!(fs::read_to_string(f.saved())?, "saved");
    fs::remove_file(f.saved())?;
    let pending = f.repo.join(".git/hooks/pre-commit.burncloud-pending");
    fs::write(&pending, "in progress")?;
    assert!(!f.command(BIN).arg("init").output()?.status.success());
    assert_eq!(fs::read_to_string(pending)?, "in progress");
    assert_eq!(fs::read_to_string(f.hook())?, "original");
    Ok(())
}

#[test]
fn invalid_repositories_fail() -> Result<()> {
    let f = Fixture::new()?;
    assert!(!f
        .command(BIN)
        .arg("init")
        .current_dir(&f.base)
        .output()?
        .status
        .success());
    fs::remove_file(f.repo.join(".github/hooks/pre-commit"))?;
    failed(
        f.command(BIN).arg("init").output()?,
        "BurnCloud source checkout",
    );
    Ok(())
}

#[test]
fn worktree_subdirectory_uses_common_hooks() -> Result<()> {
    let f = Fixture::new()?;
    f.seed()?;
    let linked = f.base.join("linked worktree");
    success(
        f.command("git")
            .args(["worktree", "add", "-b", "linked"])
            .arg(&linked)
            .output()?,
    )?;
    success(
        f.command(BIN)
            .arg("init")
            .current_dir(linked.join(".github"))
            .output()?,
    )?;
    assert_eq!(fs::read_to_string(f.hook())?, HOOK);
    success(
        f.command("git")
            .args(["commit", "--allow-empty", "-qm", "empty"])
            .current_dir(linked)
            .output()?,
    )?;
    assert!(!f.base.join("checks.log").exists()); // Explicitly no changes selected.
    Ok(())
}

#[test]
fn selects_reverse_dependency_closure_not_unrelated_packages() -> Result<()> {
    let f = Fixture::new()?;
    f.seed()?;
    f.write("crates/a/src/lib.rs", "changed")?;
    let output = f.run(&["test", "--plan"])?;
    let text = String::from_utf8(output.stdout)?;
    assert!(text.contains("Direct packages: a"));
    assert!(text.contains("Affected packages: a, b, burncloud"));
    assert!(text.contains("cargo test -p a -p b -p burncloud --no-default-features"));
    assert!(!text.contains("-p c"));
    assert!(!f.base.join("checks.log").exists());
    f.run(&["test"])?;
    assert!(f.log()?.contains("test -p a -p b -p burncloud"));
    Ok(())
}

#[test]
fn test_selection_ignores_non_test_config_and_scopes_package_manifests() -> Result<()> {
    let f = Fixture::new()?;
    f.seed()?;

    f.write(".github/test-plan/known-test-baseline.txt", "changed")?;
    f.write("deny.toml", "changed")?;
    let text = String::from_utf8(f.run(&["test", "--plan"])?.stdout)?;
    assert!(text.contains("No test-relevant changes selected; no tests executed."));
    assert!(!text.contains("cargo test --workspace"));

    f.write("crates/a/src/lib.rs", "changed")?;
    let text = String::from_utf8(f.run(&["test", "--plan"])?.stdout)?;
    assert!(text.contains("Affected packages: a, b, burncloud"));
    assert!(text.contains("cargo test -p a -p b -p burncloud --no-default-features"));
    assert!(!text.contains("cargo test --workspace"));

    f.git(&["add", "."])?;
    f.seed()?;
    f.write("crates/a/Cargo.toml", "change")?;
    let text = String::from_utf8(f.run(&["test", "--plan"])?.stdout)?;
    assert!(text.contains("Affected packages: a, b, burncloud"));
    assert!(!text.contains("cargo test --workspace"));

    f.git(&["checkout", "--", "."])?;
    f.write("Cargo.toml", "change")?;
    let text = String::from_utf8(f.run(&["test", "--plan"])?.stdout)?;
    assert!(text.contains("cargo test --workspace --no-default-features"));

    f.git(&["checkout", "--", "."])?;
    f.write(".cargo/config.toml", "change")?;
    let text = String::from_utf8(f.run(&["test", "--plan"])?.stdout)?;
    assert!(text.contains("cargo test --workspace --no-default-features"));

    fs::remove_file(f.repo.join(".cargo/config.toml"))?;
    f.write("unknown.file", "change")?;
    let text = String::from_utf8(f.run(&["test", "--plan"])?.stdout)?;
    assert!(text.contains("cargo test --workspace --no-default-features"));

    let text = String::from_utf8(f.run(&["test", "--all", "--plan"])?.stdout)?;
    assert!(text.contains("--all requested"));
    Ok(())
}

#[test]
fn clean_tree_and_docs_report_no_execution() -> Result<()> {
    let f = Fixture::new()?;
    f.seed()?;
    let output = f.run(&["test"])?;
    assert!(String::from_utf8(output.stdout)?.contains("no checks executed"));
    f.write("README.md", "documentation")?;
    assert!(String::from_utf8(f.run(&["test"])?.stdout)?.contains("no checks executed"));
    f.write("docs/architecture.md", "shared contract")?;
    let text = String::from_utf8(f.run(&["test", "--plan"])?.stdout)?;
    assert!(text.contains("unclassified path: docs/architecture.md"));
    fs::remove_file(f.repo.join("docs/architecture.md"))?;
    assert!(!f.base.join("checks.log").exists());
    // Documentation/data inside a package is not silently excluded.
    f.write("crates/c/README.md", "could be include_str data")?;
    assert!(String::from_utf8(f.run(&["test", "--plan"])?.stdout)?.contains("Affected packages: c"));
    Ok(())
}

#[test]
fn base_includes_committed_branch_work_and_invalid_base_fails() -> Result<()> {
    let f = Fixture::new()?;
    f.seed()?;
    f.git(&["branch", "baseline"])?;
    f.write("crates/b/src/lib.rs", "change")?;
    f.git(&["add", "."])?;
    f.seed()?;
    let text = String::from_utf8(f.run(&["test", "--base", "baseline", "--plan"])?.stdout)?;
    assert!(text.contains("Affected packages: b, burncloud"));
    assert!(!text.contains("-p a"));
    assert!(!f
        .command(BIN)
        .args(["test", "--base", "does-not-exist"])
        .output()?
        .status
        .success());
    assert!(!f
        .command(BIN)
        .args(["test", "--base", "baseline", "--staged"])
        .output()?
        .status
        .success());
    Ok(())
}

#[test]
fn rename_and_delete_preserve_old_package_impact() -> Result<()> {
    let f = Fixture::new()?;
    f.seed()?;
    f.git(&["mv", "crates/a/src/lib.rs", "crates/c/src/moved file.rs"])?;
    let text = String::from_utf8(f.run(&["test", "--staged", "--plan"])?.stdout)?;
    assert!(text.contains("Affected packages: a, b, burncloud, c"));
    assert!(text.contains("crates/c/src/moved file.rs"));
    Ok(())
}

#[test]
fn broken_metadata_is_an_error_not_a_pass() -> Result<()> {
    let f = Fixture::new()?;
    fs::write(f.base.join("metadata.json"), "not json")?;
    failed(
        f.command(BIN).args(["test", "--plan"]).output()?,
        "Invalid Cargo workspace metadata",
    );
    assert!(!f.base.join("checks.log").exists());
    Ok(())
}

#[test]
fn renamed_optional_target_and_dev_edges_are_included() -> Result<()> {
    let f = Fixture::new()?;
    f.seed()?;
    let file = f.base.join("metadata.json");
    let mut metadata: FixtureMetadata = serde_json::from_slice(&fs::read(&file)?)?;
    metadata.packages[3].dependencies = vec![FixtureDependency {
        path: f.repo.join("crates/a"),
        rename: "alias".into(),
        kind: "dev".into(),
        optional: true,
        target: Some("cfg(windows)".into()),
    }];
    // A cycle must terminate without duplicate selections.
    metadata.packages[1].dependencies = vec![FixtureDependency {
        path: f.repo.join("crates/b"),
        ..Default::default()
    }];
    fs::write(file, serde_json::to_vec(&metadata)?)?;
    f.write("crates/a/src/lib.rs", "changed")?;
    let text = String::from_utf8(f.run(&["test", "--plan"])?.stdout)?;
    assert!(text.contains("Affected packages: a, b, burncloud, c"));
    Ok(())
}

#[test]
fn nested_ownership_uses_components_and_longest_prefix() -> Result<()> {
    let f = Fixture::new()?;
    f.write("crates/a/nested/Cargo.toml", "fixture")?;
    f.write("crates/a/nested/src/lib.rs", "fixture")?;
    f.git(&["add", "."])?;
    f.seed()?;
    let file = f.base.join("metadata.json");
    let mut metadata: FixtureMetadata = serde_json::from_slice(&fs::read(&file)?)?;
    metadata.workspace_members.push("nested".into());
    metadata.packages.push(FixturePackage {
        id: "nested".into(),
        name: "nested".into(),
        manifest_path: f.repo.join("crates/a/nested/Cargo.toml"),
        dependencies: vec![],
    });
    fs::write(file, serde_json::to_vec(&metadata)?)?;
    f.write("crates/a/nested/src/lib.rs", "changed")?;
    let text = String::from_utf8(f.run(&["test", "--plan"])?.stdout)?;
    assert!(text.contains("Affected packages: nested\n"));
    f.write("crates/a-similar/file.rs", "unknown")?;
    let text = String::from_utf8(f.run(&["test", "--plan"])?.stdout)?;
    assert!(text.contains("unclassified path: crates/a-similar/file.rs"));
    assert!(text.contains("cargo test --workspace"));
    Ok(())
}

#[test]
fn untracked_code_and_root_cli_are_selected() -> Result<()> {
    let f = Fixture::new()?;
    f.seed()?;
    f.write("crates/interfaces/cli/src/new.rs", "new")?;
    f.write("crates/c/src/new file.rs", "new")?;
    let text = String::from_utf8(f.run(&["test", "--plan"])?.stdout)?;
    assert!(text.contains("Affected packages: burncloud, c\n"));
    assert!(!text.contains("--workspace"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_is_never_overwritten() -> Result<()> {
    let f = Fixture::new()?;
    let target = f.base.join("shared-hook");
    fs::write(&target, "original")?;
    std::os::unix::fs::symlink(&target, f.hook())?;
    failed(f.command(BIN).arg("init").output()?, "not a regular file");
    assert!(fs::symlink_metadata(f.hook())?.file_type().is_symlink());
    assert_eq!(fs::read_to_string(target)?, "original");
    Ok(())
}
