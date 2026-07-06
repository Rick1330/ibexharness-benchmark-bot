mod sanitize;

use crate::model::{BenchmarkData, BenchmarkRun, GateCheck, GateResult, StageMetrics};
pub use sanitize::{
    escape_cell, format_delta, format_latency_delta, format_number, format_throughput,
    format_throughput_delta, sanitize_branch, sanitize_gate_name, sanitize_sha, status_emoji,
    COMMENT_MARKER,
};

const DOCS_BASE: &str = "https://docs.ibexharness.com/benchmarks/history";
const HARNESS_REPO: &str = "https://github.com/Rick1330/ibex-harness";
const BRAND_MARK_LIGHT: &str =
    "https://raw.githubusercontent.com/Rick1330/ibexharness-benchmark-bot/main/docs/brand/ibex-mark-light.png";
const BRAND_MARK_DARK: &str =
    "https://raw.githubusercontent.com/Rick1330/ibexharness-benchmark-bot/main/docs/brand/ibex-mark-dark.png";

const BRAND_NAME: &str = "IBEX Benchmark Bot";
const BRAND_LOGO_PX: u32 = 32;
const P99_SLA_MS: f64 = 20.0;
const THROUGHPUT_BAR_SCALE: f64 = 10_000.0;
const STAGE_MIN_MS: f64 = 0.001;
const VISUAL_BAR_WIDTH: usize = 10;

