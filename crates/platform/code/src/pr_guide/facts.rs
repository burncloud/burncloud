use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, Output};

use anyhow::{Context, Result};

use super::github::ChangedFile;
use super::types::{
    CheckFact, CompareRange, ContextFact, Coverage, Evidence, FactBundle, FileFact, LinkedIssue,
    PullRequest, RelationFact, SymbolFact,
};

const PATCH_FILE_LIMIT: usize = 24_000;
const PATCH_TOTAL_LIMIT: usize = 360_000;
const SOURCE_EXCERPT_LIMIT: usize = 16_000;
const SOURCE_TOTAL_LIMIT: usize = 220_000;
const BLOB_LIMIT: u64 = 512_000;
const CONTEXT_LIMIT: usize = 48;
const INPUT_BUDGET: usize = 600_000;

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "fact collection keeps one ordered, fail-closed budget and coverage accounting path"
)]
pub(crate) fn collect(
    repository: &str,
    pull_request: PullRequest,
    api_files: Vec<ChangedFile>,
    checks: Vec<CheckFact>,
    linked_issues: Vec<LinkedIssue>,
) -> Result<FactBundle> {
    let mut pull_request = pull_request;
    pull_request.title = redact(&safe_prefix(&pull_request.title, 2_000));
    pull_request.body = redact(&safe_prefix(&pull_request.body, 20_000));
    let linked_issues = linked_issues
        .into_iter()
        .map(|mut issue| {
            issue.title = redact(&safe_prefix(&issue.title, 1_000));
            issue
        })
        .collect();
    ensure_git_objects(&pull_request)?;
    let merge_base = git_text(&["merge-base", &pull_request.base_sha, &pull_request.head_sha])?;
    validate_sha(&merge_base)?;

    let changes = changed_paths(&merge_base, &pull_request.head_sha)?;
    let api_by_path: BTreeMap<&str, &ChangedFile> = api_files
        .iter()
        .map(|file| (file.filename.as_str(), file))
        .collect();
    let mut warnings = Vec::new();
    if changes.len() as u64 != pull_request.changed_files {
        warnings.push(format!(
            "本地 merge-base 差异包含 {} 个文件，GitHub PR 元数据报告 {} 个文件。",
            changes.len(),
            pull_request.changed_files
        ));
    }
    if api_files.len() as u64 != pull_request.changed_files {
        warnings.push(format!(
            "GitHub 分页文件接口返回 {} 个文件，PR 元数据报告 {} 个；接口数据可能被截断。",
            api_files.len(),
            pull_request.changed_files
        ));
    }
    for change in &changes {
        if let Some(api) = api_by_path.get(change.path.as_str()) {
            if api.status != change.status
                || api.previous_filename.as_deref() != change.previous_path.as_deref()
            {
                warnings.push(format!(
                    "{} 的 GitHub 文件状态（{}）与本地 Git 状态（{}）不一致。",
                    change.path, api.status, change.status
                ));
            }
        }
    }

    let mut evidence = Vec::new();
    let mut files = Vec::new();
    let mut patch_used = 0;
    let mut source_used = 0;
    let mut checked = vec![
        "PR 标题、描述、状态和关联 Issue".to_owned(),
        "目标分支提交、merge-base 和 head 提交".to_owned(),
        "完整 Git 文件状态列表（含新增、重命名和删除）".to_owned(),
        "当前 head SHA 对应的 GitHub check-runs".to_owned(),
    ];
    let mut omitted = Vec::new();

    for change in &changes {
        let api = api_by_path.get(change.path.as_str()).copied();
        let before_path = change.previous_path.as_deref().unwrap_or(&change.path);
        let before_size = blob_size(&merge_base, before_path)?;
        let after_size = blob_size(&pull_request.head_sha, &change.path)?;
        let oversized = before_size
            .into_iter()
            .chain(after_size)
            .any(|size| size > BLOB_LIMIT);
        let binary = numstat_is_binary(&merge_base, &pull_request.head_sha, &change.path)?;
        let raw_patch = if binary || oversized {
            String::new()
        } else {
            diff_for_path(
                &merge_base,
                &pull_request.head_sha,
                change.previous_path.as_deref(),
                &change.path,
            )?
        };
        let remaining = PATCH_TOTAL_LIMIT.saturating_sub(patch_used);
        let allowed = PATCH_FILE_LIMIT.min(remaining);
        let patch_truncated = oversized || raw_patch.len() > allowed;
        let patch = if binary || allowed == 0 {
            None
        } else {
            let value = safe_prefix(&raw_patch, allowed);
            patch_used += value.len();
            Some(redact(&value))
        };
        if patch_truncated {
            omitted.push(format!(
                "{} 的 diff 超出单文件或总输入预算，已截断。",
                change.path
            ));
        }
        if binary {
            omitted.push(format!("{} 是二进制差异，未读取内容。", change.path));
        }
        if oversized {
            omitted.push(format!(
                "{} 的修改前或修改后文件超过 {} 字节，未生成完整 patch。",
                change.path, BLOB_LIMIT
            ));
        }

        let before = if change.status == "added" || binary {
            None
        } else {
            read_blob_excerpt(&merge_base, before_path, &mut omitted)?
        };
        let before = fit_source_budget(before, &change.path, &mut source_used, &mut omitted);
        let after = if change.status == "removed" || binary {
            None
        } else {
            read_blob_excerpt(&pull_request.head_sha, &change.path, &mut omitted)?
        };
        let after = fit_source_budget(after, &change.path, &mut source_used, &mut omitted);

        let (side, path, sha, line, excerpt) = if change.status == "removed" {
            (
                "before",
                before_path,
                merge_base.as_str(),
                first_hunk_line(&raw_patch, true),
                before.as_deref().unwrap_or_default(),
            )
        } else {
            (
                "after",
                change.path.as_str(),
                pull_request.head_sha.as_str(),
                first_hunk_line(&raw_patch, false),
                after.as_deref().unwrap_or_default(),
            )
        };
        let evidence_id = next_evidence_id(&evidence);
        evidence.push(Evidence {
            id: evidence_id,
            side: side.to_owned(),
            path: path.to_owned(),
            sha: sha.to_owned(),
            line_start: line,
            line_end: line,
            url: source_url(repository, sha, path, line, line),
            excerpt: excerpt.lines().take(3).collect::<Vec<_>>().join("\n"),
            reason: format!("{} 文件的首个变更位置", change.status),
        });

        files.push(FileFact {
            status: change.status.clone(),
            path: change.path.clone(),
            previous_path: change.previous_path.clone(),
            additions: api.map(|file| file.additions),
            deletions: api.map(|file| file.deletions),
            binary,
            patch,
            patch_truncated,
            before_excerpt: before,
            after_excerpt: after,
            module: module_for_path(&change.path),
        });
    }

    let symbols = collect_symbols(
        repository,
        &merge_base,
        &pull_request.head_sha,
        &files,
        &mut evidence,
    )?;
    let mut context = collect_structural_context(
        repository,
        &pull_request.head_sha,
        &files,
        &mut evidence,
        &mut omitted,
    )?;
    collect_symbol_references(
        repository,
        &pull_request.head_sha,
        &files,
        &symbols,
        &mut evidence,
        &mut context,
    )?;
    let relations =
        collect_import_relations(repository, &pull_request.head_sha, &files, &mut evidence);

    if files.iter().any(|file| {
        file.after_excerpt
            .as_deref()
            .map(|source| {
                source.contains("cfg(")
                    || source.contains("macro_rules!")
                    || source.contains("dyn ")
                    || source.contains("impl ")
            })
            .unwrap_or(false)
    }) {
        warnings.push(
            "变更涉及条件编译、宏、trait 实现或动态分派；纯文本关系分析不能确认全部运行时调用。"
                .to_owned(),
        );
    }

    checked.push("修改前后源码、Rust 声明、Cargo 清单和模块入口".to_owned());
    checked.push("关键符号在 head 树中的文本引用（不等同于动态调用）".to_owned());
    if files.iter().any(|file| file.patch_truncated || file.binary) {
        warnings.push("部分内容未进入模型上下文；报告不得据此宣称完整审查。".to_owned());
    }

    let mut bundle = FactBundle {
        schema_version: "burncloud.pr-guide.facts.v1".to_owned(),
        repository: repository.to_owned(),
        pull_request,
        compare: CompareRange {
            target_branch_sha: String::new(),
            merge_base_sha: merge_base.clone(),
            head_sha: String::new(),
            notation: format!("{merge_base}...HEAD"),
        },
        files,
        symbols,
        evidence,
        context,
        relations,
        checks,
        linked_issues,
        coverage: Coverage {
            complete: omitted.is_empty() && warnings.is_empty(),
            checked,
            omitted,
            warnings,
            input_bytes: 0,
            input_budget_bytes: INPUT_BUDGET,
        },
    };
    bundle.compare.target_branch_sha = bundle.pull_request.base_sha.clone();
    bundle.compare.head_sha = bundle.pull_request.head_sha.clone();
    bundle.compare.notation = format!(
        "{}...{}",
        bundle.compare.merge_base_sha, bundle.compare.head_sha
    );
    bundle.coverage.input_bytes = serde_json::to_vec(&bundle)?.len();
    if bundle.coverage.input_bytes > INPUT_BUDGET {
        bundle.coverage.complete = false;
        bundle.coverage.warnings.push(format!(
            "事实数据为 {} 字节，超过模型输入预算 {} 字节；模型阶段会再次裁剪。",
            bundle.coverage.input_bytes, INPUT_BUDGET
        ));
    }
    Ok(bundle)
}

