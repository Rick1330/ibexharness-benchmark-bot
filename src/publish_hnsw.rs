//! Publish HNSW / memory benchmark history to the harness repo.

use std::fs;

use serde_json::Value;

use crate::config::{HNSW_BENCHMARK_DATA_LABEL, HNSW_BENCHMARK_DATA_PATH};
use crate::error::{bot_err, Result};
use crate::github::{
    bot_commit_message, split_repo, CommitFile, CommitFilesRequest, CreateBranch, GitHubClient,
    IssueRef, LabeledPrSearch, OpenPullRequest, RepoPathRef, RepoRef,
};
use crate::hnsw_artifact::extract_hnsw_artifact_zip;
use crate::hnsw_validate::{
    cross_check_hnsw_artifact_run, hnsw_max_published_run_number, hnsw_published_sha_exists,
    validate_hnsw_file,
};
use crate::model::{DispatchPayload, HnswBenchmarkData};
use crate::render::render_hnsw_data_pr_body;
use crate::verify;

pub struct PublishResult {
    pub skipped: bool,
    pub pr_url: Option<String>,
    pub branch: String,
}

pub async fn publish_hnsw_benchmark_data(
    client: &GitHubClient,
    repo_full: &str,
    payload: &DispatchPayload,
    dry_run: bool,
) -> Result<PublishResult> {
    let run = verify::verify_hnsw_dispatch(client, repo_full, payload).await?;
    let (owner, repo) = split_repo(repo_full)?;
    let repo_ref = RepoRef::new(owner, repo);
    let branch = format!("chore/hnsw-bench-data-{}", payload.run_number);
    let head_sha = run
        .head_sha
        .as_deref()
        .ok_or_else(|| bot_err("verified run missing head_sha".to_string()))?;

    if let Some(existing) = find_existing_publish_pr(client, repo_ref, &branch, head_sha).await? {
        return Ok(PublishResult {
            skipped: true,
            pr_url: existing
                .get("html_url")
                .and_then(|value| value.as_str())
                .map(str::to_owned),
            branch,
        });
    }

    ensure_not_replay(client, repo_ref, payload, head_sha).await?;

    let zip = client
        .download_hnsw_artifact_zip(repo_ref, payload.run_id)
        .await?;
    let extracted = extract_hnsw_artifact_zip(&zip)?;
    validate_hnsw_file(&extracted.json_path)?;

    let json_bytes = fs::read(&extracted.json_path)
        .map_err(|err| bot_err(format!("read hnsw benchmark json: {err}")))?;
    let benchmark_data: HnswBenchmarkData = serde_json::from_slice(&json_bytes)
        .map_err(|err| bot_err(format!("decode hnsw benchmark json: {err}")))?;
    cross_check_hnsw_artifact_run(&benchmark_data, &run, payload.run_id, payload.run_number)?;

    if dry_run {
        return Ok(PublishResult {
            skipped: false,
            pr_url: None,
            branch,
        });
    }

    let main_sha = client.main_sha(repo_ref).await?;
    if !client.ref_exists(repo_ref, &branch).await? {
        client
            .create_branch(CreateBranch {
                repo: repo_ref,
                branch: &branch,
                sha: &main_sha,
            })
            .await?;
    }

    let subject = format!(
        "chore(bench): hnsw benchmark data update (run #{})",
        payload.run_number
    );
    let message = bot_commit_message(&subject);
    client
        .commit_files(
            repo_ref,
            CommitFilesRequest {
                branch: &branch,
                message: &message,
                files: &[CommitFile {
                    path: HNSW_BENCHMARK_DATA_PATH,
                    bytes: &json_bytes,
                }],
            },
        )
        .await?;

    let body =
        render_hnsw_data_pr_body(&benchmark_data, run.html_url.as_deref(), payload.run_number);
    let title = subject;
    let pr = client
        .open_pull_request(OpenPullRequest {
            repo: repo_ref,
            branch: &branch,
            title: &title,
            body: &body,
        })
        .await?;
    if let Some(number) = pr.get("number").and_then(Value::as_i64) {
        client
            .add_labels(
                IssueRef {
                    repo: repo_ref,
                    number,
                },
                &["automated", HNSW_BENCHMARK_DATA_LABEL],
            )
            .await?;
    }

    Ok(PublishResult {
        skipped: false,
        pr_url: pr
            .get("html_url")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
        branch,
    })
}

async fn ensure_not_replay(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    payload: &DispatchPayload,
    head_sha: &str,
) -> Result<()> {
    let published = client
        .get_file_bytes(RepoPathRef {
            repo,
            path: HNSW_BENCHMARK_DATA_PATH,
            git_ref: "main",
        })
        .await?;
    let Some(bytes) = published else {
        return Ok(());
    };
    let data: HnswBenchmarkData = serde_json::from_slice(&bytes)
        .map_err(|err| bot_err(format!("decode published hnsw data: {err}")))?;
    if hnsw_published_sha_exists(&data, head_sha) {
        return Err(bot_err("head_sha already published on main".to_string()));
    }
    if let Some(max_run) = hnsw_max_published_run_number(&data) {
        if payload.run_number <= max_run {
            return Err(bot_err(format!(
                "run_number {} is not newer than published max {}",
                payload.run_number, max_run
            )));
        }
    }
    Ok(())
}

async fn find_existing_publish_pr(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    branch: &str,
    head_sha: &str,
) -> Result<Option<Value>> {
    if let Some(existing) = client.find_open_pr(repo, branch).await? {
        return Ok(Some(existing));
    }
    client
        .find_labeled_pr_with_sha(LabeledPrSearch {
            repo,
            label: HNSW_BENCHMARK_DATA_LABEL,
            head_sha,
        })
        .await
}