pub fn render_pr_comment(data: &BenchmarkData, gate: &GateResult) -> Result<String, String> {
    let run = data
        .runs
        .as_ref()
        .and_then(|runs| runs.first())
        .ok_or_else(|| "benchmark data has no runs".to_string())?;

    let short_sha = resolve_short_sha(run);
    let status = run.status.as_deref().unwrap_or("unknown");
    let run_number = run
        .run_number
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".to_string());
    let baseline_sha = data.baseline_sha.as_deref();

    let sections = vec![
        COMMENT_MARKER.to_string(),
        render_header(&short_sha, &run_number),
        String::new(),
        render_verdict_banner(status, gate, run),
        String::new(),
        "---".to_string(),
        String::new(),
        "### 🏎️ Performance summary".to_string(),
        String::new(),
        render_performance_summary(run, baseline_sha),
        String::new(),
        render_verdict_note(status, run.regression_vs_baseline_pct),
        String::new(),
        render_details_block(run, gate, baseline_sha),
        String::new(),
        render_microbench_details(run),
        String::new(),
        render_env_details(run, baseline_sha),
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
        render_compact_brand(),
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

fn render_header(short_sha: &str, run_number: &str) -> String {
    format!(
        r#"<p align="left">
  <a href="{DOCS_BASE}/{short_sha}">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="{BRAND_MARK_DARK}">
      <img src="{BRAND_MARK_LIGHT}" width="{BRAND_LOGO_PX}" height="{BRAND_LOGO_PX}" align="left" valign="middle" alt="{BRAND_NAME}">
    </picture>
  </a>
  <strong>{BRAND_NAME}</strong> &nbsp;•&nbsp; Run #{run_number} &nbsp;•&nbsp; <code>{short_sha}</code>
</p>"#
    )
}

fn render_compact_brand() -> String {
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

fn render_verdict_banner(status: &str, gate: &GateResult, run: &BenchmarkRun) -> String {
    let failures = count_gate_failures(gate);
    let status_label = status.to_uppercase();
    let status_color = match status {
        "pass" => "brightgreen",
        "regression" => "yellow",
        _ => "red",
    };
    let regression_color = if failures == 0 { "green" } else { "red" };
    let p99 = run
        .k6
        .as_ref()
        .and_then(|k| k.p99_ms)
        .map(|v| format!("{v:.2}ms"))
        .unwrap_or_else(|| "n/a".to_string());

    format!(
        r#"<br/>
<p align="center">
  <a href="{DOCS_BASE}/{}">
    <img src="https://img.shields.io/badge/Status-{status_label}-{status_color}?style=for-the-badge&labelColor=2B2B2B" alt="Status: {status_label}">
  </a>
  <img src="https://img.shields.io/badge/Regressions-{failures}-{regression_color}?style=for-the-badge&labelColor=2B2B2B" alt="Regressions: {failures}">
  <img src="https://img.shields.io/badge/P99-{p99}-informational?style=for-the-badge&labelColor=2B2B2B" alt="P99 Latency">
</p>"#,
        resolve_short_sha(run)
    )
}

fn render_performance_summary(run: &BenchmarkRun, baseline_sha: Option<&str>) -> String {
    let k6 = run.k6.as_ref();
    let p99 = format!("`{} ms`", format_number(k6.and_then(|k| k.p99_ms)));
    let throughput = format!("`{}`", format_throughput(k6.and_then(|k| k.req_per_s)));
    let error_rate = format!(
        "`{}%`",
        sanitize::format_number_precise(k6.and_then(|k| k.error_rate).map(|v| v * 100.0), 2)
    );
    let latency_delta = format_latency_delta(run.regression_vs_baseline_pct);
    let throughput_delta = format_throughput_delta(run.regression_vs_baseline_pct);
    let baseline_col = baseline_link(baseline_sha, &latency_delta);
    let throughput_baseline = baseline_link(baseline_sha, &throughput_delta);
    let error_delta = if k6.and_then(|k| k.error_rate).unwrap_or(0.0) == 0.0 {
        "✅ No change".to_string()
    } else {
        "⚠️ Elevated".to_string()
    };

    markdown_table(
        &["Metric", "Value", "vs Baseline", "Visual"],
        &[
            vec![
                "**P99 latency**".to_string(),
                p99,
                baseline_col,
                sanitize::latency_visual_bar(
                    k6.and_then(|k| k.p99_ms),
                    P99_SLA_MS,
                    VISUAL_BAR_WIDTH,
                ),
            ],
            vec![
                "**Throughput**".to_string(),
                throughput,
                throughput_baseline,
                sanitize::throughput_visual_bar(
                    k6.and_then(|k| k.req_per_s),
                    THROUGHPUT_BAR_SCALE,
                    VISUAL_BAR_WIDTH,
                ),
            ],
            vec![
                "**Error rate**".to_string(),
                error_rate,
                error_delta,
                "`0 errors`".to_string(),
            ],
        ],
    )
}

fn baseline_link(baseline_sha: Option<&str>, delta: &str) -> String {
    let Some(sha) = baseline_sha else {
        return delta.to_string();
    };
    let safe = sanitize_sha(Some(sha));
    if safe == "invalid" || safe == "unknown" {
        return delta.to_string();
    }
    format!("[{delta}]({HARNESS_REPO}/commit/{safe})")
}

fn render_verdict_note(status: &str, regression_pct: Option<f64>) -> String {
    let note = match (status, regression_pct) {
        ("pass", Some(pct)) if pct < -5.0 => format!(
            "> **Verdict:** Significant performance improvement detected. P99 latency reduced by ~{:.0}%.",
            pct.abs()
        ),
        ("pass", Some(pct)) if pct > 5.0 => format!(
            "> **Verdict:** Gates passed, but P99 latency is ~{:.1}% higher than baseline. Review the delta before merging.",
            pct
        ),
        ("pass", _) => {
            "> **Verdict:** All benchmark gates passed. Safe to merge from a performance perspective."
                .to_string()
        }
        ("regression", Some(pct)) => format!(
            "> **Verdict:** Performance regression detected ({:.1}% vs baseline). Review before merging.",
            pct
        ),
        ("fail", _) => {
            "> **Verdict:** Benchmark gates failed. Do not merge until resolved.".to_string()
        }
        _ => "> **Verdict:** Benchmark status could not be determined.".to_string(),
    };
    note
}

fn render_details_block(
    run: &BenchmarkRun,
    gate: &GateResult,
    baseline_sha: Option<&str>,
) -> String {
    let mut lines = vec![
        "<details>".to_string(),
        "<summary><b>📊 Detailed breakdown</b></summary>".to_string(),
        String::new(),
        "### Load test (k6)".to_string(),
        String::new(),
        render_k6_detail_table(run, gate),
        String::new(),
        "### Regression analysis".to_string(),
        String::new(),
        render_gate_table(gate),
    ];

    if let Some(stage_section) = render_stage_details(run.stages.as_ref()) {
        lines.push(String::new());
        lines.push(stage_section);
    }

    lines.push(String::new());
    lines.push("</details>".to_string());
    if let Some(sha) = baseline_sha {
        let safe = sanitize_sha(Some(sha));
        if safe != "invalid" && safe != "unknown" {
            lines.push(String::new());
            lines.push(format!(
                "_Baseline commit: [`{safe}`]({HARNESS_REPO}/commit/{safe})_"
            ));
        }
    }
    lines.join("\n")
}

fn render_k6_detail_table(run: &BenchmarkRun, gate: &GateResult) -> String {
    let k6 = run.k6.as_ref();
    let p99_limit = gate_limit(gate, "k6 p99 SLA");
    let throughput_limit = gate_limit(gate, "k6 throughput present");
    let check_limit = gate_limit(gate, "k6 checks passing");
    let error_limit = gate_limit(gate, "error rate");

    markdown_table(
        &["Metric", "Value", "Limit", "Status"],
        &[
            gate_row("P50 latency", k6.and_then(|k| k.p50_ms), None, None),
            gate_row("P95 latency", k6.and_then(|k| k.p95_ms), None, None),
            gate_row(
                "P99 latency",
                k6.and_then(|k| k.p99_ms),
                p99_limit,
                Some(P99_SLA_MS),
            ),
            gate_row(
                "Throughput",
                k6.and_then(|k| k.req_per_s),
                throughput_limit,
                Some(1.0),
            ),
            gate_row(
                "Error rate",
                k6.and_then(|k| k.error_rate).map(|v| v * 100.0),
                error_limit.map(|v| v * 100.0),
                Some(0.1),
            ),
            gate_row(
                "Check rate",
                k6.and_then(|k| k.check_rate).map(|v| v * 100.0),
                check_limit.map(|v| v * 100.0),
                Some(99.0),
            ),
        ],
    )
}

fn gate_row(
    name: &str,
    value: Option<f64>,
    gate_limit: Option<f64>,
    default_limit: Option<f64>,
) -> Vec<String> {
    let limit = gate_limit.or(default_limit);
    let display_value = match name {
        "Throughput" => format_throughput(value),
        "Error rate" | "Check rate" => format!(
            "{}%",
            sanitize::format_number_precise(value, if name == "Error rate" { 2 } else { 1 })
        ),
        _ => format!("{} ms", format_number(value)),
    };
    let limit_text = limit
        .map(|v| {
            if name == "Throughput" {
                format!("> {v} req/s")
            } else if name.contains("rate") {
                format!("< {v}%")
            } else {
                format!("{v} ms")
            }
        })
        .unwrap_or_else(|| "—".to_string());
    let ok = value_is_within_limit(name, value, limit);
    vec![
        name.to_string(),
        display_value,
        limit_text,
        check_status(ok),
    ]
}

fn value_is_within_limit(name: &str, value: Option<f64>, limit: Option<f64>) -> Option<bool> {
    let (value, limit) = (value?, limit?);
    Some(match name {
        "Throughput" => value >= limit,
        "Error rate" => value <= limit,
        "Check rate" => value >= limit,
        "P99 latency" => value <= limit,
        _ => true,
    })
}

fn check_status(ok: Option<bool>) -> String {
    match ok {
        Some(true) => "✅".to_string(),
        Some(false) => "❌".to_string(),
        None => "—".to_string(),
    }
}

fn gate_limit(gate: &GateResult, name: &str) -> Option<f64> {
    gate.checks.as_ref().and_then(|checks| {
        checks
            .iter()
            .find(|check| check.name.as_deref() == Some(name))
            .and_then(|check| check.limit)
    })
}

fn render_gate_table(gate: &GateResult) -> String {
    let Some(checks) = gate.checks.as_ref() else {
        return "_No gate checks available._".to_string();
    };
    if checks.is_empty() {
        return "_No gate checks available._".to_string();
    }
    let rows: Vec<Vec<String>> = checks.iter().map(|check| gate_check_row(check)).collect();
    markdown_table(&["Check", "Value", "Threshold", "Result"], &rows)
}

fn gate_check_row(check: &GateCheck) -> Vec<String> {
    vec![
        sanitize_gate_name(check.name.as_deref()),
        format_number(check.value),
        format_number(check.limit),
        if check.ok.unwrap_or(false) {
            "✅ Pass".to_string()
        } else {
            "❌ Fail".to_string()
        },
    ]
}

fn render_stage_details(stages: Option<&StageMetrics>) -> Option<String> {
    let stages = stages?;
    let rows = non_zero_stage_rows(stages);
    if rows.is_empty() {
        return None;
    }
    let table = markdown_table(&["Stage", "p99 (ms)"], &rows);
    Some(format!(
        "<details>\n<summary>Stage breakdown (p99)</summary>\n\n{table}\n</details>"
    ))
}

fn non_zero_stage_rows(stages: &StageMetrics) -> Vec<Vec<String>> {
    let entries = [
        ("Auth LRU", stages.auth_lru_p99_ms),
        ("Auth gRPC", stages.auth_grpc_p99_ms),
        ("Rate limit", stages.rate_limit_p99_ms),
        ("Directive resolve", stages.directive_resolve_p99_ms),
        ("Prompt inject", stages.prompt_inject_p99_ms),
        ("Total overhead", stages.total_overhead_p99_ms),
    ];
    entries
        .into_iter()
        .filter_map(|(name, value)| {
            value
                .filter(|ms| *ms >= STAGE_MIN_MS)
                .map(|ms| vec![name.to_string(), format_number(Some(ms))])
        })
        .collect()
}

fn render_microbench_details(run: &BenchmarkRun) -> String {
    let bench = run
        .go_benchmarks
        .as_ref()
        .and_then(|value| value.get("BenchmarkProxyOverhead"));
    if bench.is_none() {
        return String::new();
    }
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
    let ci = match (low, high) {
        (Some(low), Some(high)) => format!(
            "[{} - {}]",
            format_number_precise(Some(low), 0),
            format_number_precise(Some(high), 0)
        ),
        _ => "—".to_string(),
    };

    format!(
        "<details>\n<summary><b>🔬 Microbenchmarks (Go)</b></summary>\n\n{}\n</details>",
        markdown_table(
            &["Metric", "Value", "95% CI"],
            &[
                vec!["ns/op".to_string(), format_number_precise(ns, 0), ci],
                vec![
                    "allocs/op".to_string(),
                    format_number_precise(allocs, 1),
                    "—".to_string(),
                ],
                vec![
                    "bytes/op".to_string(),
                    format_number_precise(bytes, 0),
                    "—".to_string(),
                ],
            ],
        )
    )
}

fn render_env_details(run: &BenchmarkRun, baseline_sha: Option<&str>) -> String {
    let runner = match (
        run.runner_os.as_deref(),
        run.runner_vcpus,
        run.runner_ram_gb,
    ) {
        (Some(os), Some(vcpus), Some(ram)) => format!("{os} ({vcpus} vCPU, {ram}GB RAM)"),
        (Some(os), _, _) => os.to_string(),
        _ => "—".to_string(),
    };
    let baseline_branch = baseline_sha
        .map(|sha| sanitize_sha(Some(sha)))
        .filter(|sha| sha != "invalid" && sha != "unknown")
        .map(|sha| format!("`{sha}`"))
        .unwrap_or_else(|| "`main`".to_string());

    format!(
        "<details>\n<summary><b>⚙️ Environment</b></summary>\n\n* **Go version:** {}\n* **Runner:** {}\n* **k6 version:** {}\n* **Baseline:** {}\n</details>",
        escape_cell(run.go_version.as_deref()),
        escape_cell(Some(&runner)),
        escape_cell(run.k6_version.as_deref()),
        baseline_branch,
    )
}

fn count_gate_failures(gate: &GateResult) -> usize {
    gate.checks
        .as_ref()
        .map(|checks| {
            checks
                .iter()
                .filter(|check| !check.ok.unwrap_or(false))
                .count()
        })
        .unwrap_or(0)
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