#[derive(Clone, Debug)]
struct PathChange {
    status: String,
    path: String,
    previous_path: Option<String>,
}

fn ensure_git_objects(pr: &PullRequest) -> Result<()> {
    if git_status(&["cat-file", "-e", &format!("{}^{{commit}}", pr.base_sha)])? != 0 {
        run_git(&[
            "fetch",
            "--no-tags",
            "--filter=blob:none",
            "origin",
            &pr.base_sha,
        ])?;
    }
    let pull_ref = format!(
        "+refs/pull/{}/head:refs/remotes/pr-guide/{}",
        pr.number, pr.number
    );
    run_git(&[
        "fetch",
        "--no-tags",
        "--filter=blob:none",
        "origin",
        &pull_ref,
    ])?;
    let fetched = git_text(&["rev-parse", &format!("refs/remotes/pr-guide/{}", pr.number)])?;
    if fetched != pr.head_sha {
        anyhow::bail!(
            "fetched PR head {fetched} does not match GitHub API head {}",
            pr.head_sha
        );
    }
    Ok(())
}

fn changed_paths(base: &str, head: &str) -> Result<Vec<PathChange>> {
    let output = git_bytes(&[
        "diff",
        "--name-status",
        "-z",
        "--find-renames",
        base,
        head,
        "--",
    ])?;
    let fields: Vec<&[u8]> = output
        .split(|byte| *byte == 0)
        .filter(|v| !v.is_empty())
        .collect();
    let mut changes = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let status = std::str::from_utf8(fields[index]).context("non-UTF-8 Git status")?;
        index += 1;
        if status.starts_with('R') || status.starts_with('C') {
            let previous = utf8_field(fields.get(index), "rename source")?;
            let path = utf8_field(fields.get(index + 1), "rename target")?;
            index += 2;
            changes.push(PathChange {
                status: if status.starts_with('R') {
                    "renamed".to_owned()
                } else {
                    "copied".to_owned()
                },
                path,
                previous_path: Some(previous),
            });
        } else {
            let path = utf8_field(fields.get(index), "changed path")?;
            index += 1;
            changes.push(PathChange {
                status: match status.chars().next() {
                    Some('A') => "added",
                    Some('D') => "removed",
                    Some('M') => "modified",
                    Some('T') => "type_changed",
                    _ => "changed",
                }
                .to_owned(),
                path,
                previous_path: None,
            });
        }
    }
    changes.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(changes)
}

