use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::error::{bot_err, Result};
use crate::model::BenchmarkData;

const MAX_RUNS: usize = 365;
const MAX_P99_MS: f64 = 500.0;
const MAX_RUN_NUMBER: i64 = 1_000_000;

pub fn validate_file(path: &Path) -> Result<()> {
    let bytes = fs::read(path).map_err(|err| bot_err(format!("read failed: {err}")))?;
    let payload: BenchmarkData =
        serde_json::from_slice(&bytes).map_err(|err| bot_err(format!("json decode failed: {err}")))?;
    validate_payload(&payload)
}

pub fn validate_payload(payload: &BenchmarkData) -> Result<()> {
    if payload.schema_version != Some(1) {
        return Err(bot_err("schema_version must be 1".to_string()));
    }
    if payload.baseline_sha.as_deref().unwrap_or("").is_empty() {
        return Err(bot_err("baseline_sha required".to_string()));
    }
    let runs = payload
        .runs
        .as_ref()
        .ok_or_else(|| bot_err("runs must be an array".to_string()))?;
    if runs.len() > MAX_RUNS {
        return Err(bot_err(format!("runs exceeds max {MAX_RUNS}")));
    }

    let mut seen_sha = HashSet::new();
    let mut seen_pr = HashSet::new();
    for (index, run) in runs.iter().enumerate() {
        validate_run(run, index)?;
        if let Some(sha) = run.sha.as_deref() {
            if !seen_sha.insert(sha.to_string()) {
                return Err(bot_err(format!("duplicate sha: {sha}")));
            }
        }
        if let Some(pr) = run.pr_number {
            if !seen_pr.insert(pr) {
                return Err(bot_err(format!("duplicate pr_number: {pr}")));
            }
        }
    }
    Ok(())
}

fn validate_run(run: &crate::model::BenchmarkRun, index: usize) -> Result<()> {
    let label = format!("runs[{index}]");
    require_string(run.sha.as_deref(), &format!("{label}.sha"))?;
    require_string(run.short_sha.as_deref(), &format!("{label}.short_sha"))?;
    let status = require_string(run.status.as_deref(), &format!("{label}.status"))?;
    if !matches!(status.as_str(), "pass" | "regression" | "fail" | "unknown") {
        return Err(bot_err(format!("{label}.status invalid: {status}")));
    }
    if let Some(pr) = run.pr_number {
        if pr <= 0 {
            return Err(bot_err(format!("{label}.pr_number must be positive")));
        }
    }
    validate_run_number(run, &label)?;
    validate_k6(run.k6.as_ref(), &format!("{label}.k6"))?;
    Ok(())
}

fn validate_k6(k6: Option<&crate::model::K6Metrics>, label: &str) -> Result<()> {
    let k6 = k6.ok_or_else(|| bot_err(format!("{label} required")))?;
    let p99 = k6.p99_ms.ok_or_else(|| bot_err(format!("{label}.p99_ms required")))?;
    if p99 <= 0.0 || p99 > MAX_P99_MS {
        return Err(bot_err(format!("{label}.p99_ms out of bounds: {p99}")));
    }
    let error_rate = k6.error_rate.unwrap_or(0.0);
    if !(0.0..=1.0).contains(&error_rate) {
        return Err(bot_err(format!("{label}.error_rate out of bounds: {error_rate}")));
    }
    Ok(())
}

fn validate_run_number(run: &crate::model::BenchmarkRun, label: &str) -> Result<()> {
    let Some(run_number) = run.run_number else {
        return Ok(());
    };
    if run_number <= 0 || run_number > MAX_RUN_NUMBER {
        return Err(bot_err(format!("{label}.run_number out of bounds: {run_number}")));
    }
    if let Some(run_id) = run_id_from_url(run.run_url.as_deref()) {
        if run_number == run_id {
            return Err(bot_err(format!(
                "{label}.run_number must be workflow run number, not run id"
            )));
        }
    }
    Ok(())
}

fn run_id_from_url(run_url: Option<&str>) -> Option<i64> {
    let url = run_url?;
    let marker = "/actions/runs/";
    let tail = url.rsplit(marker).next()?;
    if tail == url {
        return None;
    }
    let id = tail.trim_matches('/');
    id.parse().ok()
}

fn require_string(value: Option<&str>, label: &str) -> Result<String> {
    let value = value.ok_or_else(|| bot_err(format!("{label} must be a string")))?;
    if value.is_empty() {
        return Err(bot_err(format!("{label} must not be empty")));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_run_number_equal_to_run_id() {
        let payload = BenchmarkData {
            schema_version: Some(1),
            baseline_sha: Some("abc".to_string()),
            runs: Some(vec![crate::model::BenchmarkRun {
                sha: Some("abc123".to_string()),
                short_sha: Some("abc123".to_string()),
                branch: Some("main".to_string()),
                pr_number: None,
                status: Some("pass".to_string()),
                run_number: Some(12345),
                run_url: Some("https://github.com/o/r/actions/runs/12345".to_string()),
                regression_vs_baseline_pct: Some(0.0),
                go_version: None,
                runner_os: None,
                runner_cpu: None,
                runner_vcpus: None,
                runner_ram_gb: None,
                k6_version: None,
                k6: Some(crate::model::K6Metrics {
                    p50_ms: Some(1.0),
                    p95_ms: Some(2.0),
                    p99_ms: Some(3.0),
                    p999_ms: Some(4.0),
                    req_per_s: Some(100.0),
                    error_rate: Some(0.0),
                    check_rate: Some(1.0),
                }),
                stages: None,
                go_benchmarks: None,
            }]),
        };
        assert!(validate_payload(&payload).is_err());
    }
}
