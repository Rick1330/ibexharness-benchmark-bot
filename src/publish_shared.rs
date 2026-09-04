//! Shared upsert for proxy + memory benchmark suites into one harness PR.

use serde_json::Value;

use crate::config::{
    BENCHMARK_DATA_LABEL, BENCHMARK_DATA_PATH, DATA_PR_BRANCH,
    EXTRACTION_QUALITY_BENCHMARK_DATA_LABEL, EXTRACTION_QUALITY_BENCHMARK_DATA_PATH,
    HNSW_BENCHMARK_DATA_LABEL, HNSW_BENCHMARK_DATA_PATH, RANKING_QUALITY_BENCHMARK_DATA_LABEL,
    RANKING_QUALITY_BENCHMARK_DATA_PATH, WRITE_PIPELINE_BENCHMARK_DATA_LABEL,
    WRITE_PIPELINE_BENCHMARK_DATA_PATH,
};
use crate::error::{bot_err, Result};
use crate::extraction_quality_validate::extraction_quality_published_sha_exists;
use crate::github::{
    GitHubClient, IssueRef, OpenPullRequest, RepoPathRef, RepoRef, UpdatePullRequest,
};
use crate::hnsw_validate::hnsw_published_sha_exists;
use crate::model::{
    BenchmarkData, ExtractionQualityBenchmarkData, HnswBenchmarkData, RankingQualityBenchmarkData,
    WritePipelineBenchmarkData,
};
use crate::ranking_quality_validate::ranking_quality_published_sha_exists;
use crate::render::{render_combined_data_pr_body, CombinedDataPrInput};
use crate::validate::published_sha_exists;
use crate::write_pipeline_validate::write_pipeline_published_sha_exists;

pub struct PendingSuites {
    pub proxy: Option<BenchmarkData>,
    pub hnsw: Option<HnswBenchmarkData>,
    pub ranking: Option<RankingQualityBenchmarkData>,
    pub write: Option<WritePipelineBenchmarkData>,
    pub extraction: Option<ExtractionQualityBenchmarkData>,
}

impl PendingSuites {
    pub fn is_empty(&self) -> bool {
        self.proxy.is_none()
            && self.hnsw.is_none()
            && self.ranking.is_none()
            && self.write.is_none()
            && self.extraction.is_none()
    }
}

