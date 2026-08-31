//! Validate published ranking-quality benchmark JSON.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::error::{bot_err, Result};
use crate::model::{RankingQualityBenchmarkData, RankingQualityBenchmarkRun, WorkflowRun};
use crate::verify::is_hex_sha;

const MAX_RUNS: usize = 50;
const MAX_JSON_BYTES: usize = 2 * 1024 * 1024;
const MAX_RUN_NUMBER: i64 = 1_000_000;

pub fn validate_ranking_quality_file(path: &Path) -> Result<()> {
    let bytes = fs::read(path).map_err(|err| bot_err(format!("read failed: {err}")))?;
    if bytes.len() > MAX_JSON_BYTES {
        return Err(bot_err(format!("json exceeds {MAX_JSON_BYTES} bytes")));
    }
    let payload: RankingQualityBenchmarkData = serde_json::from_slice(&bytes)
        .map_err(|err| bot_err(format!("ranking-quality json decode failed: {err}")))?;
    validate_ranking_quality_payload(&payload)
}

pub fn validate_ranking_quality_payload(payload: &RankingQualityBenchmarkData) -> Result<()> {
    if payload.schema_version != Some(1) {
        return Err(bot_err(
            "ranking-quality schema_version must be 1".to_string(),
        ));
    }
    if payload.benchmark.as_deref() != Some("ranking_quality") {
        return Err(bot_err(
            "ranking-quality benchmark must be ranking_quality".to_string(),
        ));
    }
    let runs = payload
        .runs
        .as_ref()
        .ok_or_else(|| bot_err("ranking-quality runs must be an array".to_string()))?;
    if runs.is_empty() {
        return Err(bot_err(
            "ranking-quality runs must not be empty".to_string(),
        ));
    }
    if runs.len() > MAX_RUNS {
        return Err(bot_err(format!(
            "ranking-quality runs exceeds max {MAX_RUNS}"
        )));
    }
    let mut seen_sha = HashSet::new();
    for (index, run) in runs.iter().enumerate() {
        validate_ranking_quality_run(run, index)?;
        if let Some(sha) = run.sha.as_deref() {
            if !seen_sha.insert(sha.to_string()) {
                return Err(bot_err(format!("duplicate ranking-quality sha: {sha}")));
            }
        }
    }
    Ok(())
}

fn validate_ranking_quality_run(run: &RankingQualityBenchmarkRun, index: usize) -> Result<()> {
    let sha = run
        .sha
        .as_deref()
        .ok_or_else(|| bot_err(format!("runs[{index}].sha required")))?;
    require_sha_field(sha, &format!("runs[{index}].sha"))?;
    let short = run
        .short_sha
        .as_deref()
        .ok_or_else(|| bot_err(format!("runs[{index}].short_sha required")))?;
    if short.len() < 7 {
        return Err(bot_err(format!("runs[{index}].short_sha too short")));
    }
    let run_number = run
        .run_number
        .ok_or_else(|| bot_err(format!("runs[{index}].run_number required")))?;
    if !(0..=MAX_RUN_NUMBER).contains(&run_number) {
        return Err(bot_err(format!("runs[{index}].run_number out of range")));
    }
    if let Some(status) = run.status.as_deref() {
        if !matches!(status, "pass" | "fail" | "warn") {
            return Err(bot_err(format!("runs[{index}].status invalid")));
        }
    }
    let metrics = run
        .metrics
        .as_ref()
        .ok_or_else(|| bot_err(format!("runs[{index}].metrics required")))?;
    for (name, value) in [
        ("precision_at_5", metrics.precision_at_5),
        ("recall_at_10", metrics.recall_at_10),
        ("mrr", metrics.mrr),
    ] {
        let v = value.ok_or_else(|| bot_err(format!("runs[{index}].metrics.{name} required")))?;
        if !(0.0..=1.0).contains(&v) {
            return Err(bot_err(format!(
                "runs[{index}].metrics.{name} out of range"
            )));
        }
    }
    Ok(())
}

pub fn cross_check_ranking_quality_artifact_run(
    payload: &RankingQualityBenchmarkData,
    workflow: &WorkflowRun,
    run_id: i64,
    expected_run_number: i64,
) -> Result<()> {
    cross_check_memory_suite_run(
        payload.runs.as_deref(),
        workflow,
        run_id,
        expected_run_number,
        "ranking-quality",
    )
}

