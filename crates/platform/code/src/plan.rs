use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::manifest::{self, Impact, RootManifestChange};

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
    name: String,
    rename: Option<String>,
    path: Option<PathBuf>,
}

impl Dependency {
    fn manifest_name(&self) -> &str {
        self.rename.as_deref().unwrap_or(&self.name)
    }
}

pub(crate) struct Plan {
    pub(crate) full_reason: Option<String>,
    pub(crate) direct: BTreeSet<String>,
    pub(crate) affected: BTreeSet<String>,
}

pub(crate) fn no_test_impact(path: &str) -> bool {
    matches!(
        path,
        "README.md" | "LICENSE" | "LICENSE.md" | ".github/README.md"
    ) || path.starts_with(".github/")
        || matches!(path, "clippy.toml" | "deny.toml")
}

fn global(path: &str) -> bool {
    matches!(
        path,
        "Cargo.lock" | "rust-toolchain" | "rust-toolchain.toml"
    ) || path.starts_with(".cargo/")
}

pub(crate) fn select(
    root: &Path,
    files: &BTreeSet<String>,
    metadata: Metadata,
    all: bool,
    root_manifest: Option<&RootManifestChange>,
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
        if no_test_impact(file) {
            continue;
        }
        if file == "Cargo.toml" {
            let Some(change) = root_manifest else {
                plan.full_reason.get_or_insert_with(|| {
                    "root Cargo.toml changed but its previous version is unavailable".to_owned()
                });
                continue;
            };
            match manifest::analyze(change) {
                Impact::Full(reason) => {
                    plan.full_reason.get_or_insert_with(|| {
                        format!("root Cargo.toml requires full workspace: {reason}")
                    });
                }
                Impact::Scoped {
                    added_members,
                    changed_workspace_dependencies,
                    root_package_changed,
                } => {
                    for member in added_members {
                        let member_path = root.join(&member);
                        let owner = member_path
                            .canonicalize()
                            .ok()
                            .and_then(|directory| directories.get(&directory));
                        if let Some(name) = owner {
                            plan.direct.insert(name.clone());
                        } else {
                            plan.full_reason.get_or_insert_with(|| {
                                format!(
                                    "root Cargo.toml added workspace member that cannot be mapped safely: {member}"
                                )
                            });
                        }
                    }
                    if root_package_changed {
                        if let Some(name) = directories.get(&root) {
                            plan.direct.insert(name.clone());
                        } else {
                            plan.full_reason.get_or_insert_with(|| {
                                "root package changed but cargo metadata has no root package"
                                    .to_owned()
                            });
                        }
                    }
                    for dependency in changed_workspace_dependencies {
                        for package in &packages {
                            if package
                                .dependencies
                                .iter()
                                .any(|candidate| candidate.manifest_name() == dependency)
                            {
                                plan.direct.insert(package.name.clone());
                            }
                        }
                    }
                }
            }
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
    pub(crate) fn clippy_command(&self) -> Vec<String> {
        let mut command = vec!["clippy".to_owned()];
        if self.full_reason.is_some() {
            command.push("--workspace".to_owned());
        } else {
            for name in &self.affected {
                command.extend(["-p".to_owned(), name.clone()]);
            }
        }
        command.extend([
            "--all-targets".to_owned(),
            "--no-default-features".to_owned(),
            "--".to_owned(),
            "-D".to_owned(),
            "warnings".to_owned(),
        ]);
        command
    }

    pub(crate) fn commands(&self) -> Result<Vec<Vec<String>>> {
        if self.affected.is_empty() {
            return Ok(Vec::new());
        }

        let mut commands = Vec::new();
        if self.affected.contains("burncloud-tests") {
            commands.push(vec![
                "build".to_owned(),
                "-p".to_owned(),
                "burncloud".to_owned(),
                "--bin".to_owned(),
                "burncloud".to_owned(),
                "--no-default-features".to_owned(),
            ]);
        }

        let mut test = vec!["test".to_owned()];
        if self.full_reason.is_some() {
            test.push("--workspace".to_owned());
        } else {
            for name in &self.affected {
                test.extend(["-p".to_owned(), name.clone()]);
            }
        }
        test.push("--no-default-features".to_owned());
        commands.push(test);
        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::{global, Plan};
    use std::collections::BTreeSet;

    #[test]
    fn only_truly_global_cargo_inputs_force_workspace() {
        assert!(!global("Cargo.toml"));
        assert!(global("Cargo.lock"));
        assert!(global("rust-toolchain"));
        assert!(global("rust-toolchain.toml"));
        assert!(global(".cargo/config.toml"));
    }

    #[test]
    fn affected_clippy_scopes_packages_and_stays_strict() {
        let plan = Plan {
            full_reason: None,
            direct: BTreeSet::from(["a".to_owned()]),
            affected: BTreeSet::from(["a".to_owned(), "b".to_owned()]),
        };
        assert_eq!(
            plan.clippy_command(),
            [
                "clippy",
                "-p",
                "a",
                "-p",
                "b",
                "--all-targets",
                "--no-default-features",
                "--",
                "-D",
                "warnings"
            ]
        );
    }

    #[test]
    fn full_clippy_uses_workspace_and_stays_strict() {
        let plan = Plan {
            full_reason: Some("shared configuration changed".to_owned()),
            direct: BTreeSet::new(),
            affected: BTreeSet::from(["a".to_owned(), "b".to_owned()]),
        };
        assert_eq!(
            plan.clippy_command(),
            [
                "clippy",
                "--workspace",
                "--all-targets",
                "--no-default-features",
                "--",
                "-D",
                "warnings"
            ]
        );
    }

    #[test]
    fn black_box_tests_build_the_server_binary_before_running() -> anyhow::Result<()> {
        let plan = Plan {
            full_reason: None,
            direct: BTreeSet::from(["burncloud-tests".to_owned()]),
            affected: BTreeSet::from(["burncloud-tests".to_owned()]),
        };
        let commands = plan.commands()?;
        anyhow::ensure!(
            commands[0]
                == [
                    "build",
                    "-p",
                    "burncloud",
                    "--bin",
                    "burncloud",
                    "--no-default-features"
                ],
            "black-box server build prerequisite changed unexpectedly"
        );
        anyhow::ensure!(
            commands
                .get(1)
                .and_then(|command| command.first())
                .map(String::as_str)
                == Some("test"),
            "black-box tests must run after the server build prerequisite"
        );
        Ok(())
    }

    #[test]
    fn test_command_never_injects_hidden_skips() -> anyhow::Result<()> {
        let plan = Plan {
            full_reason: Some("test".to_owned()),
            direct: BTreeSet::new(),
            affected: BTreeSet::from(["burncloud-code".to_owned()]),
        };
        let commands = plan.commands()?;
        anyhow::ensure!(
            commands[0] == ["test", "--workspace", "--no-default-features"],
            "code test must run selected tests directly without hidden libtest skip arguments"
        );
        Ok(())
    }
}

