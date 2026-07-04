use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::resolve_harness_repo;
use crate::error::{bot_err, Result};
use crate::github::{installation_token, GitHubClient};
use crate::model::{BenchmarkData, GateResult};
use crate::publish;
use crate::render::render_pr_comment;
use crate::verify;

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
            let body = render_comment_from_paths(&benchmark_data, &gate_result)?;
            print!("{body}");
        }
        Commands::PostPrComment {
            benchmark_data,
            gate_result,
            github_token,
            github_repository,
            pr_number,
        } => {
            let body = render_comment_from_paths(&benchmark_data, &gate_result)?;
            let (owner, repo) = crate::github::split_repo(&github_repository)?;
            let client = comment_client(&github_token).await?;
            client
                .post_issue_comment(owner, repo, pr_number, &body)
                .await?;
            println!("post-pr-comment: comment posted");
        }
    }
    Ok(())
}

async fn app_client() -> Result<GitHubClient> {
    let app_id = std::env::var("APP_ID").map_err(|_| bot_err("missing APP_ID".to_string()))?;
    let private_key = std::env::var("APP_PRIVATE_KEY")
        .map_err(|_| bot_err("missing APP_PRIVATE_KEY".to_string()))?;
    let installation_id = std::env::var("INSTALLATION_ID")
        .map_err(|_| bot_err("missing INSTALLATION_ID".to_string()))?;
    let token = installation_token(&app_id, &private_key, &installation_id).await?;
    Ok(GitHubClient::new(token))
}

async fn comment_client(fallback_token: &str) -> Result<GitHubClient> {
    let has_app = std::env::var("APP_ID").is_ok()
        && std::env::var("APP_PRIVATE_KEY").is_ok()
        && std::env::var("INSTALLATION_ID").is_ok();
    if has_app {
        return app_client().await;
    }
    Ok(GitHubClient::new(fallback_token.to_string()))
}

fn render_comment_from_paths(benchmark_data: &PathBuf, gate_result: &PathBuf) -> Result<String> {
    let data = read_json::<BenchmarkData>(benchmark_data)?;
    let gate = if gate_result.exists() {
        read_json::<GateResult>(gate_result)?
    } else {
        GateResult {
            status: None,
            regression_pct: None,
            checks: Some(vec![]),
        }
    };
    render_pr_comment(&data, &gate).map_err(bot_err)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T> {
    let bytes =
        fs::read(path).map_err(|err| bot_err(format!("read {} failed: {err}", path.display())))?;
    serde_json::from_slice(&bytes).map_err(|err| bot_err(format!("json decode failed: {err}")))
}
