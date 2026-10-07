use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{Context, Result};

use crate::plan::{self, Metadata};
use crate::report;

const GIB_BYTES: u64 = 1024 * 1024 * 1024;
const TARGET_LIMIT_BYTES: u64 = 100 * GIB_BYTES;

pub(crate) struct Options {
    pub(crate) all: bool,
    pub(crate) plan_only: bool,
    pub(crate) staged: bool,
    pub(crate) last: bool,
    pub(crate) base: Option<String>,
}

fn output(root: &Path, program: &str, args: &[&str]) -> Result<Output> {
    let result = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("Cannot start {program}; ensure it is on PATH"))?;
    anyhow::ensure!(
        result.status.success(),
        "{program} {} failed ({}): {}",
        args.join(" "),
        result.status,
        String::from_utf8_lossy(&result.stderr).trim()
    );
    Ok(result)
}

pub(crate) fn git(root: &Path, args: &[&str]) -> Result<String> {
    String::from_utf8(output(root, "git", args)?.stdout)
        .context("Non-UTF-8 Git output; refusing to omit changed paths")
}

fn paths(result: &str, files: &mut BTreeSet<String>) {
    files.extend(
        result
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(str::to_owned),
    );
}

fn changes(root: &Path, options: &Options) -> Result<BTreeSet<String>> {
    let untracked = git(root, &["ls-files", "--others", "--exclude-standard", "-z"])?;
    if options.staged {
        let unstaged = git(
            root,
            &[
                "diff",
                "--name-only",
                "--no-renames",
                "--ignore-submodules=none",
                "-z",
                "--",
            ],
        )?;
        anyhow::ensure!(unstaged.is_empty() && untracked.is_empty(), "Commit blocked: stage or stash tracked changes and untracked files before code test --staged");
    }
    let mut files = BTreeSet::new();
    if options.staged {
        paths(
            &git(
                root,
                &[
                    "diff",
                    "--cached",
                    "--name-only",
                    "--no-renames",
                    "-z",
                    "--",
                ],
            )?,
            &mut files,
        );
    } else {
        let base = if let Some(reference) = &options.base {
            let commit = git(
                root,
                &[
                    "rev-parse",
                    "--verify",
                    "--end-of-options",
                    &format!("{reference}^{{commit}}"),
                ],
            )?;
            Some(
                git(root, &["merge-base", commit.trim(), "HEAD"])?
                    .trim()
                    .to_owned(),
            )
        } else {
            // symbolic-ref/HEAD may be unborn in a freshly initialized repository.
            let head = Command::new("git")
                .args(["rev-parse", "--verify", "HEAD"])
                .current_dir(root)
                .output()?;
            head.status.success().then(|| "HEAD".to_owned())
        };
        if let Some(base) = base {
            paths(
                &git(
                    root,
                    &["diff", "--name-only", "--no-renames", "-z", &base, "--"],
                )?,
                &mut files,
            );
        } else {
            paths(
                &git(
                    root,
                    &[
                        "diff",
                        "--cached",
                        "--name-only",
                        "--no-renames",
                        "-z",
                        "--",
                    ],
                )?,
                &mut files,
            );
            paths(
                &git(root, &["diff", "--name-only", "--no-renames", "-z", "--"])?,
                &mut files,
            );
        }
        paths(&untracked, &mut files);
    }
    Ok(files)
}

pub(crate) fn run(options: Options) -> Result<()> {
    let directory = std::env::current_dir()?;
    let root = PathBuf::from(
        git(&directory, &["rev-parse", "--show-toplevel"])?.trim_end_matches(['\r', '\n']),
    );
    let result = run_in_root(&root, options);
    let configured_target = std::env::var_os("CARGO_TARGET_DIR");
    let target = target_directory(&root, configured_target.as_deref());
    if let Err(error) = cleanup_target_if_oversized(&target, TARGET_LIMIT_BYTES) {
        eprintln!("Warning: target cleanup failed: {error:#}");
    }
    result
}

