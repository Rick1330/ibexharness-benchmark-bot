//! Validate published write-pipeline benchmark JSON.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::error::{bot_err, Result};
use crate::model::{WorkflowRun, WritePipelineBenchmarkData, WritePipelineBenchmarkRun};
use crate::ranking_quality_validate::{
    cross_check_memory_suite_run, memory_suite_max_published_run_number,
    memory_suite_published_sha_exists, MemorySuiteRunFields,
};
use crate::verify::is_hex_sha;

const MAX_RUNS: usize = 50;
const MAX_JSON_BYTES: usize = 2 * 1024 * 1024;
const MAX_RUN_NUMBER: i64 = 1_000_000;
const WRITE_P95_SLA_MS: f64 = 200.0;

pub fn validate_write_pipeline_file(path: &Path) -> Result<()> {
    let bytes = fs::read(path).map_err(|err| bot_err(format!("read failed: {err}")))?;
    if bytes.len() > MAX_JSON_BYTES {
        return Err(bot_err(format!("json exceeds {MAX_JSON_BYTES} bytes")));
    }
    let payload: WritePipelineBenchmarkData = serde_json::from_slice(&bytes)
        .map_err(|err| bot_err(format!("write-pipeline json decode failed: {err}")))?;
    validate_write_pipeline_payload(&payload)
}

pub fn validate_write_pipeline_payload(payload: &WritePipelineBenchmarkData) -> Result<()> {
    if payload.schema_version != Some(1) {
        return Err(bot_err(
            "write-pipeline schema_version must be 1".to_string(),
        ));
    }
    if payload.benchmark.as_deref() != Some("write_pipeline") {
        return Err(bot_err(
            "write-pipeline benchmark must be write_pipeline".to_string(),
        ));
    }
    let runs = payload
        .runs
        .as_ref()
        .ok_or_else(|| bot_err("write-pipeline runs must be an array".to_string()))?;
    if runs.is_empty() {
        return Err(bot_err("write-pipeline runs must not be empty".to_string()));
    }
    if runs.len() > MAX_RUNS {
        return Err(bot_err(format!(
            "write-pipeline runs exceeds max {MAX_RUNS}"
        )));
    }
    let mut seen_sha = HashSet::new();
    for (index, run) in runs.iter().enumerate() {
        validate_write_pipeline_run(run, index)?;
        if let Some(sha) = run.sha.as_deref() {
            if !seen_sha.insert(sha.to_string()) {
                return Err(bot_err(format!("duplicate write-pipeline sha: {sha}")));
            }
        }
    }
    Ok(())
}

fn validate_write_pipeline_run(run: &WritePipelineBenchmarkRun, index: usize) -> Result<()> {
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
        ("latency_ms_p50", metrics.latency_ms_p50),
        ("latency_ms_p95", metrics.latency_ms_p95),
        ("latency_ms_p99", metrics.latency_ms_p99),
    ] {
        let v = value.ok_or_else(|| bot_err(format!("runs[{index}].metrics.{name} required")))?;
        if v < 0.0 || !v.is_finite() {
            return Err(bot_err(format!("runs[{index}].metrics.{name} invalid")));
        }
    }
    if let Some(p95) = metrics.latency_ms_p95 {
        if p95 > WRITE_P95_SLA_MS * 10.0 {
            return Err(bot_err(format!(
                "runs[{index}].metrics.latency_ms_p95 implausibly high"
            )));
        }
    }
    Ok(())
}

pub fn cross_check_write_pipeline_artifact_run(
    payload: &WritePipelineBenchmarkData,
    workflow: &WorkflowRun,
    run_id: i64,
    expected_run_number: i64,
) -> Result<()> {
    cross_check_memory_suite_run(
        payload.runs.as_ref().and_then(|runs| runs.first()),
        workflow,
        run_id,
        expected_run_number,
        "write-pipeline",
    )
}

pub fn write_pipeline_published_sha_exists(
    data: &WritePipelineBenchmarkData,
    head_sha: &str,
) -> bool {
    memory_suite_published_sha_exists(data.runs.as_deref(), head_sha)
}

pub fn write_pipeline_max_published_run_number(data: &WritePipelineBenchmarkData) -> Option<i64> {
    memory_suite_max_published_run_number(data.runs.as_deref())
}

impl MemorySuiteRunFields for WritePipelineBenchmarkRun {
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
    use crate::model::{WritePipelineBenchmarkMetrics, WritePipelineBenchmarkRun};

    fn sample_payload() -> WritePipelineBenchmarkData {
        WritePipelineBenchmarkData {
            schema_version: Some(1),
            benchmark: Some("write_pipeline".into()),
            runs: Some(vec![WritePipelineBenchmarkRun {
                sha: Some("abcdef1".into()),
                short_sha: Some("abcdef1".into()),
                timestamp: Some("2026-08-26T00:00:00Z".into()),
                branch: Some("main".into()),
                run_number: Some(3),
                run_url: Some("https://github.com/Rick1330/ibex-harness/actions/runs/99".into()),
                iterations: Some(50),
                metrics: Some(WritePipelineBenchmarkMetrics {
                    latency_ms_p50: Some(10.0),
                    latency_ms_p95: Some(50.0),
                    latency_ms_p99: Some(80.0),
                }),
                status: Some("pass".into()),
                gate_summary: None,
            }]),
        }
    }

    #[test]
    fn accepts_valid_payload() {
        validate_write_pipeline_payload(&sample_payload()).expect("valid");
    }

    #[test]
    fn rejects_wrong_benchmark_name() {
        let mut payload = sample_payload();
        payload.benchmark = Some("proxy".into());
        assert!(validate_write_pipeline_payload(&payload).is_err());
    }
}
