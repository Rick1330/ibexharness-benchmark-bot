use std::fs;
use std::path::Path;

use ibex_benchmark_bot::model::{BenchmarkData, GateResult, HnswBenchmarkData};
use ibex_benchmark_bot::render::{
    merge_hnsw_into_comment, merge_proxy_into_comment, render_pr_comment, COMMENT_MARKER,
    COMMENT_MARKER_HNSW,
};

#[test]
fn render_pr_comment_uses_matrix_layout() {
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
    assert!(body.contains("View IBEX dashboard"));
    assert!(body.contains("#### Suite matrix"));
    assert!(body.contains("| Suite | Primary SLA | vs Baseline | Status |"));
    assert!(body.contains("**Proxy**"));
    assert!(body.contains("<!-- IBEX_PROXY_START -->"));
    assert!(body.contains("<summary>Deep Dive: Proxy</summary>"));
    assert!(body.contains("<br>"));
    assert!(!body.contains("open>"));
    assert!(!body.contains("img.shields.io"));
    assert!(body.contains("Auth LRU"));
    assert!(body.contains("<!-- IBEX_ENV_START -->"));
    assert!(body.contains("<summary>Environment & meta</summary>"));
    assert!(!body.contains("```mermaid"));
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
    assert!(body.contains("Auth LRU"));
    assert!(body.contains("376 ns") || body.contains("0.38 µs"));
}

#[test]
fn render_pr_comment_hides_zero_stage_chips() {
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
    assert!(!body.contains("**Stages:**"));
    assert!(!body.contains("Auth LRU"));
}

#[test]
fn merge_keeps_proxy_and_hnsw_in_one_matrix_comment() {
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
            }],
            "gate_summary": {"has_1m": false, "recall_ok": true}
        }]
    }))
    .unwrap();
    let body = merge_hnsw_into_comment(&proxy, &hnsw).expect("merge");
    assert_eq!(body.matches(COMMENT_MARKER).count(), 1);
    assert!(!body.contains(COMMENT_MARKER_HNSW));
    assert!(body.contains("✅ All benchmarks passed"));
    assert!(body.contains("**Proxy**"));
    assert!(body.contains("**Memory HNSW**"));
    assert!(body.contains("<summary>Deep Dive: Memory HNSW</summary>"));
    assert!(body.contains("1M:** Deferred") || body.contains("Deferred *(smoke/fast"));
    let proxy_idx = body
        .find("<!-- IBEX_PROXY_START -->")
        .expect("proxy section");
    let hnsw_idx = body.find("<!-- IBEX_HNSW_START -->").expect("hnsw section");
    assert!(proxy_idx < hnsw_idx);

    let hnsw_first = merge_hnsw_into_comment("", &hnsw).expect("hnsw first");
    let proxy_second = merge_proxy_into_comment(&hnsw_first, &data, &gate).expect("proxy second");
    let proxy_idx = proxy_second
        .find("<!-- IBEX_PROXY_START -->")
        .expect("proxy section");
    let hnsw_idx = proxy_second
        .find("<!-- IBEX_HNSW_START -->")
        .expect("hnsw section");
    assert!(proxy_idx < hnsw_idx);
    assert!(proxy_second.contains("| Suite | Primary SLA | vs Baseline | Status |"));
}

#[test]
fn warn_gate_status_surfaces_warning_verdict() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let data: BenchmarkData = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/benchmark-data.json")).unwrap(),
    )
    .unwrap();
    let mut gate: GateResult = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/gate-result.json")).unwrap(),
    )
    .unwrap();
    gate.status = Some("warn".to_string());
    let body = render_pr_comment(&data, &gate).expect("render");
    assert!(body.contains("Benchmarks warning"));
    assert!(body.contains("<details open>"));
    assert!(body.contains("|proxy|Proxy|"));
    assert!(body.contains("|warn -->") || body.contains("|warn|") || body.contains("|warn -->"));
}

#[test]
fn hnsw_gate_note_is_sanitized() {
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
            }],
            "gate_summary": {
                "has_1m": false,
                "recall_ok": true,
                "note": "[click](https://evil.example) then |pipe|"
            }
        }]
    }))
    .unwrap();
    let body = merge_hnsw_into_comment("", &hnsw).expect("merge");
    assert!(
        !body.contains("](https://evil.example)"),
        "markdown link must be neutralized"
    );
    assert!(body.contains(r"\|") || body.contains("pipe"));
    assert!(body.contains("\n> "));
}