pub fn ranking_quality_published_sha_exists(
    data: &RankingQualityBenchmarkData,
    head_sha: &str,
) -> bool {
    memory_suite_published_sha_exists(data.runs.as_deref(), head_sha)
}

pub fn ranking_quality_max_published_run_number(data: &RankingQualityBenchmarkData) -> Option<i64> {
    memory_suite_max_published_run_number(data.runs.as_deref())
}

pub(crate) fn cross_check_memory_suite_run<T>(
    runs: Option<&[T]>,
    workflow: &WorkflowRun,
    run_id: i64,
    expected_run_number: i64,
    suite: &str,
) -> Result<()>
where
    T: MemorySuiteRunFields,
{
    let runs = runs.ok_or_else(|| bot_err(format!("{suite} runs must be an array")))?;
    let latest = runs
        .first()
        .ok_or_else(|| bot_err(format!("{suite} runs must contain latest entry")))?;

    let head_sha = workflow
        .head_sha
        .as_deref()
        .map(str::to_lowercase)
        .ok_or_else(|| bot_err("workflow head_sha missing".to_string()))?;

    let run_sha = latest
        .sha()
        .ok_or_else(|| bot_err("runs[0].sha required".to_string()))?
        .to_lowercase();
    if run_sha != head_sha {
        return Err(bot_err(format!(
            "{suite} runs[0].sha mismatch with verified workflow head_sha"
        )));
    }

    if latest.run_number() != Some(expected_run_number) {
        return Err(bot_err(format!(
            "{suite} runs[0].run_number mismatch with dispatch payload"
        )));
    }

    let run_url = latest
        .run_url()
        .ok_or_else(|| bot_err("runs[0].run_url required".to_string()))?;
    verify_dispatch_run_url(run_url, workflow, run_id, suite)?;

    let mut seen_sha = HashSet::new();
    for (index, run) in runs.iter().enumerate() {
        if let Some(run_number) = run.run_number() {
            if run_number > expected_run_number {
                return Err(bot_err(format!(
                    "{suite} runs[{index}].run_number exceeds verified dispatch run_number"
                )));
            }
        }
        if let Some(sha) = run.sha() {
            if !seen_sha.insert(sha.to_lowercase()) {
                return Err(bot_err(format!(
                    "{suite} runs[{index}].sha duplicates an earlier run"
                )));
            }
        }
    }
    Ok(())
}

fn verify_dispatch_run_url(
    run_url: &str,
    workflow: &WorkflowRun,
    run_id: i64,
    suite: &str,
) -> Result<()> {
    if let Some(expected) = workflow.html_url.as_deref() {
        if run_url.trim() != expected.trim() {
            return Err(bot_err(format!(
                "{suite} runs[0].run_url must match verified workflow html_url"
            )));
        }
        return Ok(());
    }

    let suffix = format!("/actions/runs/{run_id}");
    if !run_url.trim().ends_with(&suffix) {
        return Err(bot_err(format!(
            "{suite} runs[0].run_url must reference dispatch run_id"
        )));
    }
    Ok(())
}

pub(crate) fn memory_suite_published_sha_exists<T: MemorySuiteRunFields>(
    runs: Option<&[T]>,
    head_sha: &str,
) -> bool {
    let needle = head_sha.to_lowercase();
    runs.into_iter().flatten().any(|run| {
        run.sha()
            .is_some_and(|sha| sha.eq_ignore_ascii_case(&needle))
    })
}

pub(crate) fn memory_suite_max_published_run_number<T: MemorySuiteRunFields>(
    runs: Option<&[T]>,
) -> Option<i64> {
    runs?.iter().filter_map(|run| run.run_number()).max()
}

pub(crate) trait MemorySuiteRunFields {
    fn sha(&self) -> Option<&str>;
    fn run_number(&self) -> Option<i64>;
    fn run_url(&self) -> Option<&str>;
}