fn diff_for_path(base: &str, head: &str, previous: Option<&str>, path: &str) -> Result<String> {
    let mut args = vec![
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--find-renames",
        "--unified=40",
        base,
        head,
        "--",
    ];
    if let Some(previous) = previous {
        args.push(previous);
    }
    args.push(path);
    Ok(String::from_utf8_lossy(&git_bytes(&args)?).into_owned())
}

fn numstat_is_binary(base: &str, head: &str, path: &str) -> Result<bool> {
    let value = git_text(&["diff", "--numstat", base, head, "--", path])?;
    Ok(value.lines().any(|line| line.starts_with("-\t-\t")))
}

fn read_blob_excerpt(sha: &str, path: &str, omitted: &mut Vec<String>) -> Result<Option<String>> {
    let spec = format!("{sha}:{path}");
    let size_text = match git_optional_text(&["cat-file", "-s", &spec])? {
        Some(value) => value,
        None => return Ok(None),
    };
    let size = size_text
        .parse::<u64>()
        .with_context(|| format!("invalid Git blob size for {path}"))?;
    if size > BLOB_LIMIT {
        omitted.push(format!("{path} 为 {size} 字节，超过源码读取上限。"));
        return Ok(None);
    }
    let bytes = git_bytes(&["show", &spec])?;
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            omitted.push(format!("{path} 不是 UTF-8 文本，未读取内容。"));
            return Ok(None);
        }
    };
    Ok(Some(redact(&safe_prefix(&text, SOURCE_EXCERPT_LIMIT))))
}

