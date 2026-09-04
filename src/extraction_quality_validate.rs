//! Validate published extraction-quality benchmark JSON.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::error::{bot_err, Result};
use crate::model::{
    ExtractionQualityBenchmarkData, ExtractionQualityBenchmarkMetrics,
    ExtractionQualityBenchmarkRun, WorkflowRun,
};
use crate::ranking_quality_validate::{
    cross_check_memory_suite_run, memory_suite_max_published_run_number,
    memory_suite_published_sha_exists, MemorySuiteRunFields,
};
use crate::verify::is_hex_sha;

const MAX_RUNS: usize = 50;
const MAX_JSON_BYTES: usize = 2 * 1024 * 1024;
const MAX_RUN_NUMBER: i64 = 1_000_000;

pub fn validate_extraction_quality_file(path: &Path) -> Result<()> {
    let bytes = fs::read(path).map_err(|err| bot_err(format!("read failed: {err}")))?;
    if bytes.len() > MAX_JSON_BYTES {
        return Err(bot_err(format!("json exceeds {MAX_JSON_BYTES} bytes")));
    }
    let payload: ExtractionQualityBenchmarkData = serde_json::from_slice(&bytes)
        .map_err(|err| bot_err(format!("extraction-quality json decode failed: {err}")))?;
    validate_extraction_quality_payload(&payload)
}

pub fn validate_extraction_quality_payload(payload: &ExtractionQualityBenchmarkData) -> Result<()> {
    if payload.schema_version != Some(1) {
        return Err(bot_err(
            "extraction-quality schema_version must be 1".to_string(),
        ));
    }
    if payload.benchmark.as_deref() != Some("extraction_quality") {
        return Err(bot_err(
            "extraction-quality benchmark must be extraction_quality".to_string(),
        ));
    }
    let runs = payload
        .runs
        .as_ref()
        .ok_or_else(|| bot_err("extraction-quality runs must be an array".to_string()))?;
    if runs.is_empty() {
        return Err(bot_err(
            "extraction-quality runs must not be empty".to_string(),
        ));
    }
    if runs.len() > MAX_RUNS {
        return Err(bot_err(format!(
            "extraction-quality runs exceeds max {MAX_RUNS}"
        )));
    }
    let mut seen_sha = HashSet::new();
    for (index, run) in runs.iter().enumerate() {
        validate_extraction_quality_run(run, index)?;
        if let Some(sha) = run.sha.as_deref() {
            if !seen_sha.insert(sha.to_string()) {
                return Err(bot_err(format!("duplicate extraction-quality sha: {sha}")));
            }
        }
    }
    Ok(())
}

fn validate_extraction_quality_run(
    run: &ExtractionQualityBenchmarkRun,
    index: usize,
) -> Result<()> {
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
    validate_required_unit_metrics(metrics, index)?;
    validate_optional_unit_metrics(metrics, index)?;
    Ok(())
}

fn validate_required_unit_metrics(
    metrics: &ExtractionQualityBenchmarkMetrics,
    index: usize,
) -> Result<()> {
    for (name, value) in [
        ("precision_macro", metrics.precision_macro),
        ("recall_macro", metrics.recall_macro),
        (
            "category_assignment_accuracy",
            metrics.category_assignment_accuracy,
        ),
        ("temporal_field_accuracy", metrics.temporal_field_accuracy),
    ] {
        let v = value.ok_or_else(|| bot_err(format!("runs[{index}].metrics.{name} required")))?;
        require_unit_metric(v, index, name)?;
    }
    Ok(())
}

fn validate_optional_unit_metrics(
    metrics: &ExtractionQualityBenchmarkMetrics,
    index: usize,
) -> Result<()> {
    for (name, value) in [
        ("precision_factual", metrics.precision_factual),
        ("recall_factual", metrics.recall_factual),
        ("precision_preference", metrics.precision_preference),
        ("recall_preference", metrics.recall_preference),
        ("precision_behavioral", metrics.precision_behavioral),
        ("recall_behavioral", metrics.recall_behavioral),
        ("precision_episodic", metrics.precision_episodic),
        ("recall_episodic", metrics.recall_episodic),
        ("precision_procedural", metrics.precision_procedural),
        ("recall_procedural", metrics.recall_procedural),
    ] {
        if let Some(v) = value {
            require_unit_metric(v, index, name)?;
        }
    }
    Ok(())
}

