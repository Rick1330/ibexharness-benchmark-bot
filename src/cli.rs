use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::resolve_harness_repo;
use crate::error::{bot_err, Result};
use crate::github::{
    installation_token, CommitStatus, GitHubClient, IssueCommentUpdate, IssueRef, RepoRef,
};
use crate::model::{
    BenchmarkData, BenchmarkRun, GateResult, HnswBenchmarkData, RankingQualityBenchmarkData,
    WritePipelineBenchmarkData,
};
use crate::publish;
use crate::publish_hnsw;
use crate::publish_ranking_quality;
use crate::publish_write_pipeline;
use crate::render::{
    merge_hnsw_into_comment, merge_proxy_into_comment, merge_ranking_quality_into_comment,
    merge_write_pipeline_into_comment, render_hnsw_pr_comment, render_pr_comment,
    render_ranking_quality_pr_comment, render_write_pipeline_pr_comment, COMMENT_MARKER,
    COMMENT_MARKER_HNSW,
};
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
    /// Download HNSW artifact, validate, and open a memory benchmark data PR
    PublishHnsw {
        #[arg(long)]
        payload: String,
        #[arg(long, env = "HARNESS_REPO", default_value = "Rick1330/ibex-harness")]
        repo: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Verify a memory-benchmark repository_dispatch payload
    VerifyHnswDispatch {
        #[arg(long)]
        payload: String,
        #[arg(long, env = "HARNESS_REPO", default_value = "Rick1330/ibex-harness")]
        repo: String,
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
    /// Render a Memory HNSW PR comment to stdout
    RenderHnswPrComment {
        #[arg(long, env = "HNSW_BENCHMARK_DATA_PATH")]
        hnsw_benchmark_data: PathBuf,
    },
    /// Render and post a Memory HNSW section on the shared sticky comment
    PostHnswPrComment {
        #[arg(long, env = "HNSW_BENCHMARK_DATA_PATH")]
        hnsw_benchmark_data: PathBuf,
        #[arg(long, env = "GITHUB_TOKEN")]
        github_token: String,
        #[arg(long, env = "GITHUB_REPOSITORY")]
        github_repository: String,
        #[arg(long, env = "PR_NUMBER")]
        pr_number: i64,
    },
    /// Download ranking-quality artifact, validate, and open a memory benchmark data PR
    PublishRanking {
        #[arg(long)]
        payload: String,
        #[arg(long, env = "HARNESS_REPO", default_value = "Rick1330/ibex-harness")]
        repo: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Download write-pipeline artifact, validate, and open a memory benchmark data PR
    PublishWrite {
        #[arg(long)]
        payload: String,
        #[arg(long, env = "HARNESS_REPO", default_value = "Rick1330/ibex-harness")]
        repo: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Render a ranking-quality PR comment to stdout
    RenderRankingPrComment {
        #[arg(long, env = "RANKING_QUALITY_BENCHMARK_DATA_PATH")]
        ranking_benchmark_data: PathBuf,
    },
    /// Render and post a ranking-quality section on the shared sticky comment
    PostRankingPrComment {
        #[arg(long, env = "RANKING_QUALITY_BENCHMARK_DATA_PATH")]
        ranking_benchmark_data: PathBuf,
        #[arg(long, env = "GITHUB_TOKEN")]
        github_token: String,
        #[arg(long, env = "GITHUB_REPOSITORY")]
        github_repository: String,
        #[arg(long, env = "PR_NUMBER")]
        pr_number: i64,
    },
    /// Render a write-pipeline PR comment to stdout
    RenderWritePrComment {
        #[arg(long, env = "WRITE_PIPELINE_BENCHMARK_DATA_PATH")]
        write_benchmark_data: PathBuf,
    },
    /// Render and post a write-pipeline section on the shared sticky comment
    PostWritePrComment {
        #[arg(long, env = "WRITE_PIPELINE_BENCHMARK_DATA_PATH")]
        write_benchmark_data: PathBuf,
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
        Commands::PublishHnsw {
            payload,
            repo,
            dry_run,
        } => {
            let repo = locked_repo(&repo)?;
            let parsed = verify::parse_payload_json(&payload)?;
            let client = app_client().await?;
            let result =
                publish_hnsw::publish_hnsw_benchmark_data(&client, repo, &parsed, dry_run).await?;
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
        Commands::VerifyHnswDispatch { payload, repo } => {
            let repo = locked_repo(&repo)?;
            let parsed = verify::parse_payload_json(&payload)?;
            let client = app_client().await?;
            verify::verify_hnsw_dispatch(&client, repo, &parsed).await?;
            println!("{{\"ok\":true}}");
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
            let data = read_json::<BenchmarkData>(&benchmark_data)?;
            let gate = load_gate_result(&gate_result);
            let (owner, repo) = crate::github::split_repo(&github_repository)?;
            let repo_ref = RepoRef::new(owner, repo);
            let client = comment_client(&github_token).await?;
            let existing = existing_sticky_body(&client, repo_ref, pr_number).await?;
            let body = merge_proxy_into_comment(&existing, &data, &gate).map_err(bot_err)?;
            upsert_pr_comment(&client, repo_ref, pr_number, &body, COMMENT_MARKER).await?;
            post_commit_status(&client, repo_ref, &data, &gate).await?;
        }
        Commands::RenderHnswPrComment {
            hnsw_benchmark_data,
        } => {
            let body = render_hnsw_comment_from_path(&hnsw_benchmark_data)?;
            print!("{body}");
        }
        Commands::PostHnswPrComment {
            hnsw_benchmark_data,
            github_token,
            github_repository,
            pr_number,
        } => {
            let data = read_json::<HnswBenchmarkData>(&hnsw_benchmark_data)?;
            let (owner, repo) = crate::github::split_repo(&github_repository)?;
            let repo_ref = RepoRef::new(owner, repo);
            let client = comment_client(&github_token).await?;
            let existing = existing_sticky_body(&client, repo_ref, pr_number).await?;
            let body = merge_hnsw_into_comment(&existing, &data).map_err(bot_err)?;
            upsert_pr_comment(&client, repo_ref, pr_number, &body, COMMENT_MARKER).await?;
        }
        Commands::PublishRanking {
            payload,
            repo,
            dry_run,
        } => {
            let repo = locked_repo(&repo)?;
            let parsed = verify::parse_payload_json(&payload)?;
            let client = app_client().await?;
            let result = publish_ranking_quality::publish_ranking_quality_benchmark_data(
                &client, repo, &parsed, dry_run,
            )
            .await?;
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
        Commands::PublishWrite {
            payload,
            repo,
            dry_run,
        } => {
            let repo = locked_repo(&repo)?;
            let parsed = verify::parse_payload_json(&payload)?;
            let client = app_client().await?;
            let result = publish_write_pipeline::publish_write_pipeline_benchmark_data(
                &client, repo, &parsed, dry_run,
            )
            .await?;
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
        Commands::RenderRankingPrComment {
            ranking_benchmark_data,
        } => {
            let body = render_ranking_comment_from_path(&ranking_benchmark_data)?;
            print!("{body}");
        }
        Commands::PostRankingPrComment {
            ranking_benchmark_data,
            github_token,
            github_repository,
            pr_number,
        } => {
            let data = read_json::<RankingQualityBenchmarkData>(&ranking_benchmark_data)?;
            let (owner, repo) = crate::github::split_repo(&github_repository)?;
            let repo_ref = RepoRef::new(owner, repo);
            let client = comment_client(&github_token).await?;
            let existing = existing_sticky_body(&client, repo_ref, pr_number).await?;
            let body = merge_ranking_quality_into_comment(&existing, &data).map_err(bot_err)?;
            upsert_pr_comment(&client, repo_ref, pr_number, &body, COMMENT_MARKER).await?;
        }
        Commands::RenderWritePrComment {
            write_benchmark_data,
        } => {
            let body = render_write_comment_from_path(&write_benchmark_data)?;
            print!("{body}");
        }
        Commands::PostWritePrComment {
            write_benchmark_data,
            github_token,
            github_repository,
            pr_number,
        } => {
            let data = read_json::<WritePipelineBenchmarkData>(&write_benchmark_data)?;
            let (owner, repo) = crate::github::split_repo(&github_repository)?;
            let repo_ref = RepoRef::new(owner, repo);
            let client = comment_client(&github_token).await?;
            let existing = existing_sticky_body(&client, repo_ref, pr_number).await?;
            let body = merge_write_pipeline_into_comment(&existing, &data).map_err(bot_err)?;
            upsert_pr_comment(&client, repo_ref, pr_number, &body, COMMENT_MARKER).await?;
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

fn render_hnsw_comment_from_path(hnsw_benchmark_data: &PathBuf) -> Result<String> {
    let data = read_json::<HnswBenchmarkData>(hnsw_benchmark_data)?;
    render_hnsw_pr_comment(&data).map_err(bot_err)
}

fn render_ranking_comment_from_path(ranking_benchmark_data: &PathBuf) -> Result<String> {
    let data = read_json::<RankingQualityBenchmarkData>(ranking_benchmark_data)?;
    render_ranking_quality_pr_comment(&data).map_err(bot_err)
}

fn render_write_comment_from_path(write_benchmark_data: &PathBuf) -> Result<String> {
    let data = read_json::<WritePipelineBenchmarkData>(write_benchmark_data)?;
    render_write_pipeline_pr_comment(&data).map_err(bot_err)
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

async fn existing_sticky_body(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    pr_number: i64,
) -> Result<String> {
    let issue = IssueRef {
        repo,
        number: pr_number,
    };
    let comments = client.list_issue_comments(issue).await?;
    for marker in [COMMENT_MARKER, COMMENT_MARKER_HNSW] {
        if let Some(existing) = comments
            .iter()
            .find(|comment| comment_has_marker(comment, marker))
        {
            return Ok(existing
                .get("body")
                .and_then(|body| body.as_str())
                .unwrap_or("")
                .to_string());
        }
    }
    Ok(String::new())
}

async fn upsert_pr_comment(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    pr_number: i64,
    body: &str,
    marker: &str,
) -> Result<()> {
    let body = ensure_comment_marker(body, marker);
    let issue = IssueRef {
        repo,
        number: pr_number,
    };
    let comments = client.list_issue_comments(issue).await?;
    let existing = comments
        .iter()
        .find(|comment| comment_has_marker(comment, COMMENT_MARKER))
        .or_else(|| {
            comments
                .iter()
                .find(|comment| comment_has_marker(comment, COMMENT_MARKER_HNSW))
        });
    if let Some(existing) = existing {
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
        println!("post-pr-comment: comment updated ({marker})");
        return Ok(());
    }
    client.post_issue_comment(issue, &body).await?;
    println!("post-pr-comment: comment posted ({marker})");
    Ok(())
}

fn ensure_comment_marker(body: &str, marker: &str) -> String {
    if body.contains(marker) {
        body.to_string()
    } else {
        format!("{marker}\n{body}")
    }
}

fn comment_has_marker(comment: &serde_json::Value, marker: &str) -> bool {
    comment
        .get("body")
        .and_then(|body| body.as_str())
        .is_some_and(|body| body.contains(marker))
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
