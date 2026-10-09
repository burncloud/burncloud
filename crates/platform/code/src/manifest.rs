use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub(crate) struct RootManifestChange {
    pub(crate) before: String,
    pub(crate) after: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Impact {
    Scoped {
        added_members: BTreeSet<String>,
        changed_workspace_dependencies: BTreeSet<String>,
        root_package_changed: bool,
    },
    Full(String),
}

fn sections(text: &str) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut result = BTreeMap::<String, Vec<String>>::new();
    let mut current = String::new();

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = line
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_owned();
            if current.is_empty() {
                return Err("empty TOML section header".to_owned());
            }
            result.entry(current.clone()).or_default();
            continue;
        }
        if current.is_empty() {
            return Err(format!("content outside a TOML section: {line}"));
        }
        result.entry(current.clone()).or_default().push(line.to_owned());
    }

    Ok(result)
}

fn quoted_values(text: &str) -> Result<BTreeSet<String>, String> {
    let mut values = BTreeSet::new();
    let mut chars = text.char_indices().peekable();
    while let Some((_, ch)) = chars.next() {
        if ch != '"' {
            continue;
        }
        let mut value = String::new();
        let mut escaped = false;
        let mut closed = false;
        for (_, next) in chars.by_ref() {
            if escaped {
                value.push(next);
                escaped = false;
            } else if next == '\\' {
                escaped = true;
            } else if next == '"' {
                closed = true;
                break;
            } else {
                value.push(next);
            }
        }
        if !closed {
            return Err("unterminated quoted value".to_owned());
        }
        values.insert(value);
    }
    Ok(values)
}

fn workspace_members(lines: &[String]) -> Result<(BTreeSet<String>, Vec<String>), String> {
    let mut members = BTreeSet::new();
    let mut remainder = Vec::new();
    let mut index = 0;
    let mut found = false;

    while index < lines.len() {
        let line = &lines[index];
        let Some((key, value)) = line.split_once('=') else {
            remainder.push(line.clone());
            index += 1;
            continue;
        };
        if key.trim() != "members" {
            remainder.push(line.clone());
            index += 1;
            continue;
        }
        if found {
            return Err("workspace.members is declared more than once".to_owned());
        }
        found = true;

        let mut block = value.to_owned();
        let mut balance = value.matches('[').count() as isize - value.matches(']').count() as isize;
        if !value.contains('[') {
            return Err("workspace.members is not an array".to_owned());
        }
        while balance > 0 {
            index += 1;
            let Some(next) = lines.get(index) else {
                return Err("unterminated workspace.members array".to_owned());
            };
            block.push('\n');
            block.push_str(next);
            balance += next.matches('[').count() as isize - next.matches(']').count() as isize;
        }
        if balance != 0 {
            return Err("invalid workspace.members array".to_owned());
        }
        members = quoted_values(&block)?;
        if members
            .iter()
            .any(|member| member.contains('*') || member.contains('?') || member.contains('['))
        {
            return Err("workspace.members glob requires full-workspace fallback".to_owned());
        }
        index += 1;
    }

    Ok((members, remainder))
}

fn dependency_entries(lines: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut entries = BTreeMap::new();
    for line in lines {
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("unsupported multi-line workspace dependency: {line}"));
        };
        let key = key.trim().trim_matches('"').to_owned();
        if key.is_empty() {
            return Err("empty workspace dependency key".to_owned());
        }
        if entries.insert(key.clone(), value.trim().to_owned()).is_some() {
            return Err(format!("duplicate workspace dependency: {key}"));
        }
    }
    Ok(entries)
}

