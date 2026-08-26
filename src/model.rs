use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BenchmarkData {
    pub schema_version: Option<i64>,
    pub baseline_sha: Option<String>,
    pub runs: Option<Vec<BenchmarkRun>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BenchmarkRun {
    pub sha: Option<String>,
    pub short_sha: Option<String>,
    pub branch: Option<String>,
    pub pr_number: Option<i64>,
    pub status: Option<String>,
    pub run_number: Option<i64>,
    pub run_url: Option<String>,
    pub regression_vs_baseline_pct: Option<f64>,
    pub go_version: Option<String>,
    pub runner_os: Option<String>,
    pub runner_cpu: Option<String>,
    pub runner_vcpus: Option<i64>,
    pub runner_ram_gb: Option<i64>,
    pub k6_version: Option<String>,
    pub k6: Option<K6Metrics>,
    pub stages: Option<StageMetrics>,
    pub go_benchmarks: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct K6Metrics {
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
    pub p999_ms: Option<f64>,
    pub req_per_s: Option<f64>,
    pub error_rate: Option<f64>,
    pub check_rate: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageMetrics {
    pub auth_lru_p99_ms: Option<f64>,
    pub auth_grpc_p99_ms: Option<f64>,
    pub rate_limit_p99_ms: Option<f64>,
    pub directive_resolve_p99_ms: Option<f64>,
    pub prompt_inject_p99_ms: Option<f64>,
    pub total_overhead_p99_ms: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GateResult {
    pub status: Option<String>,
    pub regression_pct: Option<f64>,
    pub checks: Option<Vec<GateCheck>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GateCheck {
    pub name: Option<String>,
    pub value: Option<f64>,
    pub limit: Option<f64>,
    pub ok: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DispatchPayload {
    pub run_id: i64,
    pub head_sha: String,
    pub run_number: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowRun {
    pub conclusion: Option<String>,
    pub head_branch: Option<String>,
    pub head_sha: Option<String>,
    pub run_number: Option<i64>,
    pub name: Option<String>,
    pub path: Option<String>,
    pub html_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HnswBenchmarkData {
    pub schema_version: Option<i64>,
    pub benchmark: Option<String>,
    pub runs: Option<Vec<HnswBenchmarkRun>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HnswBenchmarkRun {
    pub sha: Option<String>,
    pub short_sha: Option<String>,
    pub timestamp: Option<String>,
    pub branch: Option<String>,
    pub run_number: Option<i64>,
    pub run_url: Option<String>,
    pub methodology: Option<Value>,
    pub mean_recall_at_10: Option<f64>,
    pub results: Option<Vec<HnswSizeResult>>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HnswSizeResult {
    pub corpus_size: Option<i64>,
    pub query_count: Option<i64>,
    pub recall_at_10: Option<f64>,
    pub latency_ms_p50: Option<f64>,
    pub latency_ms_p95: Option<f64>,
    pub latency_ms_p99: Option<f64>,
    pub latency_ms_p95_ci_low: Option<f64>,
    pub latency_ms_p95_ci_high: Option<f64>,
    pub ef_search: Option<i64>,
    pub min_similarity: Option<f64>,
    pub iterative_scan: Option<String>,
    pub index_build_mode: Option<String>,
    pub plan_node_type: Option<String>,
    pub plan_index_name: Option<String>,
    pub shared_hit_blocks: Option<i64>,
    pub shared_read_blocks: Option<i64>,
    pub idx_scan_delta: Option<i64>,
    pub row_count_verified: Option<i64>,
}