fn fit_source_budget(
    excerpt: Option<String>,
    path: &str,
    used: &mut usize,
    omitted: &mut Vec<String>,
) -> Option<String> {
    let excerpt = excerpt?;
    let remaining = SOURCE_TOTAL_LIMIT.saturating_sub(*used);
    if remaining == 0 {
        omitted.push(format!("{path} 的源码摘录因总预算耗尽而省略。"));
        return None;
    }
    let fitted = safe_prefix(&excerpt, remaining.min(SOURCE_EXCERPT_LIMIT));
    *used += fitted.len();
    if fitted.len() < excerpt.len() {
        omitted.push(format!("{path} 的源码摘录因输入预算而截断。"));
    }
    Some(fitted)
}

#[derive(Clone)]
struct Decl {
    name: String,
    kind: String,
    visibility: String,
    signature: String,
    line: u64,
    line_end: u64,
}

fn collect_symbols(
    repository: &str,
    base: &str,
    head: &str,
    files: &[FileFact],
    evidence: &mut Vec<Evidence>,
) -> Result<Vec<SymbolFact>> {
    let mut symbols = Vec::new();
    for file in files.iter().filter(|file| file.path.ends_with(".rs")) {
        let before_path = file.previous_path.as_deref().unwrap_or(&file.path);
        let before = if file.status == "added" {
            BTreeMap::new()
        } else {
            declarations(&read_blob_text(base, before_path)?.unwrap_or_default())
        };
        let after = if file.status == "removed" {
            BTreeMap::new()
        } else {
            declarations(&read_blob_text(head, &file.path)?.unwrap_or_default())
        };
        let keys: BTreeSet<String> = before.keys().chain(after.keys()).cloned().collect();
        for key in keys {
            let old = before.get(&key);
            let new = after.get(&key);
            if old.map(|value| &value.signature) == new.map(|value| &value.signature) {
                continue;
            }
            let mut evidence_ids = Vec::new();
            if let Some(old) = old {
                evidence_ids.push(push_decl_evidence(
                    evidence,
                    repository,
                    "before",
                    before_path,
                    base,
                    old,
                ));
            }
            if let Some(new) = new {
                evidence_ids.push(push_decl_evidence(
                    evidence, repository, "after", &file.path, head, new,
                ));
            }
            let sample = new
                .or(old)
                .context("symbol comparison lost both declarations")?;
            symbols.push(SymbolFact {
                name: sample.name.clone(),
                kind: sample.kind.clone(),
                visibility: sample.visibility.clone(),
                change: match (old, new) {
                    (None, Some(_)) => "added",
                    (Some(_), None) => "removed",
                    _ => "signature_modified",
                }
                .to_owned(),
                before_signature: old.map(|value| value.signature.clone()),
                after_signature: new.map(|value| value.signature.clone()),
                evidence_ids,
            });
        }
    }
    symbols.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(symbols)
}

