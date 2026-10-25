use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Arg, ArgAction, ArgMatches, Command};
use sha2::{Digest, Sha256};

use self::types::{FactBundle, MarkerState, RunState, COMMENT_MARKER};

mod facts;
mod github;
mod model;
mod render;
mod types;

pub(crate) async fn run() -> Result<()> {
    let matches = command().get_matches();
    match matches.subcommand() {
        Some(("analyze", options)) => analyze(options).await,
        Some(("publish", options)) => publish(options).await,
        Some(("render-fixture", options)) => render_fixture(options).await,
        _ => anyhow::bail!("a pr-guide subcommand is required"),
    }
}

fn command() -> Command {
    Command::new("pr-guide")
        .about("Generate evidence-bound PR guides without executing PR code")
        .subcommand_required(true)
        .subcommand(
            Command::new("analyze").arg(pr_arg()).arg(output_arg()).arg(
                Arg::new("force")
                    .long("force")
                    .action(ArgAction::SetTrue)
                    .help("Regenerate even when the input fingerprint is unchanged"),
            ),
        )
        .subcommand(Command::new("publish").arg(pr_arg()).arg(output_arg()))
        .subcommand(
            Command::new("render-fixture")
                .arg(
                    Arg::new("input")
                        .long("input")
                        .value_name("FACTS_JSON")
                        .required(true),
                )
                .arg(output_arg()),
        )
}

fn pr_arg() -> Arg {
    Arg::new("pr")
        .long("pr")
        .value_name("NUMBER")
        .required(true)
        .value_parser(clap::value_parser!(u64).range(1..))
}

fn output_arg() -> Arg {
    Arg::new("output")
        .long("output")
        .value_name("DIRECTORY")
        .required(true)
}

