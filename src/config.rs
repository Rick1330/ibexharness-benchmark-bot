pub const EXPECTED_HARNESS_REPO: &str = "Rick1330/ibex-harness";
pub const EXPECTED_WORKFLOW_NAME: &str = "Benchmarks";
pub const EXPECTED_WORKFLOW_PATH: &str = ".github/workflows/benchmark.yml";
pub const EXPECTED_HNSW_WORKFLOW_NAME: &str = "Memory Benchmarks";
pub const EXPECTED_HNSW_WORKFLOW_PATH: &str = ".github/workflows/memory-benchmark.yml";
pub const BENCHMARK_DATA_PATH: &str = "web/public/benchmarks/benchmark-data.json";
pub const BADGE_PATH: &str = "web/public/benchmarks/badge.svg";
pub const HNSW_BENCHMARK_DATA_PATH: &str = "web/public/benchmarks/hnsw-benchmark-data.json";
pub const BENCHMARK_DATA_LABEL: &str = "benchmark-data";
pub const HNSW_BENCHMARK_DATA_LABEL: &str = "hnsw-benchmark-data";
/// Shared branch for all suite data publishes — one open PR at a time.
pub const DATA_PR_BRANCH: &str = "chore/bench-data-publish";

pub fn resolve_harness_repo(requested: &str) -> Result<&str, String> {
    if harness_repo_override_enabled() {
        return Ok(requested);
    }
    if requested != EXPECTED_HARNESS_REPO {
        return Err(format!(
            "harness repo must be {EXPECTED_HARNESS_REPO} (set ALLOW_HARNESS_REPO_OVERRIDE=true to override)"
        ));
    }
    Ok(requested)
}

fn harness_repo_override_enabled() -> bool {
    matches!(
        std::env::var("ALLOW_HARNESS_REPO_OVERRIDE")
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn rejects_unexpected_repo_without_override() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let previous = std::env::var_os("ALLOW_HARNESS_REPO_OVERRIDE");
        std::env::remove_var("ALLOW_HARNESS_REPO_OVERRIDE");
        let err = resolve_harness_repo("evil/evil").expect_err("must reject");
        assert!(err.contains(EXPECTED_HARNESS_REPO));
        match previous {
            Some(v) => std::env::set_var("ALLOW_HARNESS_REPO_OVERRIDE", v),
            None => std::env::remove_var("ALLOW_HARNESS_REPO_OVERRIDE"),
        }
    }

    #[test]
    fn accepts_expected_repo_without_override() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let previous = std::env::var_os("ALLOW_HARNESS_REPO_OVERRIDE");
        std::env::remove_var("ALLOW_HARNESS_REPO_OVERRIDE");
        assert_eq!(
            resolve_harness_repo(EXPECTED_HARNESS_REPO).expect("ok"),
            EXPECTED_HARNESS_REPO
        );
        match previous {
            Some(v) => std::env::set_var("ALLOW_HARNESS_REPO_OVERRIDE", v),
            None => std::env::remove_var("ALLOW_HARNESS_REPO_OVERRIDE"),
        }
    }

    #[test]
    fn empty_override_does_not_bypass_lock() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let previous = std::env::var_os("ALLOW_HARNESS_REPO_OVERRIDE");
        std::env::set_var("ALLOW_HARNESS_REPO_OVERRIDE", "");
        let err = resolve_harness_repo("evil/evil").expect_err("empty must not bypass");
        assert!(err.contains(EXPECTED_HARNESS_REPO));
        match previous {
            Some(v) => std::env::set_var("ALLOW_HARNESS_REPO_OVERRIDE", v),
            None => std::env::remove_var("ALLOW_HARNESS_REPO_OVERRIDE"),
        }
    }

    #[test]
    fn truthy_override_allows_any_repo() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let previous = std::env::var_os("ALLOW_HARNESS_REPO_OVERRIDE");
        std::env::set_var("ALLOW_HARNESS_REPO_OVERRIDE", "true");
        assert_eq!(
            resolve_harness_repo("evil/evil").expect("override"),
            "evil/evil"
        );
        match previous {
            Some(v) => std::env::set_var("ALLOW_HARNESS_REPO_OVERRIDE", v),
            None => std::env::remove_var("ALLOW_HARNESS_REPO_OVERRIDE"),
        }
    }
}
