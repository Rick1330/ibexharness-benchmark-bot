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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HnswBenchmarkRun, HnswSizeResult};
    use crate::render::render_hnsw_data_pr_body;

    fn data_with_results(results: Option<Vec<HnswSizeResult>>) -> HnswBenchmarkData {
        HnswBenchmarkData {
            schema_version: Some(1),
            benchmark: Some("hnsw_recall_latency".into()),
            runs: Some(vec![HnswBenchmarkRun {
                sha: Some("abcdef1".into()),
                short_sha: Some("abcdef1".into()),
                timestamp: None,
                branch: Some("main".into()),
                run_number: Some(42),
                run_url: None,
                methodology: None,
                mean_recall_at_10: Some(0.98765),
                results,
                ..Default::default()
            }]),
        }
    }

    #[test]
    fn renders_hnsw_pr_comment_with_separate_marker() {
        let data = data_with_results(Some(vec![HnswSizeResult {
            corpus_size: Some(10_000),
            recall_at_10: Some(1.0),
            latency_ms_p95: Some(20.0),
            ef_search: Some(40),
            min_similarity: Some(0.7),
            iterative_scan: Some("off".into()),
            index_build_mode: Some("bulk".into()),
            ..Default::default()
        }]));
        let mut data = data;
        if let Some(run) = data.runs.as_mut().and_then(|runs| runs.first_mut()) {
            run.status = Some("warn".into());
            run.gate_summary = Some(serde_json::json!({
                "recall_ok": true,
                "has_1m": false,
                "note": "1M cell absent (expected on smoke/fast profiles)"
            }));
        }
        let body = crate::render::render_hnsw_pr_comment(&data).expect("render");
        assert!(body.contains("<!-- IBEX_BOT_COMMENT_HNSW -->"));
        assert!(body.contains("### Memory HNSW suite"));
        assert!(body.contains("Memory HNSW"));
        assert!(body.contains("WARN"));
        assert!(body.contains("/benchmarks/memory"));
        assert!(body.contains("### Gate summary"));
    }

    #[test]
    fn renders_each_hnsw_cell_with_methodology_knobs() {
        let data = data_with_results(Some(vec![
            HnswSizeResult {
                corpus_size: Some(10_000),
                latency_ms_p95: Some(1.26),
                ef_search: Some(40),
                min_similarity: Some(0.7),
                iterative_scan: Some("off".into()),
                index_build_mode: Some("bulk".into()),
                ..Default::default()
            },
            HnswSizeResult {
                corpus_size: Some(1_000_000),
                latency_ms_p95: Some(12.04),
                ef_search: Some(80),
                min_similarity: Some(0.755),
                iterative_scan: Some("strict_order".into()),
                index_build_mode: Some("incremental".into()),
                ..Default::default()
            },
        ]));

        let body = render_hnsw_data_pr_body(&data, Some("https://example.test/runs/42"), 42);

        assert!(body.contains("## Automated HNSW benchmark data update"));
        assert!(body.contains("| Size | Recall@10 | p95 | ef | min_sim | iterative | build |"));
        assert!(body.contains("| 10000 |"));
        assert!(body.contains("1.3ms"));
        assert!(body.contains("| 1000000 |"));
        assert!(body.contains("12.0ms"));
        assert!(body.contains("strict_order"));
        assert!(body.contains("incremental"));
        assert!(body.contains("production-like knobs"));
        assert!(body.contains("https://example.test/runs/42"));
    }

    #[test]
    fn renders_explicit_fallbacks_for_missing_cell_data() {
        let data = data_with_results(Some(vec![HnswSizeResult::default()]));
        let body = render_hnsw_data_pr_body(&data, None, 7);

        assert!(body.contains("| — | — | — | — | — | — | — |"));

        let no_runs = HnswBenchmarkData {
            schema_version: Some(1),
            benchmark: Some("hnsw_recall_latency".into()),
            runs: None,
        };
        let body = render_hnsw_data_pr_body(&no_runs, None, 8);
        assert!(body.contains("_No result cells in latest run._"));
    }
}