fn run_in_root(root: &Path, options: Options) -> Result<()> {
    if options.last {
        return report::show_latest(root);
    }
    let files = changes(root, &options)?;
    println!("Changed paths ({}):", files.len());
    for file in &files {
        println!("  {file}");
    }
    if !options.all && files.iter().all(|file| plan::no_test_impact(file)) {
        println!("No test-relevant changes selected; no tests executed. Use --all for the full workspace or --base REF for branch changes.");
        if !options.plan_only {
            let mut summary = report::new(
                root,
                if options.staged { "staged" } else { "working" },
                files.iter().cloned().collect(),
                Vec::new(),
            )?;
            report::finish(root, &mut summary, "skipped", None)?;
        }
        return Ok(());
    }
    let raw = output(
        root,
        "cargo",
        &["metadata", "--no-deps", "--format-version", "1"],
    )?;
    let metadata: Metadata =
        serde_json::from_slice(&raw.stdout).context("Invalid Cargo workspace metadata")?;
    let plan = plan::select(root, &files, metadata, options.all)?;
    if let Some(reason) = &plan.full_reason {
        println!("Selection: full workspace ({reason})");
    } else {
        println!(
            "Direct packages: {}",
            plan.direct.iter().cloned().collect::<Vec<_>>().join(", ")
        );
        println!("Selection: changed packages plus all transitive workspace consumers");
    }
    println!(
        "Affected packages: {}",
        plan.affected.iter().cloned().collect::<Vec<_>>().join(", ")
    );
    let commands = plan.commands()?;
    for args in &commands {
        println!("  cargo {}", args.join(" "));
    }
    if options.plan_only {
        println!("PLAN ONLY: no checks executed.");
        return Ok(());
    }
    let mode = if options.staged {
        "staged"
    } else if options.all {
        "all"
    } else if options.base.is_some() {
        "base"
    } else {
        "working"
    };
    let mut summary = report::new(
        root,
        mode,
        files.iter().cloned().collect(),
        plan.affected.iter().cloned().collect(),
    )?;
    let result = run_checks(root, &commands, &mut summary);
    report::finish(
        root,
        &mut summary,
        if result.is_ok() { "passed" } else { "failed" },
        result.as_ref().err().map(ToString::to_string),
    )?;
    result?;
    println!("All selected tests passed.");
    Ok(())
}

fn run_checks(root: &Path, commands: &[Vec<String>], summary: &mut report::Summary) -> Result<()> {
    for (index, args) in commands.iter().enumerate() {
        println!(
            "[{}/{}] cargo {}",
            index + 1,
            commands.len(),
            args.join(" ")
        );
        let step_name = if args.first().is_some_and(|arg| arg == "test") {
            "test"
        } else {
            "prepare"
        };
        let status = report::execute(root, summary, step_name, args)?;
        anyhow::ensure!(
            status.success(),
            "Check failed ({}): cargo {}",
            status,
            args.join(" ")
        );
    }
    Ok(())
}

fn target_directory(root: &Path, configured: Option<&std::ffi::OsStr>) -> PathBuf {
    let Some(configured) = configured else {
        return root.join("target");
    };
    let configured = PathBuf::from(configured);
    if configured.is_absolute() {
        configured
    } else {
        root.join(configured)
    }
}

fn cleanup_target_if_oversized(target: &Path, limit: u64) -> Result<()> {
    let metadata = match fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).context("Cannot inspect target directory"),
    };
    anyhow::ensure!(
        metadata.file_type().is_dir(),
        "Refusing to clean non-directory target path: {}",
        target.display()
    );
    let bytes = directory_size(&target)?;
    println!(
        "Target directory size: {:.2} GiB (cleanup threshold: 100 GiB)",
        bytes as f64 / GIB_BYTES as f64
    );
    if bytes > limit {
        println!(
            "Target directory exceeds 100 GiB; removing {}",
            target.display()
        );
        fs::remove_dir_all(&target).with_context(|| {
            format!(
                "Cannot remove oversized target directory {}",
                target.display()
            )
        })?;
        println!("Oversized target directory removed.");
    }
    Ok(())
}

fn directory_size(root: &Path) -> Result<u64> {
    let mut total = 0_u64;
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .with_context(|| format!("Cannot read {}", directory.display()))?
        {
            let entry = entry?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_dir() {
                pending.push(entry.path());
            } else {
                total = total.saturating_add(metadata.len());
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::{cleanup_target_if_oversized, target_directory};
    use anyhow::Result;
    use std::fs;

    #[test]
    fn default_target_directory_is_inside_the_repository() {
        let root = std::path::Path::new("workspace");
        assert_eq!(target_directory(root, None), root.join("target"));
    }

    #[test]
    fn configured_target_directory_can_live_outside_the_repository() -> Result<()> {
        let root = std::path::Path::new("workspace");
        let external_root = tempfile::tempdir()?;
        let external = external_root.path().join("target");
        assert_eq!(target_directory(root, Some(external.as_os_str())), external);
        Ok(())
    }

    #[test]
    fn relative_configured_target_directory_is_resolved_from_the_repository() {
        let root = std::path::Path::new("workspace");
        assert_eq!(
            target_directory(root, Some(std::ffi::OsStr::new("../cache/target"))),
            root.join("../cache/target")
        );
    }

    #[test]
    fn target_at_or_below_limit_is_kept() -> Result<()> {
        let root = tempfile::tempdir()?;
        let target = root.path().join("target");
        fs::create_dir_all(&target)?;
        fs::write(target.join("artifact"), b"1234")?;
        cleanup_target_if_oversized(&target, 4)?;
        assert!(target.exists());
        Ok(())
    }

    #[test]
    fn target_above_limit_is_removed() -> Result<()> {
        let root = tempfile::tempdir()?;
        let target = root.path().join("target/nested");
        fs::create_dir_all(&target)?;
        fs::write(target.join("artifact"), b"12345")?;
        cleanup_target_if_oversized(&root.path().join("target"), 4)?;
        assert!(!root.path().join("target").exists());
        Ok(())
    }
}
