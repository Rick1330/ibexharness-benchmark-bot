//! Publish ranking-quality benchmark history to the harness repo.

use std::fs;

use crate::config::{DATA_PR_BRANCH, RANKING_QUALITY_BENCHMARK_DATA_PATH};
use crate::error::{bot_err, Result};
use crate::github::{
    bot_commit_message, split_repo, CommitFile, CommitFilesRequest, CreateBranch, GitHubClient,
    RepoPathRef, RepoRef,
};
use crate::model::{DispatchPayload, RankingQualityBenchmarkData};
use crate::publish_shared::{
    ranking_quality_head_already_on_branch, upsert_combined_data_pr, UpsertDataPr,
};
use crate::ranking_quality_artifact::extract_ranking_quality_artifact_zip;
use crate::ranking_quality_validate::{
    cross_check_ranking_quality_artifact_run, ranking_quality_max_published_run_number,
    ranking_quality_published_sha_exists, validate_ranking_quality_file,
};
use crate::verify;

pub struct PublishResult {
    pub skipped: bool,
    pub pr_url: Option<String>,
    pub branch: String,
}

pub async fn publish_ranking_quality_benchmark_data(
    client: &GitHubClient,
    repo_full: &str,
    payload: &DispatchPayload,
    dry_run: bool,
) -> Result<PublishResult> {
    let run = verify::verify_hnsw_dispatch(client, repo_full, payload).await?;
    let (owner, repo) = split_repo(repo_full)?;
    let repo_ref = RepoRef::new(owner, repo);
    let branch = DATA_PR_BRANCH.to_string();
    let head_sha = run
        .head_sha
        .as_deref()
        .ok_or_else(|| bot_err("verified run missing head_sha".to_string()))?;

    ensure_not_replay(client, repo_ref, payload, head_sha).await?;

    if let Some(existing) = client.find_open_pr(repo_ref, DATA_PR_BRANCH).await? {
        if ranking_quality_head_already_on_branch(client, repo_ref, head_sha).await? {
            return Ok(PublishResult {
                skipped: true,
                pr_url: existing
                    .get("html_url")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned),
                branch,
            });
        }
    }

    let zip = client
        .download_ranking_quality_artifact_zip(repo_ref, payload.run_id)
        .await?;
    let extracted = extract_ranking_quality_artifact_zip(&zip)?;
    validate_ranking_quality_file(&extracted.json_path)?;

    let json_bytes = fs::read(&extracted.json_path)
        .map_err(|err| bot_err(format!("read ranking-quality benchmark json: {err}")))?;
    let benchmark_data: RankingQualityBenchmarkData = serde_json::from_slice(&json_bytes)
        .map_err(|err| bot_err(format!("decode ranking-quality benchmark json: {err}")))?;
    cross_check_ranking_quality_artifact_run(
        &benchmark_data,
        &run,
        payload.run_id,
        payload.run_number,
    )?;

    if dry_run {
        return Ok(PublishResult {
            skipped: false,
            pr_url: None,
            branch,
        });
    }

    let main_sha = client.main_sha(repo_ref).await?;
    if !client.ref_exists(repo_ref, DATA_PR_BRANCH).await? {
        client
            .create_branch(CreateBranch {
                repo: repo_ref,
                branch: DATA_PR_BRANCH,
                sha: &main_sha,
            })
            .await?;
    }

    let subject = format!(
        "chore(bench): ranking-quality benchmark data update (run #{})",
        payload.run_number
    );
    let message = bot_commit_message(&subject);
    client
        .commit_files(
            repo_ref,
            CommitFilesRequest {
                branch: DATA_PR_BRANCH,
                message: &message,
                files: &[CommitFile {
                    path: RANKING_QUALITY_BENCHMARK_DATA_PATH,
                    bytes: &json_bytes,
                }],
            },
        )
        .await?;

    let pr = upsert_combined_data_pr(UpsertDataPr {
        client,
        repo: repo_ref,
        proxy_run_url: None,
        proxy_run_number: None,
        hnsw_run_url: None,
        hnsw_run_number: None,
        ranking_run_url: run.html_url.as_deref(),
        ranking_run_number: Some(payload.run_number),
        write_run_url: None,
        write_run_number: None,
        extraction_run_url: None,
        extraction_run_number: None,
    })
    .await?;

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
            path: RANKING_QUALITY_BENCHMARK_DATA_PATH,
            git_ref: "main",
        })
        .await?;
    let Some(bytes) = published else {
        return Ok(());
    };
    let data: RankingQualityBenchmarkData = serde_json::from_slice(&bytes)
        .map_err(|err| bot_err(format!("decode published ranking-quality data: {err}")))?;
    if ranking_quality_published_sha_exists(&data, head_sha) {
        return Err(bot_err("head_sha already published on main".to_string()));
    }
    if let Some(max_run) = ranking_quality_max_published_run_number(&data) {
        if payload.run_number <= max_run {
            return Err(bot_err(format!(
                "run_number {} is not newer than published max {}",
                payload.run_number, max_run
            )));
        }
    }
    Ok(())
}
