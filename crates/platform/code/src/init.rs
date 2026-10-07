//! Local repository setup; deliberately independent of application services.
use std::fs;
use std::io::{self, Error, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

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
// Older managed wrapper: upgrade it without chaining the deleted script.
const LEGACY_HOOK: &str = r#"#!/bin/sh
# BurnCloud managed pre-commit hook. Reinstall with: cargo run -- code init
set -eu

root=$(git rev-parse --show-toplevel)
cd "$root"
sh "$root/.github/scripts/pre-commit-checks.sh"

# Keep the previous hook's interpreter, arguments and exit status intact.
hooks=$(git rev-parse --git-path hooks)
if [ -x "$hooks/pre-commit.burncloud-original" ]; then
    exec "$hooks/pre-commit.burncloud-original" "$@"
fi
"#;

fn git(directory: &Path, args: &[&str]) -> io::Result<String> {
    let output = Command::new("git")
        .current_dir(directory)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(Error::other(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim_end_matches(['\r', '\n']).to_owned())
        .map_err(Error::other)
}

pub(crate) fn init() -> io::Result<()> {
    let hooks = install(&std::env::current_dir()?)?;
    println!(
        "Pre-commit checks and commit-message receipts installed in {}",
        hooks.display()
    );
    println!("Required tools ready: rustfmt, Clippy and cargo-deny.");
    println!("Commits run cargo fmt, affected code test, cargo clippy and cargo deny.");
    Ok(())
}

pub(crate) fn ensure_environment(root: &Path) -> io::Result<()> {
    ensure_tool(root, "fmt", "rustup", &["component", "add", "rustfmt"])?;
    ensure_tool(root, "clippy", "rustup", &["component", "add", "clippy"])?;
    ensure_tool(
        root,
        "deny",
        "cargo",
        &["install", "--locked", "cargo-deny"],
    )?;
    Ok(())
}

fn install(directory: &Path) -> io::Result<PathBuf> {
    let root = PathBuf::from(git(directory, &["rev-parse", "--show-toplevel"])?);
    if !root.join(".github/hooks/pre-commit").is_file()
        || !root.join(".github/hooks/commit-msg").is_file()
        || !root.join("deny.toml").is_file()
        || !root.join("Cargo.toml").is_file()
    {
        return Err(Error::other(
            "Run code init inside a BurnCloud source checkout.",
        ));
    }

    // A custom path can be shared by unrelated repositories. Never overwrite it.
    let configured = Command::new("git")
        .current_dir(&root)
        .args(["config", "--get", "core.hooksPath"])
        .output()?;
    match configured.status.code() {
        Some(1) => {}
        Some(0) => {
            return Err(Error::other(
                "core.hooksPath is already configured. Integrate cargo fmt, cargo run -- code test --staged, cargo clippy and cargo deny into your existing hook manager, or remove that setting before code init.",
            ));
        }
        _ => return Err(Error::other("Unable to inspect Git core.hooksPath.")),
    }

    // Git resolves the common hooks directory correctly for linked worktrees.
    let hooks = root.join(git(&root, &["rev-parse", "--git-path", "hooks"])?);
    fs::create_dir_all(&hooks)?;
    // Check both hooks before changing either one.
    let pre_commit_legacy: &[&str] = &[PREVIOUS_HOOK, LEGACY_HOOK];
    for (name, content, legacy) in [
        ("pre-commit", HOOK, pre_commit_legacy),
        ("commit-msg", MESSAGE_HOOK, &[]),
    ] {
        inspect(&hooks, name, content, legacy)?;
    }
    // Install tools before activating either hook. A failed installation leaves
    // the existing Git hooks unchanged, and a later code init can retry.
    ensure_environment(&root)?;
    install_hook(&hooks, "pre-commit", HOOK, &[PREVIOUS_HOOK, LEGACY_HOOK])?;
    install_hook(&hooks, "commit-msg", MESSAGE_HOOK, &[])?;
    Ok(hooks)
}

fn ensure_tool(root: &Path, tool: &str, installer: &str, args: &[&str]) -> io::Result<()> {
    let available = || -> io::Result<bool> {
        Ok(Command::new("cargo")
            .current_dir(root)
            .args([tool, "--version"])
            .output()?
            .status
            .success())
    };
    if available()? {
        println!("Already available: cargo {tool}");
        return Ok(());
    }
    println!("Installing {tool}: {installer} {}", args.join(" "));
    let status = Command::new(installer)
        .current_dir(root)
        .args(args)
        .status()?;
    if !status.success() {
        return Err(Error::other(format!(
            "Could not install {tool} ({status}). Retry: {installer} {}",
            args.join(" ")
        )));
    }
    if !available()? {
        return Err(Error::other(format!(
            "{installer} completed, but cargo {tool} --version still fails. Check PATH and retry code init."
        )));
    }
    Ok(())
}

fn inspect(hooks: &Path, name: &str, content: &str, legacy: &[&str]) -> io::Result<()> {
    let hook = hooks.join(name);
    let backup = hooks.join(format!("{name}.burncloud-original"));
    let existing = match fs::symlink_metadata(&hook) {
        Ok(metadata) if metadata.is_file() => Some(fs::read(&hook)?),
        Ok(_) => {
            return Err(Error::other(format!(
                "Existing {name} is not a regular file; leaving it unchanged."
            )))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let upgrading = legacy
        .iter()
        .any(|text| existing.as_deref() == Some(text.as_bytes()));
    if existing.as_deref() != Some(content.as_bytes()) && !upgrading {
        match fs::symlink_metadata(&backup) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
            Ok(_) => {
                let message = format!(
                    "A saved {name}.burncloud-original already exists; refusing to overwrite hooks."
                );
                return Err(Error::other(message));
            }
        }
    }
    Ok(())
}

fn install_hook(hooks: &Path, name: &str, content: &str, legacy: &[&str]) -> io::Result<()> {
    let hook = hooks.join(name);
    let backup = hooks.join(format!("{name}.burncloud-original"));
    // Lock before reading existing state, including across linked worktrees.
    let pending = hooks.join(format!("{name}.burncloud-pending"));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)?;
    let result = (|| {
        inspect(hooks, name, content, legacy)?;
        let existing = match fs::read(&hook) {
            Ok(content) => Some(content),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        if existing.as_deref() == Some(content.as_bytes()) {
            make_executable(&hook)?;
            return Ok(());
        }
        let upgrading = legacy
            .iter()
            .any(|text| existing.as_deref() == Some(text.as_bytes()));
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        drop(file);
        make_executable(&pending)?;
        if existing.is_some() && !upgrading {
            fs::rename(&hook, &backup)?;
        }
        if let Err(error) = fs::rename(&pending, &hook) {
            if existing.is_some() && !upgrading {
                fs::rename(&backup, &hook)?;
            }
            return Err(error);
        }
        Ok(())
    })();
    if pending.exists() {
        fs::remove_file(pending)?;
    }
    result
}

fn make_executable(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    let _ = path; // Git for Windows runs hooks through its bundled shell.
    Ok(())
}
