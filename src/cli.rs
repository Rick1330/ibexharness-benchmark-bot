use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::resolve_harness_repo;
use crate::error::{bot_err, Result};
use crate::github::{
    installation_token, CommitStatus, GitHubClient, IssueCommentUpdate, IssueRef, RepoRef,
};
use crate::model::{BenchmarkData, BenchmarkRun, GateResult};
use crate::publish;
use crate::render::{render_pr_comment, COMMENT_MARKER};
use crate::verify;

const STATUS_CONTEXT: &str = "ibex-harness/benchmarks";

fn locked_repo(repo: &str) -> Result<&str> {
    resolve_harness_repo(repo).map_err(bot_err)
}

#[derive(Parser)]
#[command(name = "ibex-benchmark-bot", about = "IBEX Harness Benchmark Bot")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Verify a repository_dispatch payload against the Actions API
    VerifyDispatch {
        #[arg(long)]
        payload: String,
        #[arg(long, env = "HARNESS_REPO", default_value = "Rick1330/ibex-harness")]
        repo: String,
    },
    /// Download artifact, validate, and open a benchmark data PR
    Publish {
        #[arg(long)]
        payload: String,
        #[arg(long, env = "HARNESS_REPO", default_value = "Rick1330/ibex-harness")]
        repo: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Render a PR benchmark comment to stdout
    RenderPrComment {
        #[arg(long, env = "BENCHMARK_DATA_PATH")]
        benchmark_data: PathBuf,
        #[arg(long, env = "GATE_RESULT_PATH")]
        gate_result: PathBuf,
    },
    /// Render and post a PR benchmark comment
    PostPrComment {
        #[arg(long, env = "BENCHMARK_DATA_PATH")]
        benchmark_data: PathBuf,
        #[arg(long, env = "GATE_RESULT_PATH")]
        gate_result: PathBuf,
        #[arg(long, env = "GITHUB_TOKEN")]
        github_token: String,
        #[arg(long, env = "GITHUB_REPOSITORY")]
        github_repository: String,
        #[arg(long, env = "PR_NUMBER")]
        pr_number: i64,
    },
}

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::VerifyDispatch { payload, repo } => {
            let repo = locked_repo(&repo)?;
            let parsed = verify::parse_payload_json(&payload)?;
            let client = app_client().await?;
            verify::verify_dispatch(&client, repo, &parsed).await?;
            println!("{{\"ok\":true}}");
        }
        Commands::Publish {
            payload,
            repo,
            dry_run,
        } => {
            let repo = locked_repo(&repo)?;
            let parsed = verify::parse_payload_json(&payload)?;
            let client = app_client().await?;
            let result = publish::publish_benchmark_data(&client, repo, &parsed, dry_run).await?;
            println!(
                "{}",
                serde_json::json!({
                    "ok": true,
                    "skipped": result.skipped,
                    "branch": result.branch,
                    "pr_url": result.pr_url,
                    "dry_run": dry_run,
                })
            );
        }
        Commands::RenderPrComment {
            benchmark_data,
            gate_result,
        } => {
            let (body, _, _) = render_comment_from_paths(&benchmark_data, &gate_result)?;
            print!("{body}");
        }
        Commands::PostPrComment {
            benchmark_data,
            gate_result,
            github_token,
            github_repository,
            pr_number,
        } => {
            let (body, data, gate) = render_comment_from_paths(&benchmark_data, &gate_result)?;
            let (owner, repo) = crate::github::split_repo(&github_repository)?;
            let repo_ref = RepoRef::new(owner, repo);
            let client = comment_client(&github_token).await?;
            upsert_pr_comment(&client, repo_ref, pr_number, &body).await?;
            post_commit_status(&client, repo_ref, &data, &gate).await?;
        }
    }
    Ok(())
}

async fn app_client() -> Result<GitHubClient> {
    let app_id = require_env("APP_ID")?;
    let private_key = require_env("APP_PRIVATE_KEY")?;
    let installation_id = require_env("INSTALLATION_ID")?;
    let token = installation_token(&app_id, &private_key, &installation_id).await?;
    Ok(GitHubClient::new(token))
}

fn require_env(key: &str) -> Result<String> {
    let value = std::env::var(key).map_err(|_| bot_err(format!("missing {key}")))?;
    if value.trim().is_empty() {
        return Err(bot_err(format!("missing {key}")));
    }
    Ok(value)
}

