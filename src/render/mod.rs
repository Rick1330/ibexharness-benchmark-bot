mod sanitize;

use crate::model::{BenchmarkData, BenchmarkRun, GateResult, StageMetrics};
pub use sanitize::{
    escape_cell, format_delta, format_number, sanitize_branch, sanitize_gate_name, sanitize_sha,
    status_emoji,
};

const DOCS_BASE: &str = "https://docs.ibexharness.com/benchmarks/history";
const BRAND_MARK_LIGHT: &str =
    "https://raw.githubusercontent.com/Rick1330/ibexharness-benchmark-bot/main/docs/brand/ibex-mark-light.png";
const BRAND_MARK_DARK: &str =
    "https://raw.githubusercontent.com/Rick1330/ibexharness-benchmark-bot/main/docs/brand/ibex-mark-dark.png";

const BRAND_NAME: &str = "IBEX Harness Benchmark Bot";
const BRAND_LOGO_PX: u32 = 48;

fn brand_header() -> String {
    format!(
        r#"<p align="left">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="{BRAND_MARK_DARK}">
    <img alt="{BRAND_NAME}" src="{BRAND_MARK_LIGHT}" width="{BRAND_LOGO_PX}" height="{BRAND_LOGO_PX}" valign="middle">
  </picture>
  <strong>{BRAND_NAME}</strong>
</p>"#
    )
}

pub fn render_pr_comment(data: &BenchmarkData, gate: &GateResult) -> Result<String, String> {
    let run = data
        .runs
        .as_ref()
        .and_then(|runs| runs.first())
        .ok_or_else(|| "benchmark data has no runs".to_string())?;

    let short_sha = resolve_short_sha(run);
    let branch = sanitize_branch(run.branch.as_deref().unwrap_or("unknown"));
    let status = run.status.as_deref().unwrap_or("unknown");
    let emoji = status_emoji(status);
    let run_number = run
        .run_number
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".to_string());

    let sections = vec![
        brand_header(),
        String::new(),
        format!("## Benchmark Results — Run #{run_number}"),
        String::new(),
        format!(
            "**Status:** {emoji} **{}** | Commit: `{short_sha}` | Branch: `{branch}` | [View dashboard →]({DOCS_BASE}/{short_sha})",
            status.to_uppercase()
        ),
        String::new(),
        "### Regression gate".to_string(),
        String::new(),
        render_gate_table(gate),
        String::new(),
        "### Load test (k6)".to_string(),
        String::new(),
        render_k6_table(run),
        String::new(),
        "### Stage breakdown (p99)".to_string(),
        String::new(),
        render_stage_table(run.stages.as_ref()),
        String::new(),
        render_stage_mermaid(run.stages.as_ref()),
        String::new(),
        "### Go microbench (BenchmarkProxyOverhead)".to_string(),
        String::new(),
        render_go_table(run),
        String::new(),
        "<details>".to_string(),
        "<summary>Environment</summary>".to_string(),
        String::new(),
        render_env_table(run),
        String::new(),
        "</details>".to_string(),
        String::new(),
        "> Regression threshold: >10% degradation on proxy p99 fails CI.".to_string(),
    ];

    Ok(sections.join("\n"))
}

pub fn render_data_pr_body(
    data: &BenchmarkData,
    run_url: Option<&str>,
    run_number: Option<i64>,
) -> String {
    let run = data.runs.as_ref().and_then(|runs| runs.first());
    let short_sha = run
        .map(resolve_short_sha)
        .unwrap_or_else(|| sanitize_sha(None));
    let status = run.and_then(|r| r.status.as_deref()).unwrap_or("unknown");
    let number = run_number
        .or(run.and_then(|r| r.run_number))
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".to_string());
    let p99 = format!(
        "{} ms",
        format_number(run.and_then(|r| r.k6.as_ref()).and_then(|k| k.p99_ms),)
    );

    let mut lines = vec![
        brand_header(),
        String::new(),
        "## Automated benchmark data update".to_string(),
        String::new(),
        format!(
            "**Status:** {} **{}**",
            status_emoji(status),
            status.to_uppercase()
        ),
        String::new(),
        markdown_table(
            &["Field", "Value"],
            &[
                vec!["Run number".to_string(), number],
                vec!["Head SHA".to_string(), short_sha],
                vec!["Proxy p99".to_string(), p99],
            ],
        ),
    ];
    if let Some(url) = run_url {
        lines.push(String::new());
        lines.push(format!("- [Harness benchmark workflow run]({url})"));
    }
    lines.push(String::new());
    lines.push("### Reviewer checklist".to_string());
    lines.push(String::new());
    lines.push("- [ ] Validation passed in bot workflow".to_string());
    lines.push("- [ ] Harness CI green".to_string());
    lines.push("- [ ] Docs preview shows updated history".to_string());
    lines.join("\n")
}

