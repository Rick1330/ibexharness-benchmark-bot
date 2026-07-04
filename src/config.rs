pub const EXPECTED_HARNESS_REPO: &str = "Rick1330/ibex-harness";
pub const EXPECTED_WORKFLOW_NAME: &str = "Benchmarks";
pub const EXPECTED_WORKFLOW_PATH: &str = ".github/workflows/benchmark.yml";
pub const BENCHMARK_DATA_PATH: &str = "docs/app/public/benchmarks/benchmark-data.json";
pub const BADGE_PATH: &str = "docs/app/public/benchmarks/badge.svg";
pub const BENCHMARK_DATA_LABEL: &str = "benchmark-data";

pub fn resolve_harness_repo(requested: &str) -> Result<&str, String> {
    if std::env::var("ALLOW_HARNESS_REPO_OVERRIDE").is_ok() {
        return Ok(requested);
    }
    if requested != EXPECTED_HARNESS_REPO {
        return Err(format!(
            "harness repo must be {EXPECTED_HARNESS_REPO} (set ALLOW_HARNESS_REPO_OVERRIDE to override)"
        ));
    }
    Ok(requested)
}