fn env_nonempty(key: &str) -> bool {
    std::env::var(key)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

async fn comment_client(fallback_token: &str) -> Result<GitHubClient> {
    let has_app = env_nonempty("APP_ID")
        && env_nonempty("APP_PRIVATE_KEY")
        && env_nonempty("INSTALLATION_ID");
    if has_app {
        return app_client().await;
    }
    Ok(GitHubClient::new(fallback_token.to_string()))
}

fn render_comment_from_paths(
    benchmark_data: &PathBuf,
    gate_result: &PathBuf,
) -> Result<(String, BenchmarkData, GateResult)> {
    let data = read_json::<BenchmarkData>(benchmark_data)?;
    let gate = load_gate_result(gate_result);
    let body = render_pr_comment(&data, &gate).map_err(bot_err)?;
    Ok((body, data, gate))
}

fn load_gate_result(gate_result: &PathBuf) -> GateResult {
    if !gate_result.exists() {
        return empty_gate_result();
    }
    read_json::<GateResult>(gate_result).unwrap_or_else(|_| empty_gate_result())
}

fn empty_gate_result() -> GateResult {
    GateResult {
        status: None,
        regression_pct: None,
        checks: Some(vec![]),
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T> {
    let bytes =
        fs::read(path).map_err(|err| bot_err(format!("read {} failed: {err}", path.display())))?;
    serde_json::from_slice(&bytes).map_err(|err| bot_err(format!("json decode failed: {err}")))
}

async fn upsert_pr_comment(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    pr_number: i64,
    body: &str,
) -> Result<()> {
    let body = ensure_comment_marker(body);
    let issue = IssueRef {
        repo,
        number: pr_number,
    };
    let comments = client.list_issue_comments(issue).await?;
    if let Some(existing) = comments.iter().find(|comment| comment_has_marker(comment)) {
        let id = existing
            .get("id")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| bot_err("comment id missing".to_string()))?;
        client
            .update_issue_comment(IssueCommentUpdate {
                repo,
                comment_id: id,
                body: &body,
            })
            .await?;
        println!("post-pr-comment: comment updated");
        return Ok(());
    }
    client.post_issue_comment(issue, &body).await?;
    println!("post-pr-comment: comment posted");
    Ok(())
}

fn ensure_comment_marker(body: &str) -> String {
    if body.contains(COMMENT_MARKER) {
        body.to_string()
    } else {
        format!("{COMMENT_MARKER}\n{body}")
    }
}

fn comment_has_marker(comment: &serde_json::Value) -> bool {
    comment
        .get("body")
        .and_then(|body| body.as_str())
        .is_some_and(|body| body.contains(COMMENT_MARKER))
}

async fn post_commit_status(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    data: &BenchmarkData,
    gate: &GateResult,
) -> Result<()> {
    let Some(run) = data.runs.as_ref().and_then(|runs| runs.first()) else {
        return Ok(());
    };
    let Some(sha) = resolve_commit_sha(run) else {
        return Ok(());
    };
    let status = run.status.as_deref().unwrap_or("unknown");
    let (state, description) = commit_status_for(status, gate);
    match client
        .create_commit_status(CommitStatus {
            repo,
            sha: &sha,
            state,
            description,
            context: STATUS_CONTEXT,
        })
        .await
    {
        Ok(()) => println!("post-pr-comment: commit status {state}"),
        Err(err) => {
            eprintln!("post-pr-comment: commit status skipped: {err}");
        }
    }
    Ok(())
}

fn resolve_commit_sha(run: &BenchmarkRun) -> Option<String> {
    run.sha
        .as_deref()
        .filter(|sha| sha.len() >= 7)
        .map(str::to_string)
}

fn commit_status_for(status: &str, gate: &GateResult) -> (&'static str, &'static str) {
    let gate_ok = gate
        .checks
        .as_ref()
        .is_none_or(|checks| checks.iter().all(|check| check.ok.unwrap_or(false)));
    match (status, gate_ok) {
        ("pass", true) => ("success", "Benchmark gates passed"),
        ("regression", _) => ("failure", "Performance regression vs baseline"),
        ("fail", _) | (_, false) => ("failure", "Benchmark gate checks failed"),
        _ => ("error", "Benchmark status unknown"),
    }
}