pub(crate) fn analyze(change: &RootManifestChange) -> Impact {
    let before = match sections(&change.before) {
        Ok(value) => value,
        Err(error) => return Impact::Full(format!("cannot classify previous Cargo.toml: {error}")),
    };
    let after = match sections(&change.after) {
        Ok(value) => value,
        Err(error) => return Impact::Full(format!("cannot classify current Cargo.toml: {error}")),
    };

    let mut names = BTreeSet::new();
    names.extend(before.keys().cloned());
    names.extend(after.keys().cloned());

    let mut added_members = BTreeSet::new();
    let mut changed_workspace_dependencies = BTreeSet::new();
    let mut root_package_changed = false;

    for name in names {
        let before_lines = before.get(&name).cloned().unwrap_or_default();
        let after_lines = after.get(&name).cloned().unwrap_or_default();
        if before_lines == after_lines {
            continue;
        }

        if name == "workspace" {
            let (before_members, before_remainder) = match workspace_members(&before_lines) {
                Ok(value) => value,
                Err(error) => return Impact::Full(error),
            };
            let (after_members, after_remainder) = match workspace_members(&after_lines) {
                Ok(value) => value,
                Err(error) => return Impact::Full(error),
            };
            if before_remainder != after_remainder {
                return Impact::Full(
                    "workspace configuration other than members changed".to_owned(),
                );
            }
            added_members.extend(after_members.difference(&before_members).cloned());
            continue;
        }

        if name == "workspace.dependencies" {
            let before_dependencies = match dependency_entries(&before_lines) {
                Ok(value) => value,
                Err(error) => return Impact::Full(error),
            };
            let after_dependencies = match dependency_entries(&after_lines) {
                Ok(value) => value,
                Err(error) => return Impact::Full(error),
            };
            let mut keys = BTreeSet::new();
            keys.extend(before_dependencies.keys().cloned());
            keys.extend(after_dependencies.keys().cloned());
            for key in keys {
                if before_dependencies.get(&key) != after_dependencies.get(&key) {
                    changed_workspace_dependencies.insert(key);
                }
            }
            continue;
        }

        if name.starts_with("workspace.") {
            return Impact::Full(format!("shared workspace section changed: [{name}]"));
        }
        if name == "profile"
            || name.starts_with("profile.")
            || name == "patch"
            || name.starts_with("patch.")
            || name == "replace"
            || name.starts_with("replace.")
            || name == "__root__"
        {
            return Impact::Full(format!("global Cargo section changed: [{name}]"));
        }

        // All remaining sections belong to the root package itself: [package],
        // [dependencies], [features], [lints], [target.*], [[bin]], etc.
        root_package_changed = true;
    }

    Impact::Scoped {
        added_members,
        changed_workspace_dependencies,
        root_package_changed,
    }
}

#[cfg(test)]
mod tests {
    use super::{analyze, Impact, RootManifestChange};
    use std::collections::BTreeSet;

    fn change(before: &str, after: &str) -> RootManifestChange {
        RootManifestChange {
            before: before.to_owned(),
            after: after.to_owned(),
        }
    }

    #[test]
    fn workspace_member_rename_is_scoped_to_the_added_member() {
        let impact = analyze(&change(
            r#"[workspace]
members = [
    "crates/identity/database-user",
    "crates/identity/service-user",
]
"#,
            r#"[workspace]
members = [
    "crates/identity/user",
]
"#,
        ));
        assert_eq!(
            impact,
            Impact::Scoped {
                added_members: BTreeSet::from(["crates/identity/user".to_owned()]),
                changed_workspace_dependencies: BTreeSet::new(),
                root_package_changed: false,
            }
        );
    }

    #[test]
    fn workspace_dependency_change_reports_only_changed_keys() {
        let impact = analyze(&change(
            r#"[workspace.dependencies]
serde = "1"
foo = { path = "crates/foo" }
"#,
            r#"[workspace.dependencies]
serde = "1"
foo = { path = "crates/foo-new" }
bar = "2"
"#,
        ));
        assert_eq!(
            impact,
            Impact::Scoped {
                added_members: BTreeSet::new(),
                changed_workspace_dependencies: BTreeSet::from([
                    "bar".to_owned(),
                    "foo".to_owned(),
                ]),
                root_package_changed: false,
            }
        );
    }

    #[test]
    fn root_package_dependency_change_is_scoped_to_root_package() {
        let impact = analyze(&change(
            r#"[package]
name = "root"

[dependencies]
foo = "1"
"#,
            r#"[package]
name = "root"

[dependencies]
foo = "2"
"#,
        ));
        assert_eq!(
            impact,
            Impact::Scoped {
                added_members: BTreeSet::new(),
                changed_workspace_dependencies: BTreeSet::new(),
                root_package_changed: true,
            }
        );
    }

    #[test]
    fn workspace_lint_change_forces_full_workspace() {
        let impact = analyze(&change(
            r#"[workspace.lints.rust]
unused_must_use = "warn"
"#,
            r#"[workspace.lints.rust]
unused_must_use = "deny"
"#,
        ));
        assert!(matches!(impact, Impact::Full(_)));
    }

    #[test]
    fn profile_change_forces_full_workspace() {
        let impact = analyze(&change(
            r#"[profile.release]
lto = false
"#,
            r#"[profile.release]
lto = true
"#,
        ));
        assert!(matches!(impact, Impact::Full(_)));
    }
}