fn declarations(source: &str) -> BTreeMap<String, Decl> {
    let mut result = BTreeMap::new();
    let lines: Vec<&str> = source.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        let normalized = trimmed
            .strip_prefix("pub(crate) ")
            .or_else(|| trimmed.strip_prefix("pub(super) "))
            .or_else(|| trimmed.strip_prefix("pub "))
            .unwrap_or(trimmed);
        let visibility = if trimmed.starts_with("pub ") || trimmed.starts_with("pub(") {
            "public"
        } else {
            "private"
        };
        let mut matched = None;
        for prefix in [
            "async fn ",
            "const fn ",
            "fn ",
            "struct ",
            "enum ",
            "trait ",
            "type ",
            "const ",
            "static ",
            "mod ",
        ] {
            if let Some(rest) = normalized.strip_prefix(prefix) {
                let name: String = rest
                    .chars()
                    .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                    .collect();
                if !name.is_empty() {
                    matched = Some((prefix.trim().replace(' ', "_"), name));
                }
                break;
            }
        }
        if let Some((kind, name)) = matched {
            let key = format!("{kind}:{name}");
            let (signature, consumed) = declaration_signature(&lines, index, &kind);
            result.insert(
                key,
                Decl {
                    name,
                    kind,
                    visibility: visibility.to_owned(),
                    signature: redact(&safe_prefix(&signature, 8_000)),
                    line: index as u64 + 1,
                    line_end: index as u64 + consumed as u64,
                },
            );
        }
    }
    result
}

fn declaration_signature(lines: &[&str], start: usize, kind: &str) -> (String, usize) {
    let block = matches!(kind, "struct" | "enum" | "trait");
    let callable = matches!(kind, "fn" | "async_fn" | "const_fn");
    if !block && !callable {
        return (lines[start].trim().to_owned(), 1);
    }
    let limit = if block { 120 } else { 24 };
    let mut selected = Vec::new();
    let mut braces = 0_i64;
    let mut parentheses = 0_i64;
    let mut saw_brace = false;
    for line in lines.iter().skip(start).take(limit) {
        let trimmed = line.trim();
        selected.push(trimmed);
        for character in trimmed.chars() {
            match character {
                '{' => {
                    saw_brace = true;
                    braces += 1;
                }
                '}' => braces -= 1,
                '(' => parentheses += 1,
                ')' => parentheses -= 1,
                _ => {}
            }
        }
        if block && ((saw_brace && braces <= 0) || (!saw_brace && trimmed.ends_with(';'))) {
            break;
        }
        if callable && parentheses <= 0 && (trimmed.contains('{') || trimmed.ends_with(';')) {
            break;
        }
    }
    let consumed = selected.len().max(1);
    (selected.join("\n"), consumed)
}

#[allow(
    clippy::too_many_arguments,
    reason = "evidence creation requires the immutable source identity and declaration together"
)]
fn push_decl_evidence(
    evidence: &mut Vec<Evidence>,
    repository: &str,
    side: &str,
    path: &str,
    sha: &str,
    decl: &Decl,
) -> String {
    let id = next_evidence_id(evidence);
    evidence.push(Evidence {
        id: id.clone(),
        side: side.to_owned(),
        path: path.to_owned(),
        sha: sha.to_owned(),
        line_start: decl.line,
        line_end: decl.line_end,
        url: source_url(repository, sha, path, decl.line, decl.line_end),
        excerpt: decl.signature.clone(),
        reason: format!("{} {} 声明", decl.kind, decl.name),
    });
    id
}

