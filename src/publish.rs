use std::fs;

use serde_json::Value;

use crate::artifact::{extract_artifact_zip, validate_badge_svg};
use crate::config::{BADGE_PATH, BENCHMARK_DATA_LABEL, BENCHMARK_DATA_PATH};
use crate::error::{bot_err, Result};
use crate::github::{
    split_repo, CreateBranch, GitHubClient, IssueRef, LabeledPrSearch, OpenPullRequest,
    PutFileRequest, RepoPathRef, RepoRef,
};
use crate::model::{BenchmarkData, DispatchPayload};
use crate::render::render_data_pr_body;
use crate::validate::{
    cross_check_artifact_run, max_published_run_number, published_sha_exists, validate_file,
    validate_payload,
};
use crate::verify;

pub struct PublishResult {
    pub skipped: bool,
    pub pr_url: Option<String>,
    pub branch: String,
}

pub async fn publish_benchmark_data(
    client: &GitHubClient,
    repo_full: &str,
    payload: &DispatchPayload,
    dry_run: bool,
) -> Result<PublishResult> {
    let run = verify::verify_dispatch(client, repo_full, payload).await?;
    let (owner, repo) = split_repo(repo_full)?;
    let repo_ref = RepoRef::new(owner, repo);
    let branch = format!("chore/bench-data-{}", payload.run_number);
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
        .download_artifact_zip(repo_ref, payload.run_id)
        .await?;
    let extracted = extract_artifact_zip(&zip)?;
    validate_file(&extracted.json_path)?;
    let badge_bytes =
        fs::read(&extracted.badge_path).map_err(|err| bot_err(format!("read badge: {err}")))?;
    validate_badge_svg(&badge_bytes)?;

    let json_bytes = fs::read(&extracted.json_path)
        .map_err(|err| bot_err(format!("read benchmark json: {err}")))?;
    let benchmark_data: BenchmarkData = serde_json::from_slice(&json_bytes)
        .map_err(|err| bot_err(format!("decode benchmark json: {err}")))?;
    validate_payload(&benchmark_data)?;
    cross_check_artifact_run(&benchmark_data, &run, payload.run_id, payload.run_number)?;

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

    let message = format!(
        "chore(bench): benchmark data update (run #{})",
        payload.run_number
    );
    let json_path = RepoPathRef {
        repo: repo_ref,
        path: BENCHMARK_DATA_PATH,
        git_ref: &branch,
    };
    let badge_path = RepoPathRef {
        repo: repo_ref,
        path: BADGE_PATH,
        git_ref: &branch,
    };
    let json_sha = client.file_sha(json_path).await?;
    let badge_sha = client.file_sha(badge_path).await?;
    client
        .put_file(
            repo_ref,
            PutFileRequest {
                path: BENCHMARK_DATA_PATH,
                branch: &branch,
                bytes: &json_bytes,
                message: &message,
                file_sha: json_sha.as_deref(),
            },
        )
        .await?;
    client
        .put_file(
            repo_ref,
            PutFileRequest {
                path: BADGE_PATH,
                branch: &branch,
                bytes: &badge_bytes,
                message: &message,
                file_sha: badge_sha.as_deref(),
            },
        )
        .await?;

    let body = render_data_pr_body(
        &benchmark_data,
        run.html_url.as_deref(),
        Some(payload.run_number),
    );
    let title = format!(
        "chore(bench): benchmark data update (run #{})",
        payload.run_number
    );
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
                &["automated", BENCHMARK_DATA_LABEL],
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
            path: BENCHMARK_DATA_PATH,
            git_ref: "main",
        })
        .await?;
    let Some(bytes) = published else {
        return Ok(());
    };
    let data: BenchmarkData = serde_json::from_slice(&bytes)
        .map_err(|err| bot_err(format!("decode published benchmark data: {err}")))?;
    if published_sha_exists(&data, head_sha) {
        return Err(bot_err("head_sha already published on main".to_string()));
    }
    if let Some(max_run) = max_published_run_number(&data) {
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
            label: BENCHMARK_DATA_LABEL,
            head_sha,
        })
        .await
}
