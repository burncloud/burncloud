use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{Context, Result};

use crate::plan::{self, Metadata};
use crate::{policy, report};

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
    if options.last {
        return report::show_latest(&root);
    }
    let files = changes(&root, &options)?;
    println!("Changed paths ({}):", files.len());
    for file in &files {
        println!("  {file}");
    }
    if !options.all && files.iter().all(|file| plan::documentation(file)) {
        println!("No code changes selected; no checks executed. Use --all for the full workspace or --base REF for branch changes.");
        if !options.plan_only {
            let mut summary = report::new(
                &root,
                if options.staged { "staged" } else { "working" },
                files.iter().cloned().collect(),
                Vec::new(),
            )?;
            report::finish(&root, &mut summary, "skipped", None)?;
        }
        return Ok(());
    }
    let raw = output(
        &root,
        "cargo",
        &["metadata", "--no-deps", "--format-version", "1"],
    )?;
    let metadata: Metadata =
        serde_json::from_slice(&raw.stdout).context("Invalid Cargo workspace metadata")?;
    let plan = plan::select(&root, &files, metadata, options.all)?;
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
    let commands = plan.commands();
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
        &root,
        mode,
        files.iter().cloned().collect(),
        plan.affected.iter().cloned().collect(),
    )?;
    let result = policy::enforce(&root, &files).and_then(|()| run_checks(&root, &commands, &mut summary));
    report::finish(
        &root,
        &mut summary,
        if result.is_ok() { "passed" } else { "failed" },
        result.as_ref().err().map(ToString::to_string),
    )?;
    result?;
    println!("All selected checks passed.");
    Ok(())
}

fn run_checks(root: &Path, commands: &[Vec<String>], summary: &mut report::Summary) -> Result<()> {
    for (tool, install) in [
        ("fmt", "rustup component add rustfmt"),
        ("clippy", "rustup component add clippy"),
        ("deny", "cargo install cargo-deny --locked"),
    ] {
        output(root, "cargo", &[tool, "--version"])
            .with_context(|| format!("Missing or broken {tool}. Run: {install}"))?;
    }
    for (index, args) in commands.iter().enumerate() {
        println!(
            "[{}/{}] cargo {}",
            index + 1,
            commands.len(),
            args.join(" ")
        );
        let status = report::execute(
            root,
            summary,
            ["fmt", "test", "clippy", "deny"][index],
            args,
        )?;
        anyhow::ensure!(
            status.success(),
            "Check failed ({}): cargo {}",
            status,
            args.join(" ")
        );
    }
    Ok(())
}