pub fn data_pr_title(suites: &PendingSuites) -> String {
    let mut names = Vec::new();
    if suites.proxy.is_some() {
        names.push("proxy");
    }
    if suites.hnsw.is_some() {
        names.push("memory HNSW");
    }
    if suites.ranking.is_some() {
        names.push("ranking-quality");
    }
    if suites.write.is_some() {
        names.push("write-pipeline");
    }
    if suites.extraction.is_some() {
        names.push("extraction-quality");
    }
    if names.is_empty() {
        return "chore(bench): publish benchmark data".to_string();
    }
    format!(
        "chore(bench): publish {} benchmark data",
        names.join(" and ")
    )
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

pub async fn load_ranking_quality_from_ref(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    git_ref: &str,
) -> Result<Option<RankingQualityBenchmarkData>> {
    let Some(bytes) = client
        .get_file_bytes(RepoPathRef {
            repo,
            path: RANKING_QUALITY_BENCHMARK_DATA_PATH,
            git_ref,
        })
        .await?
    else {
        return Ok(None);
    };
    let data: RankingQualityBenchmarkData = serde_json::from_slice(&bytes).map_err(|err| {
        bot_err(format!(
            "decode {RANKING_QUALITY_BENCHMARK_DATA_PATH}@{git_ref}: {err}"
        ))
    })?;
    Ok(Some(data))
}

pub async fn load_write_pipeline_from_ref(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    git_ref: &str,
) -> Result<Option<WritePipelineBenchmarkData>> {
    let Some(bytes) = client
        .get_file_bytes(RepoPathRef {
            repo,
            path: WRITE_PIPELINE_BENCHMARK_DATA_PATH,
            git_ref,
        })
        .await?
    else {
        return Ok(None);
    };
    let data: WritePipelineBenchmarkData = serde_json::from_slice(&bytes).map_err(|err| {
        bot_err(format!(
            "decode {WRITE_PIPELINE_BENCHMARK_DATA_PATH}@{git_ref}: {err}"
        ))
    })?;
    Ok(Some(data))
}

pub async fn load_extraction_quality_from_ref(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    git_ref: &str,
) -> Result<Option<ExtractionQualityBenchmarkData>> {
    let Some(bytes) = client
        .get_file_bytes(RepoPathRef {
            repo,
            path: EXTRACTION_QUALITY_BENCHMARK_DATA_PATH,
            git_ref,
        })
        .await?
    else {
        return Ok(None);
    };
    let data: ExtractionQualityBenchmarkData = serde_json::from_slice(&bytes).map_err(|err| {
        bot_err(format!(
            "decode {EXTRACTION_QUALITY_BENCHMARK_DATA_PATH}@{git_ref}: {err}"
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

pub async fn ranking_quality_head_already_on_branch(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    head_sha: &str,
) -> Result<bool> {
    let Some(data) = load_ranking_quality_from_ref(client, repo, DATA_PR_BRANCH).await? else {
        return Ok(false);
    };
    Ok(ranking_quality_published_sha_exists(&data, head_sha))
}

pub async fn write_pipeline_head_already_on_branch(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    head_sha: &str,
) -> Result<bool> {
    let Some(data) = load_write_pipeline_from_ref(client, repo, DATA_PR_BRANCH).await? else {
        return Ok(false);
    };
    Ok(write_pipeline_published_sha_exists(&data, head_sha))
}

pub async fn extraction_quality_head_already_on_branch(
    client: &GitHubClient,
    repo: RepoRef<'_>,
    head_sha: &str,
) -> Result<bool> {
    let Some(data) = load_extraction_quality_from_ref(client, repo, DATA_PR_BRANCH).await? else {
        return Ok(false);
    };
    Ok(extraction_quality_published_sha_exists(&data, head_sha))
}

/// Suites whose branch tip differs from `main` (pending publish).
pub async fn pending_suites(client: &GitHubClient, repo: RepoRef<'_>) -> Result<PendingSuites> {
    let proxy = diff_proxy(client, repo).await?;
    let hnsw = diff_hnsw(client, repo).await?;
    let ranking = diff_ranking(client, repo).await?;
    let write = diff_write(client, repo).await?;
    let extraction = diff_extraction(client, repo).await?;
    Ok(PendingSuites {
        proxy,
        hnsw,
        ranking,
        write,
        extraction,
    })
}

async fn diff_proxy(client: &GitHubClient, repo: RepoRef<'_>) -> Result<Option<BenchmarkData>> {
    match (
        load_proxy_from_ref(client, repo, DATA_PR_BRANCH).await?,
        load_proxy_from_ref(client, repo, "main").await?,
    ) {
        (Some(branch), Some(main)) if branch_bytes_differ(&branch, &main) => Ok(Some(branch)),
        (Some(branch), None) => Ok(Some(branch)),
        _ => Ok(None),
    }
}

async fn diff_hnsw(client: &GitHubClient, repo: RepoRef<'_>) -> Result<Option<HnswBenchmarkData>> {
    match (
        load_hnsw_from_ref(client, repo, DATA_PR_BRANCH).await?,
        load_hnsw_from_ref(client, repo, "main").await?,
    ) {
        (Some(branch), Some(main)) if hnsw_bytes_differ(&branch, &main) => Ok(Some(branch)),
        (Some(branch), None) => Ok(Some(branch)),
        _ => Ok(None),
    }
}

async fn diff_ranking(
    client: &GitHubClient,
    repo: RepoRef<'_>,
) -> Result<Option<RankingQualityBenchmarkData>> {
    match (
        load_ranking_quality_from_ref(client, repo, DATA_PR_BRANCH).await?,
        load_ranking_quality_from_ref(client, repo, "main").await?,
    ) {
        (Some(branch), Some(main)) if ranking_bytes_differ(&branch, &main) => Ok(Some(branch)),
        (Some(branch), None) => Ok(Some(branch)),
        _ => Ok(None),
    }
}

async fn diff_write(
    client: &GitHubClient,
    repo: RepoRef<'_>,
) -> Result<Option<WritePipelineBenchmarkData>> {
    match (
        load_write_pipeline_from_ref(client, repo, DATA_PR_BRANCH).await?,
        load_write_pipeline_from_ref(client, repo, "main").await?,
    ) {
        (Some(branch), Some(main)) if write_bytes_differ(&branch, &main) => Ok(Some(branch)),
        (Some(branch), None) => Ok(Some(branch)),
        _ => Ok(None),
    }
}

async fn diff_extraction(
    client: &GitHubClient,
    repo: RepoRef<'_>,
) -> Result<Option<ExtractionQualityBenchmarkData>> {
    match (
        load_extraction_quality_from_ref(client, repo, DATA_PR_BRANCH).await?,
        load_extraction_quality_from_ref(client, repo, "main").await?,
    ) {
        (Some(branch), Some(main)) if extraction_bytes_differ(&branch, &main) => Ok(Some(branch)),
        (Some(branch), None) => Ok(Some(branch)),
        _ => Ok(None),
    }
}

fn branch_bytes_differ(left: &BenchmarkData, right: &BenchmarkData) -> bool {
    serde_json::to_vec(left).ok() != serde_json::to_vec(right).ok()
}

fn hnsw_bytes_differ(left: &HnswBenchmarkData, right: &HnswBenchmarkData) -> bool {
    serde_json::to_vec(left).ok() != serde_json::to_vec(right).ok()
}

fn ranking_bytes_differ(
    left: &RankingQualityBenchmarkData,
    right: &RankingQualityBenchmarkData,
) -> bool {
    serde_json::to_vec(left).ok() != serde_json::to_vec(right).ok()
}

fn write_bytes_differ(
    left: &WritePipelineBenchmarkData,
    right: &WritePipelineBenchmarkData,
) -> bool {
    serde_json::to_vec(left).ok() != serde_json::to_vec(right).ok()
}

fn extraction_bytes_differ(
    left: &ExtractionQualityBenchmarkData,
    right: &ExtractionQualityBenchmarkData,
) -> bool {
    serde_json::to_vec(left).ok() != serde_json::to_vec(right).ok()
}

pub struct UpsertDataPr<'a> {
    pub client: &'a GitHubClient,
    pub repo: RepoRef<'a>,
    pub proxy_run_url: Option<&'a str>,
    pub proxy_run_number: Option<i64>,
    pub hnsw_run_url: Option<&'a str>,
    pub hnsw_run_number: Option<i64>,
    pub ranking_run_url: Option<&'a str>,
    pub ranking_run_number: Option<i64>,
    pub write_run_url: Option<&'a str>,
    pub write_run_number: Option<i64>,
    pub extraction_run_url: Option<&'a str>,
    pub extraction_run_number: Option<i64>,
}

/// Refresh title/body from pending suite files on the shared branch; open or update the PR.
pub async fn upsert_combined_data_pr(args: UpsertDataPr<'_>) -> Result<Value> {
    let branch = DATA_PR_BRANCH;
    let pending = pending_suites(args.client, args.repo).await?;
    if pending.is_empty() {
        return Err(bot_err(
            "no pending benchmark suite files on shared data branch".to_string(),
        ));
    }
    let title = data_pr_title(&pending);
    let body = render_combined_data_pr_body(CombinedDataPrInput {
        proxy: pending.proxy.as_ref(),
        proxy_run_url: args.proxy_run_url,
        proxy_run_number: args.proxy_run_number,
        hnsw: pending.hnsw.as_ref(),
        hnsw_run_url: args.hnsw_run_url,
        hnsw_run_number: args.hnsw_run_number,
        ranking: pending.ranking.as_ref(),
        ranking_run_url: args.ranking_run_url,
        ranking_run_number: args.ranking_run_number,
        write: pending.write.as_ref(),
        write_run_url: args.write_run_url,
        write_run_number: args.write_run_number,
        extraction: pending.extraction.as_ref(),
        extraction_run_url: args.extraction_run_url,
        extraction_run_number: args.extraction_run_number,
    });
    let labels = suite_labels(&pending);

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

pub fn suite_labels(pending: &PendingSuites) -> Vec<&'static str> {
    let mut labels = vec!["automated"];
    if pending.proxy.is_some() {
        labels.push(BENCHMARK_DATA_LABEL);
    }
    if pending.hnsw.is_some() {
        labels.push(HNSW_BENCHMARK_DATA_LABEL);
    }
    if pending.ranking.is_some() {
        labels.push(RANKING_QUALITY_BENCHMARK_DATA_LABEL);
    }
    if pending.write.is_some() {
        labels.push(WRITE_PIPELINE_BENCHMARK_DATA_LABEL);
    }
    if pending.extraction.is_some() {
        labels.push(EXTRACTION_QUALITY_BENCHMARK_DATA_LABEL);
    }
    labels
}