fn collect_structural_context(
    repository: &str,
    head: &str,
    files: &[FileFact],
    evidence: &mut Vec<Evidence>,
    omitted: &mut Vec<String>,
) -> Result<Vec<ContextFact>> {
    let mut paths = BTreeSet::from(["Cargo.toml".to_owned()]);
    for file in files {
        let path = Path::new(&file.path);
        let mut cursor = path.parent();
        while let Some(parent) = cursor {
            let cargo = parent.join("Cargo.toml");
            if git_blob_exists(head, &cargo) {
                paths.insert(path_string(&cargo)?);
                break;
            }
            cursor = parent.parent();
        }
        if file.path.ends_with(".rs") {
            if let Some(parent) = path.parent() {
                for candidate in [parent.join("mod.rs"), parent.join("lib.rs")] {
                    if git_blob_exists(head, &candidate) {
                        paths.insert(path_string(&candidate)?);
                    }
                }
            }
        }
    }
    let mut context = Vec::new();
    let mut context_bytes = 0;
    for path in paths.into_iter().take(CONTEXT_LIMIT) {
        let Some(source) = read_blob_text(head, &path)? else {
            continue;
        };
        let remaining = 80_000_usize.saturating_sub(context_bytes);
        if remaining == 0 {
            omitted.push("结构上下文达到 80,000 字节输入上限。".to_owned());
            break;
        }
        let excerpt = redact(&safe_prefix(&source, remaining.min(8_000)));
        context_bytes += excerpt.len();
        let id = next_evidence_id(evidence);
        evidence.push(Evidence {
            id: id.clone(),
            side: "after".to_owned(),
            path: path.clone(),
            sha: head.to_owned(),
            line_start: 1,
            line_end: excerpt.lines().count().max(1) as u64,
            url: source_url(
                repository,
                head,
                &path,
                1,
                excerpt.lines().count().max(1) as u64,
            ),
            excerpt: excerpt.lines().take(12).collect::<Vec<_>>().join("\n"),
            reason: "Cargo workspace、相关 crate 清单或模块入口".to_owned(),
        });
        context.push(ContextFact {
            kind: "structure".to_owned(),
            path,
            side: "after".to_owned(),
            reason: "理解 crate 边界和模块声明所需的结构上下文".to_owned(),
            excerpt,
            evidence_id: Some(id),
        });
    }
    if context.len() >= CONTEXT_LIMIT {
        omitted.push("结构上下文达到数量上限。".to_owned());
    }
    Ok(context)
}

#[allow(
    clippy::too_many_arguments,
    reason = "reference collection updates the shared evidence and context ledgers"
)]
fn collect_symbol_references(
    repository: &str,
    head: &str,
    files: &[FileFact],
    symbols: &[SymbolFact],
    evidence: &mut Vec<Evidence>,
    context: &mut Vec<ContextFact>,
) -> Result<()> {
    let changed: BTreeSet<&str> = files.iter().map(|file| file.path.as_str()).collect();
    for symbol in symbols.iter().take(24) {
        if symbol.name.len() < 3 {
            continue;
        }
        let output =
            git_optional_bytes(&["grep", "-n", "-F", "-e", &symbol.name, head, "--", "*.rs"])?;
        let Some(output) = output else {
            continue;
        };
        let text = String::from_utf8_lossy(&output);
        let prefix = format!("{head}:");
        for line in text.lines().take(8) {
            let Some(rest) = line.strip_prefix(&prefix) else {
                continue;
            };
            let mut parts = rest.splitn(3, ':');
            let Some(path) = parts.next() else { continue };
            let Some(line_number) = parts.next().and_then(|value| value.parse::<u64>().ok()) else {
                continue;
            };
            let snippet = parts.next().unwrap_or_default();
            if changed.contains(path) && snippet.contains("fn ") {
                continue;
            }
            let id = next_evidence_id(evidence);
            evidence.push(Evidence {
                id: id.clone(),
                side: "after".to_owned(),
                path: path.to_owned(),
                sha: head.to_owned(),
                line_start: line_number,
                line_end: line_number,
                url: source_url(repository, head, path, line_number, line_number),
                excerpt: redact(snippet.trim()),
                reason: format!("{} 的文本引用；不单独证明实际调用", symbol.name),
            });
            context.push(ContextFact {
                kind: if path.contains("/tests/") || path.ends_with("_test.rs") {
                    "test_reference"
                } else {
                    "lexical_reference"
                }
                .to_owned(),
                path: path.to_owned(),
                side: "after".to_owned(),
                reason: format!(
                    "{} 的文本引用；trait、宏或同名符号可能使调用关系不确定",
                    symbol.name
                ),
                excerpt: redact(snippet.trim()),
                evidence_id: Some(id),
            });
            if context.len() >= CONTEXT_LIMIT {
                return Ok(());
            }
        }
    }
    Ok(())
}

