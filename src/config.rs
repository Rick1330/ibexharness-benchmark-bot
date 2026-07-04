pub const EXPECTED_HARNESS_REPO: &str = "Rick1330/ibex-harness";
pub const EXPECTED_WORKFLOW_NAME: &str = "Benchmarks";
pub const EXPECTED_WORKFLOW_PATH: &str = ".github/workflows/benchmark.yml";
pub const BENCHMARK_DATA_PATH: &str = "docs/app/public/benchmarks/benchmark-data.json";
pub const BADGE_PATH: &str = "docs/app/public/benchmarks/badge.svg";
pub const BENCHMARK_DATA_LABEL: &str = "benchmark-data";

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

    #[test]
    fn rejects_unexpected_repo_without_override() {
        std::env::remove_var("ALLOW_HARNESS_REPO_OVERRIDE");
        let err = resolve_harness_repo("evil/evil").expect_err("must reject");
        assert!(err.contains(EXPECTED_HARNESS_REPO));
    }

    #[test]
    fn accepts_expected_repo_without_override() {
        std::env::remove_var("ALLOW_HARNESS_REPO_OVERRIDE");
        assert_eq!(
            resolve_harness_repo(EXPECTED_HARNESS_REPO).expect("ok"),
            EXPECTED_HARNESS_REPO
        );
    }

    #[test]
    fn empty_override_does_not_bypass_lock() {
        std::env::set_var("ALLOW_HARNESS_REPO_OVERRIDE", "");
        let err = resolve_harness_repo("evil/evil").expect_err("empty must not bypass");
        assert!(err.contains(EXPECTED_HARNESS_REPO));
        std::env::remove_var("ALLOW_HARNESS_REPO_OVERRIDE");
    }

    #[test]
    fn truthy_override_allows_any_repo() {
        std::env::set_var("ALLOW_HARNESS_REPO_OVERRIDE", "true");
        assert_eq!(
            resolve_harness_repo("evil/evil").expect("override"),
            "evil/evil"
        );
        std::env::remove_var("ALLOW_HARNESS_REPO_OVERRIDE");
    }
}
