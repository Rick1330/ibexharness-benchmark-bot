use std::fs;
use std::path::Path;

use ibex_benchmark_bot::model::{BenchmarkData, GateResult};
use ibex_benchmark_bot::render::render_pr_comment;

#[test]
fn render_pr_comment_includes_gate_and_stages() {
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
    assert!(body.contains("IBEX Harness Benchmark Bot"));
    assert!(body.contains("width=\"48\""));
    assert!(body.contains("## Benchmark Results"));
    assert!(body.contains("Regression gate"));
    assert!(body.contains("k6 p99 SLA"));
    assert!(body.contains("```mermaid"));
}
