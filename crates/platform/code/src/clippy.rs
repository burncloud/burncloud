use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};

use crate::plan::{self, Metadata};
use crate::test;

pub(crate) fn run(options: test::Options) -> Result<()> {
    let directory = std::env::current_dir()?;
    let root = PathBuf::from(
        test::git(&directory, &["rev-parse", "--show-toplevel"])?.trim_end_matches(['\r', '\n']),
    );
    let files = test::changes(&root, &options)?;

    println!("Changed paths ({}):", files.len());
    for file in &files {
        println!("  {file}");
    }

    let clippy_config_changed = files.contains("clippy.toml");
    if !options.all && !clippy_config_changed && files.iter().all(|file| plan::no_test_impact(file))
    {
        println!(
            "No Clippy-relevant changes selected; no lint executed. Use --all for the full workspace or --base REF for branch changes."
        );
        return Ok(());
    }

    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(&root)
        .output()
        .context("Cannot start cargo metadata; ensure Cargo is on PATH")?;
    anyhow::ensure!(
        output.status.success(),
        "cargo metadata failed ({}): {}",
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let metadata: Metadata =
        serde_json::from_slice(&output.stdout).context("Invalid Cargo workspace metadata")?;

    let mut plan = plan::select(
        &root,
        &files,
        metadata,
        options.all || clippy_config_changed,
    )?;
    if clippy_config_changed && !options.all {
        plan.full_reason = Some("Clippy configuration changed: clippy.toml".to_owned());
    }

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

    let args = plan.clippy_command();
    println!("  cargo {}", args.join(" "));

    if options.plan_only {
        println!("PLAN ONLY: no checks executed.");
        return Ok(());
    }

    let status = Command::new("cargo")
        .args(&args)
        .current_dir(&root)
        .status()
        .context("Cannot start cargo clippy; ensure Cargo is on PATH")?;
    anyhow::ensure!(
        status.success(),
        "Clippy failed ({}): cargo {}",
        status,
        args.join(" ")
    );
    println!("All selected Clippy checks passed.");
    Ok(())
}