fn require_unit_metric(value: f64, index: usize, name: &str) -> Result<()> {
    if !(0.0..=1.0).contains(&value) || !value.is_finite() {
        return Err(bot_err(format!(
            "runs[{index}].metrics.{name} out of range"
        )));
    }
    Ok(())
}

pub fn cross_check_extraction_quality_artifact_run(
    payload: &ExtractionQualityBenchmarkData,
    workflow: &WorkflowRun,
    run_id: i64,
    expected_run_number: i64,
) -> Result<()> {
    cross_check_memory_suite_run(
        payload.runs.as_deref(),
        workflow,
        run_id,
        expected_run_number,
        "extraction-quality",
    )
}

pub fn extraction_quality_published_sha_exists(
    data: &ExtractionQualityBenchmarkData,
    head_sha: &str,
) -> bool {
    memory_suite_published_sha_exists(data.runs.as_deref(), head_sha)
}

pub fn extraction_quality_max_published_run_number(
    data: &ExtractionQualityBenchmarkData,
) -> Option<i64> {
    memory_suite_max_published_run_number(data.runs.as_deref())
}

impl MemorySuiteRunFields for ExtractionQualityBenchmarkRun {
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
    use crate::model::{ExtractionQualityBenchmarkMetrics, ExtractionQualityBenchmarkRun};

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

    fn sample_payload() -> ExtractionQualityBenchmarkData {
        ExtractionQualityBenchmarkData {
            schema_version: Some(1),
            benchmark: Some("extraction_quality".into()),
            runs: Some(vec![ExtractionQualityBenchmarkRun {
                sha: Some("abcdef1".into()),
                short_sha: Some("abcdef1".into()),
                timestamp: Some("2026-09-03T00:00:00Z".into()),
                branch: Some("main".into()),
                run_number: Some(3),
                run_url: Some("https://github.com/Rick1330/ibex-harness/actions/runs/99".into()),
                gold_set: Some("v1".into()),
                conversation_count: Some(125),
                provider: Some("openai".into()),
                enforcement: Some("ci".into()),
                mode: Some("cassette".into()),
                model: Some("gpt-4o-mini".into()),
                metrics: Some(ExtractionQualityBenchmarkMetrics {
                    precision_macro: Some(1.0),
                    recall_macro: Some(1.0),
                    category_assignment_accuracy: Some(1.0),
                    temporal_field_accuracy: Some(1.0),
                    precision_factual: Some(1.0),
                    recall_factual: Some(1.0),
                    ..Default::default()
                }),
                status: Some("pass".into()),
                gate_summary: None,
            }]),
        }
    }

    #[test]
    fn accepts_valid_payload() {
        validate_extraction_quality_payload(&sample_payload()).expect("valid");
    }

    #[test]
    fn rejects_wrong_benchmark_name() {
        let mut payload = sample_payload();
        payload.benchmark = Some("ranking_quality".into());
        assert!(validate_extraction_quality_payload(&payload).is_err());
    }

    #[test]
    fn rejects_out_of_range_optional_metric() {
        let mut payload = sample_payload();
        payload.runs.as_mut().unwrap()[0]
            .metrics
            .as_mut()
            .unwrap()
            .precision_factual = Some(1.5);
        assert!(validate_extraction_quality_payload(&payload).is_err());
    }

    #[test]
    fn cross_check_rejects_future_run_number_in_history() {
        let mut payload = sample_payload();
        let template = payload.runs.as_ref().unwrap()[0].clone();
        payload
            .runs
            .as_mut()
            .unwrap()
            .push(ExtractionQualityBenchmarkRun {
                sha: Some("fedcba9".into()),
                short_sha: Some("fedcba9".into()),
                run_number: Some(999),
                ..template
            });
        let error =
            cross_check_extraction_quality_artifact_run(&payload, &verified_workflow(), 99, 3)
                .expect_err("future run_number must be rejected");
        assert!(error.to_string().contains("run_number exceeds"));
    }

    #[test]
    fn cross_check_accepts_matching_workflow_html_url() {
        cross_check_extraction_quality_artifact_run(&sample_payload(), &verified_workflow(), 99, 3)
            .expect("valid artifact cross-check");
    }
}