fn collect_import_relations(
    repository: &str,
    head: &str,
    files: &[FileFact],
    evidence: &mut Vec<Evidence>,
) -> Vec<RelationFact> {
    let mut relations = Vec::new();
    for file in files.iter().filter(|file| file.path.ends_with(".rs")) {
        let Some(source) = file.after_excerpt.as_deref() else {
            continue;
        };
        for (index, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if !(trimmed.starts_with("use crate::") || trimmed.starts_with("use super::")) {
                continue;
            }
            let target = trimmed
                .trim_start_matches("use ")
                .trim_end_matches(';')
                .split("::{")
                .next()
                .unwrap_or(trimmed);
            let line_number = index as u64 + 1;
            let id = next_evidence_id(evidence);
            evidence.push(Evidence {
                id: id.clone(),
                side: "after".to_owned(),
                path: file.path.clone(),
                sha: head.to_owned(),
                line_start: line_number,
                line_end: line_number,
                url: source_url(repository, head, &file.path, line_number, line_number),
                excerpt: redact(trimmed),
                reason: "Rust use 声明证明静态导入依赖，不证明运行时调用".to_owned(),
            });
            relations.push(RelationFact {
                from: file.module.clone(),
                to: target.to_owned(),
                relation_type: "dependency".to_owned(),
                change: file.status.clone(),
                certainty: "confirmed_import".to_owned(),
                evidence_ids: vec![id],
            });
            if relations.len() >= 40 {
                return relations;
            }
        }
    }
    relations
}

fn first_hunk_line(patch: &str, before: bool) -> u64 {
    for line in patch.lines() {
        if let Some(rest) = line.strip_prefix("@@ -") {
            let Some((old, new_and_rest)) = rest.split_once(" +") else {
                continue;
            };
            let new = new_and_rest.split_whitespace().next().unwrap_or("1");
            let selected = if before { old } else { new };
            return selected
                .split(',')
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(1)
                .max(1);
        }
    }
    1
}

fn module_for_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.first() == Some(&"crates") && parts.len() >= 3 {
        format!("{}/{}/{}", parts[0], parts[1], parts[2])
    } else if parts.len() >= 2 {
        format!("{}/{}", parts[0], parts[1])
    } else {
        path.to_owned()
    }
}

fn source_url(repository: &str, sha: &str, path: &str, start: u64, end: u64) -> String {
    let encoded = path
        .split('/')
        .map(percent_encode)
        .collect::<Vec<_>>()
        .join("/");
    if start == end {
        format!("https://github.com/{repository}/blob/{sha}/{encoded}#L{start}")
    } else {
        format!("https://github.com/{repository}/blob/{sha}/{encoded}#L{start}-L{end}")
    }
}

