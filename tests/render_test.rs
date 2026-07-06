use std::fs;
use std::path::Path;

use ibex_benchmark_bot::model::{BenchmarkData, GateResult};
use ibex_benchmark_bot::render::{render_pr_comment, COMMENT_MARKER};

#[test]
fn render_pr_comment_uses_triage_layout() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let data: BenchmarkData = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/benchmark-data.json")).unwrap(),
    )
    .unwrap();
    let gate: GateResult = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/gate-result.json")).unwrap(),
    )
    .unwrap();
    let body = render_pr_comment(&data, &gate).expect("render");
    assert!(body.contains(COMMENT_MARKER));
    assert!(body.contains("IBEX Benchmark Bot"));
    assert!(body.contains("width=\"32\""));
    assert!(body.contains("Performance summary"));
    assert!(body.contains("img.shields.io"));
    assert!(body.contains("k6 p99 SLA"));
    assert!(body.contains("Auth LRU"));
    assert!(!body.contains("```mermaid"));
    assert!(body.contains("<details>"));
}

#[test]
fn render_pr_comment_formats_sub_ms_stages() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut data: BenchmarkData = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/benchmark-data.json")).unwrap(),
    )
    .unwrap();
    let gate: GateResult = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/gate-result.json")).unwrap(),
    )
    .unwrap();
    if let Some(run) = data.runs.as_mut().and_then(|runs| runs.first_mut()) {
        run.stages = Some(ibex_benchmark_bot::model::StageMetrics {
            auth_lru_p99_ms: Some(0.000376),
            auth_grpc_p99_ms: Some(0.0),
            rate_limit_p99_ms: Some(0.0),
            directive_resolve_p99_ms: Some(0.0),
            prompt_inject_p99_ms: Some(0.0),
            total_overhead_p99_ms: Some(0.000376),
        });
    }
    let body = render_pr_comment(&data, &gate).expect("render");
    assert!(body.contains("Stage breakdown"));
    assert!(body.contains("376 ns") || body.contains("0.38 µs"));
    assert!(body.contains("Data model"));
}

#[test]
fn render_pr_comment_hides_zero_stage_rows() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut data: BenchmarkData = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/benchmark-data.json")).unwrap(),
    )
    .unwrap();
    let gate: GateResult = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/gate-result.json")).unwrap(),
    )
    .unwrap();
    if let Some(run) = data.runs.as_mut().and_then(|runs| runs.first_mut()) {
        run.stages = Some(ibex_benchmark_bot::model::StageMetrics {
            auth_lru_p99_ms: Some(0.0),
            auth_grpc_p99_ms: Some(0.0),
            rate_limit_p99_ms: Some(0.0),
            directive_resolve_p99_ms: Some(0.0),
            prompt_inject_p99_ms: Some(0.0),
            total_overhead_p99_ms: Some(0.0),
        });
    }
    let body = render_pr_comment(&data, &gate).expect("render");
    assert!(!body.contains("### Stage breakdown (synthetic)"));
    assert!(!body.contains("| Auth LRU |"));
}
