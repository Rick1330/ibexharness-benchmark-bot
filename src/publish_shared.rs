//! Shared upsert for proxy + HNSW benchmark data into one harness PR.

use serde_json::Value;

use crate::config::{
    BENCHMARK_DATA_LABEL, BENCHMARK_DATA_PATH, DATA_PR_BRANCH, HNSW_BENCHMARK_DATA_LABEL,
    HNSW_BENCHMARK_DATA_PATH,
};
use crate::error::{bot_err, Result};
use crate::github::{
    GitHubClient, IssueRef, OpenPullRequest, RepoPathRef, RepoRef, UpdatePullRequest,
};
use crate::hnsw_validate::hnsw_published_sha_exists;
use crate::model::{BenchmarkData, HnswBenchmarkData};
use crate::render::{render_combined_data_pr_body, CombinedDataPrInput};
use crate::validate::published_sha_exists;

pub fn data_pr_title(has_proxy: bool, has_hnsw: bool) -> String {
    match (has_proxy, has_hnsw) {
        (true, true) => "chore(bench): publish proxy and memory benchmark data".to_string(),
        (true, false) => "chore(bench): publish proxy benchmark data".to_string(),
        (false, true) => "chore(bench): publish memory HNSW benchmark data".to_string(),
        (false, false) => "chore(bench): publish benchmark data".to_string(),
    }
}

pub async fn load_proxy_from_ref(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    git_ref: &str,
) -> Result<Option<BenchmarkData>> {
    let Some(bytes) = client
        .get_file_bytes(RepoPathRef {
            repo,
            path: BENCHMARK_DATA_PATH,
            git_ref,
        })
        .await?
    else {
        return Ok(None);
    };
    let data: BenchmarkData = serde_json::from_slice(&bytes)
        .map_err(|err| bot_err(format!("decode {BENCHMARK_DATA_PATH}@{git_ref}: {err}")))?;
    Ok(Some(data))
}

pub async fn load_hnsw_from_ref(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    git_ref: &str,
) -> Result<Option<HnswBenchmarkData>> {
    let Some(bytes) = client
        .get_file_bytes(RepoPathRef {
            repo,
            path: HNSW_BENCHMARK_DATA_PATH,
            git_ref,
        })
        .await?
    else {
        return Ok(None);
    };
    let data: HnswBenchmarkData = serde_json::from_slice(&bytes).map_err(|err| {
        bot_err(format!(
            "decode {HNSW_BENCHMARK_DATA_PATH}@{git_ref}: {err}"
        ))
    })?;
    Ok(Some(data))
}

pub async fn proxy_head_already_on_branch(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    head_sha: &str,
) -> Result<bool> {
    let Some(data) = load_proxy_from_ref(client, repo, DATA_PR_BRANCH).await? else {
        return Ok(false);
    };
    Ok(published_sha_exists(&data, head_sha))
}

pub async fn hnsw_head_already_on_branch(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    head_sha: &str,
) -> Result<bool> {
    let Some(data) = load_hnsw_from_ref(client, repo, DATA_PR_BRANCH).await? else {
        return Ok(false);
    };
    Ok(hnsw_published_sha_exists(&data, head_sha))
}

/// Suites whose branch tip differs from `main` (pending publish).
pub async fn pending_suites(
    client: &GitHubClient,
    repo: RepoRef<'_>,
) -> Result<(Option<BenchmarkData>, Option<HnswBenchmarkData>)> {
    let proxy = match (
        load_proxy_from_ref(client, repo, DATA_PR_BRANCH).await?,
        load_proxy_from_ref(client, repo, "main").await?,
    ) {
        (Some(branch), Some(main)) if branch_bytes_differ(&branch, &main) => Some(branch),
        (Some(branch), None) => Some(branch),
        _ => None,
    };
    let hnsw = match (
        load_hnsw_from_ref(client, repo, DATA_PR_BRANCH).await?,
        load_hnsw_from_ref(client, repo, "main").await?,
    ) {
        (Some(branch), Some(main)) if hnsw_bytes_differ(&branch, &main) => Some(branch),
        (Some(branch), None) => Some(branch),
        _ => None,
    };
    Ok((proxy, hnsw))
}

fn branch_bytes_differ(left: &BenchmarkData, right: &BenchmarkData) -> bool {
    serde_json::to_vec(left).ok() != serde_json::to_vec(right).ok()
}

fn hnsw_bytes_differ(left: &HnswBenchmarkData, right: &HnswBenchmarkData) -> bool {
    serde_json::to_vec(left).ok() != serde_json::to_vec(right).ok()
}

pub struct UpsertDataPr<'a> {
    pub client: &'a GitHubClient,
    pub repo: RepoRef<'a>,
    pub proxy_run_url: Option<&'a str>,
    pub proxy_run_number: Option<i64>,
    pub hnsw_run_url: Option<&'a str>,
    pub hnsw_run_number: Option<i64>,
}

/// Refresh title/body from pending suite files on the shared branch; open or update the PR.
pub async fn upsert_combined_data_pr(args: UpsertDataPr<'_>) -> Result<Value> {
    let branch = DATA_PR_BRANCH;
    let (proxy, hnsw) = pending_suites(args.client, args.repo).await?;
    let has_proxy = proxy.is_some();
    let has_hnsw = hnsw.is_some();
    if !has_proxy && !has_hnsw {
        return Err(bot_err(
            "no pending benchmark suite files on shared data branch".to_string(),
        ));
    }
    let title = data_pr_title(has_proxy, has_hnsw);
    let body = render_combined_data_pr_body(CombinedDataPrInput {
        proxy: proxy.as_ref(),
        proxy_run_url: args.proxy_run_url,
        proxy_run_number: args.proxy_run_number,
        hnsw: hnsw.as_ref(),
        hnsw_run_url: args.hnsw_run_url,
        hnsw_run_number: args.hnsw_run_number,
    });
    let labels = suite_labels(has_proxy, has_hnsw);

    if let Some(existing) = args.client.find_open_pr(args.repo, branch).await? {
        let number = existing
            .get("number")
            .and_then(Value::as_i64)
            .ok_or_else(|| bot_err("open data PR missing number".to_string()))?;
        let pr = args
            .client
            .update_pull_request(UpdatePullRequest {
                repo: args.repo,
                number,
                title: &title,
                body: &body,
            })
            .await?;
        args.client
            .add_labels(
                IssueRef {
                    repo: args.repo,
                    number,
                },
                &labels,
            )
            .await?;
        return Ok(pr);
    }

    let pr = args
        .client
        .open_pull_request(OpenPullRequest {
            repo: args.repo,
            branch,
            title: &title,
            body: &body,
        })
        .await?;
    if let Some(number) = pr.get("number").and_then(Value::as_i64) {
        args.client
            .add_labels(
                IssueRef {
                    repo: args.repo,
                    number,
                },
                &labels,
            )
            .await?;
    }
    Ok(pr)
}

pub fn suite_labels(has_proxy: bool, has_hnsw: bool) -> Vec<&'static str> {
    let mut labels = vec!["automated"];
    if has_proxy {
        labels.push(BENCHMARK_DATA_LABEL);
    }
    if has_hnsw {
        labels.push(HNSW_BENCHMARK_DATA_LABEL);
    }
    labels
}