fn render_gate_table(gate: &GateResult) -> String {
    let Some(checks) = gate.checks.as_ref() else {
        return "_No gate checks available._".to_string();
    };
    if checks.is_empty() {
        return "_No gate checks available._".to_string();
    }
    let rows: Vec<Vec<String>> = checks
        .iter()
        .map(|check| {
            vec![
                sanitize_gate_name(check.name.as_deref()),
                format_number(check.value),
                format_number(check.limit),
                if check.ok.unwrap_or(false) {
                    "PASS".to_string()
                } else {
                    "FAIL".to_string()
                },
            ]
        })
        .collect();
    markdown_table(&["Check", "Value", "Limit", "Result"], &rows)
}

fn render_k6_table(run: &BenchmarkRun) -> String {
    let k6 = run.k6.as_ref();
    let p50 = format!("{} ms", format_number(k6.and_then(|k| k.p50_ms)));
    let p95 = format!("{} ms", format_number(k6.and_then(|k| k.p95_ms)));
    let p99 = format!("{} ms", format_number(k6.and_then(|k| k.p99_ms)));
    let p999 = format!("{} ms", format_number(k6.and_then(|k| k.p999_ms)));
    let throughput = format!(
        "{} req/s",
        format_number_precise(k6.and_then(|k| k.req_per_s), 2)
    );
    let error_rate = format!(
        "{}%",
        format_number_precise(k6.and_then(|k| k.error_rate).map(|v| v * 100.0), 3)
    );
    let check_rate = format!(
        "{}%",
        format_number_precise(k6.and_then(|k| k.check_rate).map(|v| v * 100.0), 1)
    );
    let delta = format_delta(run.regression_vs_baseline_pct);

    markdown_table(
        &["Metric", "Value", "Delta vs baseline"],
        &[
            vec!["p50".to_string(), p50, "—".to_string()],
            vec!["p95".to_string(), p95, "—".to_string()],
            vec!["p99".to_string(), p99, delta],
            vec!["p999".to_string(), p999, "—".to_string()],
            vec!["Throughput".to_string(), throughput, "—".to_string()],
            vec!["Error rate".to_string(), error_rate, "—".to_string()],
            vec!["Check rate".to_string(), check_rate, "—".to_string()],
        ],
    )
}

fn render_stage_table(stages: Option<&StageMetrics>) -> String {
    let stages = stages.cloned().unwrap_or(StageMetrics {
        auth_lru_p99_ms: None,
        auth_grpc_p99_ms: None,
        rate_limit_p99_ms: None,
        directive_resolve_p99_ms: None,
        prompt_inject_p99_ms: None,
        total_overhead_p99_ms: None,
    });
    markdown_table(
        &["Stage", "p99 (ms)"],
        &[
            vec![
                "Auth LRU".to_string(),
                format_number(stages.auth_lru_p99_ms),
            ],
            vec![
                "Auth gRPC".to_string(),
                format_number(stages.auth_grpc_p99_ms),
            ],
            vec![
                "Rate limit".to_string(),
                format_number(stages.rate_limit_p99_ms),
            ],
            vec![
                "Directive resolve".to_string(),
                format_number(stages.directive_resolve_p99_ms),
            ],
            vec![
                "Prompt inject".to_string(),
                format_number(stages.prompt_inject_p99_ms),
            ],
            vec![
                "Total overhead".to_string(),
                format_number(stages.total_overhead_p99_ms),
            ],
        ],
    )
}