impl MemorySuiteRunFields for RankingQualityBenchmarkRun {
    fn sha(&self) -> Option<&str> {
        self.sha.as_deref()
    }
    fn run_number(&self) -> Option<i64> {
        self.run_number
    }
    fn run_url(&self) -> Option<&str> {
        self.run_url.as_deref()
    }
}

fn require_sha_field(value: &str, field: &str) -> Result<()> {
    let cleaned = value.trim().to_lowercase();
    if !is_hex_sha(&cleaned) {
        return Err(bot_err(format!("{field} must be hexadecimal")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RankingQualityBenchmarkMetrics, RankingQualityBenchmarkRun, WorkflowRun};

    fn verified_workflow() -> WorkflowRun {
        WorkflowRun {
            conclusion: Some("success".into()),
            head_branch: Some("main".into()),
            head_sha: Some("abcdef1".into()),
            run_number: Some(3),
            name: Some("Memory Benchmarks".into()),
            path: Some(".github/workflows/memory-benchmark.yml".into()),
            html_url: Some("https://github.com/Rick1330/ibex-harness/actions/runs/99".into()),
        }
    }

    fn sample_payload() -> RankingQualityBenchmarkData {
        RankingQualityBenchmarkData {
            schema_version: Some(1),
            benchmark: Some("ranking_quality".into()),
            runs: Some(vec![RankingQualityBenchmarkRun {
                sha: Some("abcdef1".into()),
                short_sha: Some("abcdef1".into()),
                timestamp: Some("2026-08-26T00:00:00Z".into()),
                branch: Some("main".into()),
                run_number: Some(3),
                run_url: Some("https://github.com/Rick1330/ibex-harness/actions/runs/99".into()),
                gold_set: Some("v1".into()),
                query_count: Some(10),
                memory_count: Some(20),
                metrics: Some(RankingQualityBenchmarkMetrics {
                    precision_at_5: Some(1.0),
                    recall_at_10: Some(1.0),
                    mrr: Some(1.0),
                    expected_order_match: Some(1.0),
                    top_category_accuracy: None,
                }),
                status: Some("pass".into()),
                gate_summary: None,
            }]),
        }
    }

    #[test]
    fn accepts_valid_payload() {
        validate_ranking_quality_payload(&sample_payload()).expect("valid");
    }

    #[test]
    fn rejects_wrong_benchmark_name() {
        let mut payload = sample_payload();
        payload.benchmark = Some("proxy".into());
        assert!(validate_ranking_quality_payload(&payload).is_err());
    }

    #[test]
    fn cross_check_rejects_future_run_number_in_history() {
        let mut payload = sample_payload();
        let template = payload.runs.as_ref().unwrap()[0].clone();
        payload
            .runs
            .as_mut()
            .unwrap()
            .push(RankingQualityBenchmarkRun {
                sha: Some("fedcba9".into()),
                short_sha: Some("fedcba9".into()),
                run_number: Some(999),
                ..template
            });
        let error = cross_check_ranking_quality_artifact_run(&payload, &verified_workflow(), 99, 3)
            .expect_err("future run_number must be rejected");
        assert!(error.to_string().contains("run_number exceeds"));
    }

    #[test]
    fn cross_check_rejects_duplicate_sha_in_history() {
        let mut payload = sample_payload();
        let duplicate = payload.runs.as_ref().unwrap()[0].clone();
        payload.runs.as_mut().unwrap().push(duplicate);
        let error = cross_check_ranking_quality_artifact_run(&payload, &verified_workflow(), 99, 3)
            .expect_err("duplicate sha must be rejected");
        assert!(error.to_string().contains("duplicates"));
    }

    #[test]
    fn cross_check_rejects_run_url_that_only_contains_marker() {
        let mut payload = sample_payload();
        payload.runs.as_mut().unwrap()[0].run_url =
            Some("https://evil.example/actions/runs/99/extra".into());
        let error = cross_check_ranking_quality_artifact_run(&payload, &verified_workflow(), 99, 3)
            .expect_err("run_url must match verified workflow html_url");
        assert!(error.to_string().contains("html_url"));
    }

    #[test]
    fn cross_check_accepts_matching_workflow_html_url() {
        cross_check_ranking_quality_artifact_run(&sample_payload(), &verified_workflow(), 99, 3)
            .expect("valid artifact cross-check");
    }
}
