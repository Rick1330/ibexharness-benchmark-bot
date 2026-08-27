use std::fs;
use std::path::Path;

use ibex_benchmark_bot::model::{BenchmarkData, GateResult, HnswBenchmarkData};
use ibex_benchmark_bot::render::{
    merge_hnsw_into_comment, merge_proxy_into_comment, render_pr_comment, COMMENT_MARKER,
    COMMENT_MARKER_HNSW,
};

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
    assert!(body.contains("<!-- IBEX_PROXY_START -->"));
    assert!(body.contains("### Proxy"));
    assert!(body.contains("| Metric | Value | vs Baseline | Visual |"));
    assert!(body.contains("<summary>Auth & proxy stages (synthetic)</summary>"));
    assert!(!body.contains("open>"));
    assert!(!body.contains("img.shields.io"));
    assert!(body.contains("k6 p99 SLA") || body.contains("k6 p99"));
    assert!(body.contains("Auth LRU"));
    assert!(body.contains("Auth gRPC"));
    assert!(!body.contains("```mermaid"));
    assert!(body.contains("<details>"));
    assert!(body.contains("<summary>More</summary>"));
    assert!(body.contains("](https://github.com/Rick1330/ibex-harness/commit/"));
    assert!(!body.contains("`[`"));
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
    assert!(body.contains("Auth & proxy stages"));
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
    assert!(!body.contains("<summary>Auth & proxy stages (synthetic)</summary>"));
    assert!(!body.contains("| Auth LRU |"));
}

#[test]
fn merge_keeps_proxy_and_hnsw_in_one_comment() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let data: BenchmarkData = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/benchmark-data.json")).unwrap(),
    )
    .unwrap();
    let gate: GateResult = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/gate-result.json")).unwrap(),
    )
    .unwrap();
    let proxy = render_pr_comment(&data, &gate).expect("proxy");
    let hnsw: HnswBenchmarkData = serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "benchmark": "hnsw_recall_latency",
        "runs": [{
            "sha": "abcdef1",
            "short_sha": "abcdef1",
            "status": "pass",
            "mean_recall_at_10": 0.99,
            "results": [{
                "corpus_size": 10000,
                "recall_at_10": 0.99,
                "latency_ms_p95": 1.2
            }]
        }]
    }))
    .unwrap();
    let body = merge_hnsw_into_comment(&proxy, &hnsw).expect("merge");
    assert_eq!(body.matches(COMMENT_MARKER).count(), 1);
    assert!(!body.contains(COMMENT_MARKER_HNSW));
    let proxy_idx = body.find("### Proxy").expect("proxy heading");
    let hnsw_idx = body.find("### Memory HNSW").expect("hnsw heading");
    assert!(proxy_idx < hnsw_idx);
    let proxy_again = merge_proxy_into_comment(&body, &data, &gate).expect("re-merge proxy");
    assert!(proxy_again.contains("### Memory HNSW"));
    assert!(proxy_again.contains("### Proxy"));

    let hnsw_first = merge_hnsw_into_comment("", &hnsw).expect("hnsw first");
    let proxy_second = merge_proxy_into_comment(&hnsw_first, &data, &gate).expect("proxy second");
    let proxy_idx = proxy_second.find("### Proxy").expect("proxy heading");
    let hnsw_idx = proxy_second.find("### Memory HNSW").expect("hnsw heading");
    assert!(proxy_idx < hnsw_idx);
}