async fn analyze(options: &ArgMatches) -> Result<()> {
    let number = required_number(options)?;
    let output = required_path(options, "output")?;
    let repository =
        std::env::var("GITHUB_REPOSITORY").context("GITHUB_REPOSITORY is required for analyze")?;
    let github = github::GitHub::from_env(&repository)?;
    let pull_request = github.pull_request(number).await?;
    if pull_request.state != "open" {
        anyhow::bail!("PR #{number} is not open; automatic generation stops after close")
    }
    let (api_files, checks, linked_issues, existing_comment) = tokio::try_join!(
        github.changed_files(number),
        github.checks(&pull_request.head_sha),
        github.linked_issues(&pull_request.body),
        github.guide_comment(number),
    )?;
    let facts = facts::collect(&repository, pull_request, api_files, checks, linked_issues)?;
    let fingerprint = fingerprint(&facts)?;
    let existing_marker = existing_comment
        .as_ref()
        .and_then(|comment| parse_marker(&comment.body));
    let force = options.get_flag("force")
        || std::env::var("PR_GUIDE_FORCE")
            .map(|value| matches!(value.as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
    let artifact_name = artifact_name(number, &facts.pull_request.head_sha);
    let generated_at = Utc::now().to_rfc3339();
    if !force
        && existing_marker
            .as_ref()
            .map(|marker| {
                marker.fingerprint == fingerprint && marker.head_sha == facts.pull_request.head_sha
            })
            .unwrap_or(false)
    {
        let state = RunState {
            reused: true,
            fingerprint,
            head_sha: facts.pull_request.head_sha.clone(),
            analysis_status: "reused_unchanged_result".to_owned(),
            model_status: "not_called".to_owned(),
            model_name: None,
            html_status: "reused_unchanged_result".to_owned(),
            comment_id: existing_comment.map(|comment| comment.id),
            artifact_name,
            generated_at,
        };
        render::write_reuse_state(&output, &state)?;
        println!("Input fingerprint is unchanged; existing PR guide remains current.");
        return Ok(());
    }

    let outcome = model::analyze(&facts).await;
    let html_outcome =
        model::generate_html(&facts, &outcome.analysis, &outcome.status).await;
    let generated_html = html_outcome.html;
    let html_status = html_outcome.status;
    let analysis_status = if facts.coverage.complete && outcome.status == "model_completed" {
        "complete"
    } else {
        "partial"
    }
    .to_owned();
    let state = RunState {
        reused: false,
        fingerprint,
        head_sha: facts.pull_request.head_sha.clone(),
        analysis_status,
        model_status: outcome.status,
        model_name: outcome.model_name,
        html_status,
        comment_id: existing_comment.map(|comment| comment.id),
        artifact_name,
        generated_at,
    };
    render::write_outputs(&output, &facts, &outcome.analysis, &state, generated_html.as_deref())?;
    println!("Generated deterministic facts, Markdown guide, and standalone HTML report.");
    Ok(())
}

async fn publish(options: &ArgMatches) -> Result<()> {
    let number = required_number(options)?;
    let output = required_path(options, "output")?;
    let state: RunState = serde_json::from_slice(
        &fs::read(output.join("state.json")).context("state.json is missing")?,
    )
    .context("state.json is invalid")?;
    if state.reused {
        println!("No publish needed; existing comment has the same input fingerprint.");
        return Ok(());
    }
    let repository =
        std::env::var("GITHUB_REPOSITORY").context("GITHUB_REPOSITORY is required for publish")?;
    let github = github::GitHub::from_env(&repository)?;
    let current = github.pull_request(number).await?;
    if current.state != "open" {
        anyhow::bail!("PR #{number} closed before publish; refusing to write a stale report")
    }
    if current.head_sha != state.head_sha {
        anyhow::bail!(
            "PR head changed from {} to {}; refusing to overwrite the current report",
            state.head_sha,
            current.head_sha
        )
    }
    let body = fs::read_to_string(output.join("comment.md"))
        .context("comment.md is missing or not UTF-8")?;
    let current_comment = github.guide_comment(number).await?;
    let comment_id = current_comment
        .as_ref()
        .map(|comment| comment.id)
        .or(state.comment_id);
    let comment = github
        .upsert_guide_comment(number, comment_id, &body)
        .await?;
    println!("Published PR guide comment: {}", comment.html_url);
    Ok(())
}

async fn render_fixture(options: &ArgMatches) -> Result<()> {
    let input = required_path(options, "input")?;
    let output = required_path(options, "output")?;
    let facts: FactBundle = serde_json::from_slice(
        &fs::read(&input).with_context(|| format!("failed to read {}", input.display()))?,
    )
    .context("fixture facts JSON is invalid")?;
    let fingerprint = fingerprint(&facts)?;
    let outcome = model::analyze(&facts).await;
    let html_outcome =
        model::generate_html(&facts, &outcome.analysis, &outcome.status).await;
    let generated_html = html_outcome.html;
    let state = RunState {
        reused: false,
        fingerprint,
        head_sha: facts.pull_request.head_sha.clone(),
        analysis_status: "fixture".to_owned(),
        model_status: outcome.status,
        model_name: outcome.model_name,
        html_status: html_outcome.status,
        comment_id: None,
        artifact_name: artifact_name(facts.pull_request.number, &facts.pull_request.head_sha),
        generated_at: Utc::now().to_rfc3339(),
    };
    render::write_outputs(&output, &facts, &outcome.analysis, &state, generated_html.as_deref())
}

fn fingerprint(facts: &FactBundle) -> Result<String> {
    let encoded = serde_json::to_vec(facts)?;
    let mut digest = Sha256::new();
    digest.update(b"burncloud-pr-guide-ai-html-v1");
    digest.update(include_bytes!("model.rs"));
    digest.update(include_bytes!("render.rs"));
    digest.update(include_bytes!("mod.rs"));
    digest.update(encoded);
    Ok(format!("{:x}", digest.finalize()))
}

fn parse_marker(body: &str) -> Option<MarkerState> {
    let start = body.find(COMMENT_MARKER)? + COMMENT_MARKER.len();
    let rest = &body[start..];
    let end = rest.find("-->")?;
    serde_json::from_str(rest[..end].trim()).ok()
}

fn artifact_name(number: u64, head_sha: &str) -> String {
    format!(
        "pr-guide-{number}-{}",
        head_sha.chars().take(12).collect::<String>()
    )
}

fn required_number(options: &ArgMatches) -> Result<u64> {
    options
        .get_one::<u64>("pr")
        .copied()
        .context("--pr is required")
}

fn required_path(options: &ArgMatches, name: &str) -> Result<PathBuf> {
    options
        .get_one::<String>(name)
        .map(PathBuf::from)
        .with_context(|| format!("--{name} is required"))
}
