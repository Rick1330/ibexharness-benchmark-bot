use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use serde_json::Value;
use zip::ZipArchive;

use crate::error::{bot_err, Result};
use crate::github::{extract_artifact_paths, GitHubClient};
use crate::model::BenchmarkData;
use crate::render::render_data_pr_body;
use crate::validate;
use crate::verify;

const JSON_PATH: &str = "docs/app/public/benchmarks/benchmark-data.json";
const BADGE_PATH: &str = "docs/app/public/benchmarks/badge.svg";

pub struct PublishResult {
    pub skipped: bool,
    pub pr_url: Option<String>,
    pub branch: String,
}

pub async fn publish_benchmark_data(
    client: &GitHubClient,
    repo_full: &str,
    payload: &crate::model::DispatchPayload,
    dry_run: bool,
) -> Result<PublishResult> {
    let run = verify::verify_dispatch(client, repo_full, payload).await?;
    let (owner, repo) = crate::github::split_repo(repo_full)?;
    let branch = format!("chore/bench-data-{}", payload.run_number);

    if let Some(existing) = client.find_open_pr(owner, repo, &branch).await? {
        return Ok(PublishResult {
            skipped: true,
            pr_url: existing
                .get("html_url")
                .and_then(|value| value.as_str())
                .map(str::to_owned),
            branch,
        });
    }

    let zip = client.download_artifact_zip(owner, repo, payload.run_id).await?;
    let work_dir = extract_zip(&zip)?;
    let (json_path, badge_path) = extract_artifact_paths(&work_dir)?;
    validate::validate_file(&json_path)?;

    if dry_run {
        return Ok(PublishResult {
            skipped: false,
            pr_url: None,
            branch,
        });
    }

    let benchmark_data: BenchmarkData = serde_json::from_slice(
        &fs::read(&json_path).map_err(|err| bot_err(format!("read benchmark json: {err}")))?,
    )
    .map_err(|err| bot_err(format!("decode benchmark json: {err}")))?;

    let main_sha = client.main_sha(owner, repo).await?;
    if !client.ref_exists(owner, repo, &branch).await? {
        client.create_branch(owner, repo, &branch, &main_sha).await?;
    }

    let message = format!("chore(bench): benchmark data update (run #{})", payload.run_number);
    let json_sha = client.file_sha(owner, repo, JSON_PATH, &branch).await?;
    let badge_sha = client.file_sha(owner, repo, BADGE_PATH, &branch).await?;
    let json_bytes = fs::read(&json_path).map_err(|err| bot_err(format!("read json: {err}")))?;
    let badge_bytes = fs::read(&badge_path).map_err(|err| bot_err(format!("read badge: {err}")))?;
    client
        .put_file(
            owner,
            repo,
            JSON_PATH,
            &branch,
            &json_bytes,
            &message,
            json_sha.as_deref(),
        )
        .await?;
    client
        .put_file(
            owner,
            repo,
            BADGE_PATH,
            &branch,
            &badge_bytes,
            &message,
            badge_sha.as_deref(),
        )
        .await?;

    let body = render_data_pr_body(
        &benchmark_data,
        run.html_url.as_deref(),
        Some(payload.run_number),
    );
    let title = format!("chore(bench): benchmark data update (run #{})", payload.run_number);
    let pr = client.open_pull_request(owner, repo, &branch, &title, &body).await?;
    if let Some(number) = pr.get("number").and_then(Value::as_i64) {
        let _ = client
            .add_labels(owner, repo, number, &["automated", "benchmark-data"])
            .await;
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

fn extract_zip(bytes: &[u8]) -> Result<PathBuf> {
    let dir = tempfile::tempdir().map_err(|err| bot_err(format!("tempdir failed: {err}")))?;
    let path = dir
        .keep()
        .map_err(|err| bot_err(format!("tempdir keep failed: {err}")))?;
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|err| bot_err(format!("zip open failed: {err}")))?;
    archive
        .extract(&path)
        .map_err(|err| bot_err(format!("zip extract failed: {err}")))?;
    Ok(path)
}