fn render_go_table(run: &BenchmarkRun) -> String {
    let bench = run
        .go_benchmarks
        .as_ref()
        .and_then(|value| value.get("BenchmarkProxyOverhead"));
    let ns = bench
        .and_then(|v| v.get("ns_per_op"))
        .and_then(|v| v.as_f64());
    let allocs = bench
        .and_then(|v| v.get("allocs_per_op"))
        .and_then(|v| v.as_f64());
    let bytes = bench
        .and_then(|v| v.get("bytes_per_op"))
        .and_then(|v| v.as_f64());
    let low = bench
        .and_then(|v| v.get("ci_95_low"))
        .and_then(|v| v.as_f64());
    let high = bench
        .and_then(|v| v.get("ci_95_high"))
        .and_then(|v| v.as_f64());

    markdown_table(
        &["Metric", "Value"],
        &[
            vec!["ns/op".to_string(), format_number_precise(ns, 0)],
            vec!["allocs/op".to_string(), format_number_precise(allocs, 1)],
            vec!["bytes/op".to_string(), format_number_precise(bytes, 0)],
            vec!["95% CI low".to_string(), format_number_precise(low, 0)],
            vec!["95% CI high".to_string(), format_number_precise(high, 0)],
        ],
    )
}

fn render_env_table(run: &BenchmarkRun) -> String {
    let vcpus = run.runner_vcpus.map(|v| v.to_string());
    let ram = run.runner_ram_gb.map(|v| v.to_string());
    markdown_table(
        &["Field", "Value"],
        &[
            vec![
                "Go version".to_string(),
                escape_cell(run.go_version.as_deref()),
            ],
            vec![
                "Runner OS".to_string(),
                escape_cell(run.runner_os.as_deref()),
            ],
            vec![
                "Runner CPU".to_string(),
                escape_cell(run.runner_cpu.as_deref()),
            ],
            vec!["vCPUs".to_string(), escape_cell(vcpus.as_deref())],
            vec!["RAM (GB)".to_string(), escape_cell(ram.as_deref())],
            vec![
                "k6 version".to_string(),
                escape_cell(run.k6_version.as_deref()),
            ],
        ],
    )
}

fn render_stage_mermaid(stages: Option<&StageMetrics>) -> String {
    let Some(stages) = stages else {
        return String::new();
    };
    let entries: Vec<(&str, f64)> = [
        ("Auth LRU", stages.auth_lru_p99_ms),
        ("Auth gRPC", stages.auth_grpc_p99_ms),
        ("Rate limit", stages.rate_limit_p99_ms),
        ("Directive", stages.directive_resolve_p99_ms),
        ("Prompt", stages.prompt_inject_p99_ms),
        ("Total", stages.total_overhead_p99_ms),
    ]
    .into_iter()
    .filter_map(|(name, value)| value.map(|v| (name, v)))
    .collect();
    if entries.is_empty() {
        return String::new();
    }
    let names: Vec<&str> = entries.iter().map(|(name, _)| *name).collect();
    let values: Vec<String> = entries
        .iter()
        .map(|(_, value)| format_number(Some(*value)))
        .collect();
    format!(
        "```mermaid\nxychart-beta\n    title \"Stage p99 (ms)\"\n    x-axis [\"{}\"]\n    y-axis \"ms\"\n    bar [{}]\n```",
        names.join("\", \""),
        values.join(", ")
    )
}

fn resolve_short_sha(run: &BenchmarkRun) -> String {
    sanitize_sha(run.short_sha.as_deref().or(run.sha.as_deref()))
}

fn markdown_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let header_cells: Vec<String> = headers.iter().map(|cell| escape_cell(Some(cell))).collect();
    let mut lines = vec![
        format!("| {} |", header_cells.join(" | ")),
        format!(
            "| {} |",
            headers
                .iter()
                .map(|_| "---")
                .collect::<Vec<_>>()
                .join(" | ")
        ),
    ];
    for row in rows {
        let cells: Vec<String> = row
            .iter()
            .map(|cell| escape_cell(Some(cell.as_str())))
            .collect();
        lines.push(format!("| {} |", cells.join(" | ")));
    }
    lines.join("\n")
}

fn format_number_precise(value: Option<f64>, digits: usize) -> String {
    sanitize::format_number_precise(value, digits)
}
