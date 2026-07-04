use std::fs;
use std::path::Path;

use ibex_benchmark_bot::model::{BenchmarkData, GateCheck, GateResult};
use ibex_benchmark_bot::render::render_pr_comment;

#[test]
fn render_strips_phishing_gate_name() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let data: BenchmarkData = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/benchmark-data.json")).unwrap(),
    )
    .unwrap();
    let gate = GateResult {
        status: Some("pass".to_string()),
        regression_pct: Some(0.0),
        checks: Some(vec![GateCheck {
            name: Some("[evil](https://evil.example)".to_string()),
            value: Some(1.0),
            limit: Some(1.0),
            ok: Some(true),
        }]),
    };
    let body = render_pr_comment(&data, &gate).expect("render");
    assert!(!body.contains("evil.example"));
    assert!(body.contains("invalid"));
}