fn percent_encode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~') {
            output.push(*byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

pub(crate) fn redact(value: &str) -> String {
    value
        .lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            if [
                "api_key",
                "apikey",
                "secret",
                "password",
                "authorization",
                "bearer ",
            ]
            .iter()
            .any(|needle| lower.contains(needle))
            {
                let indent: String = line.chars().take_while(|ch| ch.is_whitespace()).collect();
                format!("{indent}[REDACTED: sensitive-looking line]")
            } else {
                redact_token_words(line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_token_words(line: &str) -> String {
    line.split_inclusive(char::is_whitespace)
        .map(|word| {
            let trimmed = word.trim();
            if (trimmed.starts_with("sk-")
                || trimmed.starts_with("ghp_")
                || trimmed.starts_with("github_pat_"))
                && trimmed.len() > 12
            {
                word.replace(trimmed, "[REDACTED_TOKEN]")
            } else {
                word.to_owned()
            }
        })
        .collect()
}

fn next_evidence_id(evidence: &[Evidence]) -> String {
    format!("E{}", evidence.len() + 1)
}

fn read_blob_text(sha: &str, path: &str) -> Result<Option<String>> {
    let spec = format!("{sha}:{path}");
    let Some(size) = git_optional_text(&["cat-file", "-s", &spec])? else {
        return Ok(None);
    };
    if size.parse::<u64>().unwrap_or(BLOB_LIMIT + 1) > BLOB_LIMIT {
        return Ok(None);
    }
    let Some(bytes) = git_optional_bytes(&["show", &spec])? else {
        return Ok(None);
    };
    Ok(String::from_utf8(bytes).ok())
}

fn blob_size(sha: &str, path: &str) -> Result<Option<u64>> {
    let spec = format!("{sha}:{path}");
    git_optional_text(&["cat-file", "-s", &spec])?
        .map(|value| {
            value
                .parse::<u64>()
                .with_context(|| format!("invalid Git blob size for {path}"))
        })
        .transpose()
}

fn git_blob_exists(sha: &str, path: &Path) -> bool {
    let Ok(path) = path.to_str().context("non-UTF-8 path") else {
        return false;
    };
    git_status(&["cat-file", "-e", &format!("{sha}:{path}")]).unwrap_or(1) == 0
}

fn path_string(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_owned)
        .context("repository path is not UTF-8")
}

fn utf8_field(field: Option<&&[u8]>, label: &str) -> Result<String> {
    let bytes = field.with_context(|| format!("Git output is missing {label}"))?;
    std::str::from_utf8(bytes)
        .with_context(|| format!("{label} is not UTF-8"))
        .map(str::to_owned)
}

fn validate_sha(value: &str) -> Result<()> {
    if (40..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        anyhow::bail!("Git returned an invalid commit SHA")
    }
}

fn safe_prefix(value: &str, byte_limit: usize) -> String {
    if value.len() <= byte_limit {
        return value.to_owned();
    }
    let mut end = byte_limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n[TRUNCATED]", &value[..end])
}

fn git_text(args: &[&str]) -> Result<String> {
    let bytes = git_bytes(args)?;
    String::from_utf8(bytes)
        .context("Git returned non-UTF-8 output")
        .map(|value| value.trim().to_owned())
}

fn git_optional_text(args: &[&str]) -> Result<Option<String>> {
    match git_optional_bytes(args)? {
        Some(bytes) => Ok(Some(
            String::from_utf8(bytes)
                .context("Git returned non-UTF-8 output")?
                .trim()
                .to_owned(),
        )),
        None => Ok(None),
    }
}

fn git_bytes(args: &[&str]) -> Result<Vec<u8>> {
    let output = git_output(args)?;
    if !output.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.first().copied().unwrap_or("command"),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

fn git_optional_bytes(args: &[&str]) -> Result<Option<Vec<u8>>> {
    let output = git_output(args)?;
    if output.status.success() {
        Ok(Some(output.stdout))
    } else if output.status.code() == Some(1) || output.status.code() == Some(128) {
        Ok(None)
    } else {
        anyhow::bail!(
            "git {} failed: {}",
            args.first().copied().unwrap_or("command"),
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }
}

fn git_status(args: &[&str]) -> Result<i32> {
    Ok(git_output(args)?.status.code().unwrap_or(128))
}

fn run_git(args: &[&str]) -> Result<()> {
    git_bytes(args).map(|_| ())
}

fn git_output(args: &[&str]) -> Result<Output> {
    Command::new("git")
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .with_context(|| format!("failed to start git {}", args.join(" ")))
}
