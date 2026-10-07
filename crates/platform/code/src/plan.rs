use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

const KNOWN_TEST_BASELINE: &str =
    include_str!("../../../../.github/test-plan/known-test-baseline.txt");

#[derive(Deserialize)]
pub(crate) struct Metadata {
    packages: Vec<Package>,
    workspace_members: BTreeSet<String>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    // Includes normal, build, dev, optional and target-specific dependencies.
    path: Option<PathBuf>,
}

pub(crate) struct Plan {
    pub(crate) full_reason: Option<String>,
    pub(crate) direct: BTreeSet<String>,
    pub(crate) affected: BTreeSet<String>,
}

pub(crate) fn documentation(path: &str) -> bool {
    matches!(
        path,
        "README.md" | "LICENSE" | "LICENSE.md" | ".github/README.md"
    )
}

fn global(path: &str) -> bool {
    matches!(
        Path::new(path).file_name().and_then(|s| s.to_str()),
        Some("Cargo.toml" | "Cargo.lock")
    ) || path.starts_with(".cargo/")
        || path.starts_with(".github/")
        || matches!(
            path,
            "clippy.toml" | "deny.toml" | "rust-toolchain" | "rust-toolchain.toml"
        )
}

fn known_test_skips() -> Result<Vec<&'static str>> {
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();

    for (index, raw) in KNOWN_TEST_BASELINE.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('|').map(str::trim).collect();
        anyhow::ensure!(
            fields.len() == 3,
            "Invalid known-test baseline line {}: expected test_name|issue_number|reason",
            index + 1
        );
        let name = fields[0];
        let issue = fields[1];
        let reason = fields[2];
        anyhow::ensure!(!name.is_empty(), "Invalid known-test baseline line {}: empty test name", index + 1);
        anyhow::ensure!(
            issue.parse::<u64>().is_ok(),
            "Invalid known-test baseline line {}: issue number must be numeric",
            index + 1
        );
        anyhow::ensure!(!reason.is_empty(), "Invalid known-test baseline line {}: empty reason", index + 1);
        anyhow::ensure!(seen.insert(name), "Duplicate known-test baseline entry: {name}");
        names.push(name);
    }

    Ok(names)
}

pub(crate) fn select(
    root: &Path,
    files: &BTreeSet<String>,
    metadata: Metadata,
    all: bool,
) -> Result<Plan> {
    let root = root.canonicalize()?;
    let mut directories = BTreeMap::new();
    let mut packages = Vec::new();
    for package in metadata.packages {
        if !metadata.workspace_members.contains(&package.id) {
            continue;
        }
        let directory = package
            .manifest_path
            .parent()
            .context("Manifest has no parent")?
            .canonicalize()?;
        directories.insert(directory, package.name.clone());
        packages.push(package);
    }
    anyhow::ensure!(
        !packages.is_empty(),
        "Cargo metadata contains no workspace packages"
    );
    let mut plan = Plan {
        full_reason: all.then(|| "--all requested".to_owned()),
        direct: BTreeSet::new(),
        affected: BTreeSet::new(),
    };
    for file in files {
        if documentation(file) {
            continue;
        }
        if global(file) {
            plan.full_reason
                .get_or_insert_with(|| format!("shared configuration changed: {file}"));
            continue;
        }
        // Component-aware, longest-prefix ownership handles nested workspace packages.
        let path = root.join(file);
        let owner = directories
            .iter()
            .filter(|(dir, _)| **dir != root && path.starts_with(dir))
            .max_by_key(|(dir, _)| dir.components().count())
            .map(|(_, name)| name);
        let owner = owner.or_else(|| {
            (file.starts_with("crates/interfaces/cli/") || file.starts_with("src/"))
                .then(|| directories.get(&root))
                .flatten()
        });
        if let Some(name) = owner {
            plan.direct.insert(name.clone());
        } else {
            plan.full_reason
                .get_or_insert_with(|| format!("unclassified path: {file}"));
        }
    }
    if plan.full_reason.is_some() {
        plan.affected
            .extend(packages.iter().map(|p| p.name.clone()));
        return Ok(plan);
    }
    // Dependency paths identify renamed packages without confusing registry crates
    // that happen to share their names. Do not filter by current host or features.
    let mut consumers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for package in &packages {
        for dependency in &package.dependencies {
            if let Some(path) = &dependency.path {
                let directory = path
                    .canonicalize()
                    .with_context(|| format!("Cannot resolve dependency {}", path.display()))?;
                if let Some(name) = directories.get(&directory) {
                    consumers
                        .entry(name.clone())
                        .or_default()
                        .insert(package.name.clone());
                }
            }
        }
    }
    plan.affected = plan.direct.clone();
    let mut pending: Vec<_> = plan.direct.iter().cloned().collect();
    while let Some(name) = pending.pop() {
        if let Some(dependents) = consumers.get(&name) {
            for dependent in dependents {
                if plan.affected.insert(dependent.clone()) {
                    pending.push(dependent.clone());
                }
            }
        }
    }
    Ok(plan)
}

impl Plan {
    pub(crate) fn commands(&self) -> Result<Vec<Vec<String>>> {
        if self.affected.is_empty() {
            return Ok(Vec::new());
        }
        let mut scope = Vec::new();
        if self.full_reason.is_some() {
            scope.push("--workspace".to_owned());
        } else {
            for name in &self.affected {
                scope.extend(["-p".to_owned(), name.clone()]);
            }
        }
        let mut test = vec!["test".to_owned()];
        test.extend(scope.clone());
        test.push("--no-default-features".to_owned());
        let known_skips = known_test_skips()?;
        if !known_skips.is_empty() {
            test.push("--".to_owned());
            for name in known_skips {
                test.extend(["--skip".to_owned(), name.to_owned()]);
            }
        }
        let mut clippy = vec!["clippy".to_owned()];
        clippy.extend(scope);
        clippy.extend([
            "--all-targets".to_owned(),
            "--no-default-features".to_owned(),
            "--".to_owned(),
            "-D".to_owned(),
            "warnings".to_owned(),
        ]);
        Ok(vec![
            vec!["fmt".into(), "--all".into(), "--".into(), "--check".into()],
            test,
            clippy,
            vec!["deny".into(), "check".into()],
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::{known_test_skips, Plan};
    use std::collections::BTreeSet;

    #[test]
    fn known_test_baseline_is_the_reviewed_seven() -> anyhow::Result<()> {
        assert_eq!(
            known_test_skips()?,
            vec![
                "test_claude_adaptor",
                "test_deepseek_proxy",
                "test_qwen_proxy",
                "test_round_robin_balancer",
                "test_failover",
                "test_vertex_full_flow",
                "test_login_user_success",
            ]
        );
        Ok(())
    }

    #[test]
    fn test_command_applies_known_baseline_as_libtest_arguments() -> anyhow::Result<()> {
        let plan = Plan {
            full_reason: Some("test".to_owned()),
            direct: BTreeSet::new(),
            affected: BTreeSet::from(["burncloud-code".to_owned()]),
        };
        let commands = plan.commands()?;
        let test = &commands[1];
        let separator = test
            .iter()
            .position(|arg| arg == "--")
            .expect("known test baseline must add libtest arguments");
        assert_eq!(
            &test[..separator],
            ["test", "--workspace", "--no-default-features"]
        );
        let skip_names: Vec<_> = test[separator + 1..]
            .chunks_exact(2)
            .map(|chunk| {
                assert_eq!(chunk[0], "--skip");
                chunk[1].as_str()
            })
            .collect();
        assert_eq!(skip_names, known_test_skips()?);
        Ok(())
    }
}
