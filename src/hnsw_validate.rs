//! Validate published HNSW / memory benchmark JSON (independent of proxy schema).

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use regex::Regex;

use crate::error::{bot_err, Result};
use crate::model::{HnswBenchmarkData, HnswBenchmarkRun, WorkflowRun};

const MAX_RUNS: usize = 50;
const MAX_JSON_BYTES: usize = 2 * 1024 * 1024;
const MAX_RUN_NUMBER: i64 = 1_000_000;

pub fn validate_hnsw_file(path: &Path) -> Result<()> {
    let bytes = fs::read(path).map_err(|err| bot_err(format!("read failed: {err}")))?;
    if bytes.len() > MAX_JSON_BYTES {
        return Err(bot_err(format!("json exceeds {MAX_JSON_BYTES} bytes")));
    }
    let payload: HnswBenchmarkData = serde_json::from_slice(&bytes)
        .map_err(|err| bot_err(format!("hnsw json decode failed: {err}")))?;
    validate_hnsw_payload(&payload)
}

pub fn validate_hnsw_payload(payload: &HnswBenchmarkData) -> Result<()> {
    if payload.schema_version != Some(1) {
        return Err(bot_err("hnsw schema_version must be 1".to_string()));
    }
    if payload.benchmark.as_deref() != Some("hnsw_recall_latency") {
        return Err(bot_err(
            "hnsw benchmark must be hnsw_recall_latency".to_string(),
        ));
    }
    let runs = payload
        .runs
        .as_ref()
        .ok_or_else(|| bot_err("hnsw runs must be an array".to_string()))?;
    if runs.is_empty() {
        return Err(bot_err("hnsw runs must not be empty".to_string()));
    }
    if runs.len() > MAX_RUNS {
        return Err(bot_err(format!("hnsw runs exceeds max {MAX_RUNS}")));
    }
    let mut seen_sha = HashSet::new();
    for (index, run) in runs.iter().enumerate() {
        validate_hnsw_run(run, index)?;
        if let Some(sha) = run.sha.as_deref() {
            if !seen_sha.insert(sha.to_string()) {
                return Err(bot_err(format!("duplicate hnsw sha: {sha}")));
            }
        }
    }
    Ok(())
}

fn validate_hnsw_run(run: &HnswBenchmarkRun, index: usize) -> Result<()> {
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
    let mean = run
        .mean_recall_at_10
        .ok_or_else(|| bot_err(format!("runs[{index}].mean_recall_at_10 required")))?;
    if !(0.0..=1.0).contains(&mean) {
        return Err(bot_err(format!(
            "runs[{index}].mean_recall_at_10 out of range"
        )));
    }
    let results = run
        .results
        .as_ref()
        .ok_or_else(|| bot_err(format!("runs[{index}].results required")))?;
    if results.is_empty() {
        return Err(bot_err(format!("runs[{index}].results empty")));
    }
    for (ri, result) in results.iter().enumerate() {
        let recall = result
            .recall_at_10
            .ok_or_else(|| bot_err(format!("runs[{index}].results[{ri}].recall_at_10")))?;
        if !(0.0..=1.0).contains(&recall) {
            return Err(bot_err(format!(
                "runs[{index}].results[{ri}].recall_at_10 out of range"
            )));
        }
        for (name, value) in [
            ("latency_ms_p50", result.latency_ms_p50),
            ("latency_ms_p95", result.latency_ms_p95),
            ("latency_ms_p99", result.latency_ms_p99),
        ] {
            let v = value
                .ok_or_else(|| bot_err(format!("runs[{index}].results[{ri}].{name} required")))?;
            if v < 0.0 || !v.is_finite() {
                return Err(bot_err(format!(
                    "runs[{index}].results[{ri}].{name} invalid"
                )));
            }
        }
        let corpus = result
            .corpus_size
            .ok_or_else(|| bot_err(format!("runs[{index}].results[{ri}].corpus_size")))?;
        if corpus < 1 {
            return Err(bot_err(format!(
                "runs[{index}].results[{ri}].corpus_size invalid"
            )));
        }
    }
    Ok(())
}

pub fn cross_check_hnsw_artifact_run(
    payload: &HnswBenchmarkData,
    workflow: &WorkflowRun,
    run_id: i64,
    expected_run_number: i64,
) -> Result<()> {
    let latest = payload
        .runs
        .as_ref()
        .and_then(|runs| runs.first())
        .ok_or_else(|| bot_err("hnsw runs must contain latest entry".to_string()))?;

    let head_sha = workflow
        .head_sha
        .as_deref()
        .map(str::to_lowercase)
        .ok_or_else(|| bot_err("workflow head_sha missing".to_string()))?;

    let run_sha = latest
        .sha
        .as_deref()
        .ok_or_else(|| bot_err("runs[0].sha required".to_string()))?
        .to_lowercase();
    if run_sha != head_sha {
        return Err(bot_err(
            "hnsw runs[0].sha mismatch with verified workflow head_sha".to_string(),
        ));
    }

    if latest.run_number != Some(expected_run_number) {
        return Err(bot_err(
            "hnsw runs[0].run_number mismatch with dispatch payload".to_string(),
        ));
    }

    let marker = format!("/actions/runs/{run_id}");
    let run_url = latest
        .run_url
        .as_deref()
        .ok_or_else(|| bot_err("runs[0].run_url required".to_string()))?;
    if !run_url.contains(&marker) {
        return Err(bot_err(
            "hnsw runs[0].run_url must reference dispatch run_id".to_string(),
        ));
    }
    Ok(())
}

pub fn hnsw_published_sha_exists(data: &HnswBenchmarkData, head_sha: &str) -> bool {
    let needle = head_sha.to_lowercase();
    data.runs.as_ref().into_iter().flatten().any(|run| {
        run.sha
            .as_deref()
            .is_some_and(|sha| sha.eq_ignore_ascii_case(&needle))
    })
}

pub fn hnsw_max_published_run_number(data: &HnswBenchmarkData) -> Option<i64> {
    data.runs
        .as_ref()?
        .iter()
        .filter_map(|run| run.run_number)
        .max()
}

fn require_sha_field(value: &str, field: &str) -> Result<()> {
    let cleaned = value.trim().to_lowercase();
    let re = Regex::new(r"^[0-9a-f]{7,40}$").expect("sha regex");
    if !re.is_match(&cleaned) {
        return Err(bot_err(format!("{field} must be hexadecimal")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HnswBenchmarkData, HnswSizeResult};

    fn sample_payload() -> HnswBenchmarkData {
        HnswBenchmarkData {
            schema_version: Some(1),
            benchmark: Some("hnsw_recall_latency".into()),
            runs: Some(vec![HnswBenchmarkRun {
                sha: Some("abcdef1".into()),
                short_sha: Some("abcdef1".into()),
                timestamp: Some("2026-08-26T00:00:00Z".into()),
                branch: Some("main".into()),
                run_number: Some(3),
                run_url: Some("https://github.com/Rick1330/ibex-harness/actions/runs/99".into()),
                methodology: None,
                mean_recall_at_10: Some(1.0),
                results: Some(vec![HnswSizeResult {
                    corpus_size: Some(10_000),
                    query_count: Some(50),
                    recall_at_10: Some(1.0),
                    latency_ms_p50: Some(1.0),
                    latency_ms_p95: Some(2.0),
                    latency_ms_p99: Some(3.0),
                    ef_search: Some(40),
                }]),
            }]),
        }
    }

    #[test]
    fn accepts_valid_payload() {
        validate_hnsw_payload(&sample_payload()).expect("valid");
    }

    #[test]
    fn rejects_wrong_benchmark_name() {
        let mut payload = sample_payload();
        payload.benchmark = Some("proxy".into());
        assert!(validate_hnsw_payload(&payload).is_err());
    }
}
