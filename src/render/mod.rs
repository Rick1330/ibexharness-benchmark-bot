mod sanitize;

use crate::model::{
    BenchmarkData, BenchmarkRun, GateCheck, GateResult, HnswBenchmarkData, HnswSizeResult,
    StageMetrics,
};
pub use sanitize::{
    escape_cell, format_delta, format_latency_delta, format_latency_ms, format_ns_per_op,
    format_number, format_throughput, format_throughput_delta, sanitize_branch, sanitize_gate_name,
    sanitize_sha, status_emoji, COMMENT_MARKER, COMMENT_MARKER_HNSW, ENV_SECTION_END,
    ENV_SECTION_START, HNSW_SECTION_END, HNSW_SECTION_START, PROXY_SECTION_END,
    PROXY_SECTION_START, SUITE_META_PREFIX,
};

const DOCS_BASE: &str = "https://docs.ibexharness.com/benchmarks";
const HARNESS_REPO: &str = "https://github.com/Rick1330/ibex-harness";
const BRAND_MARK_LIGHT: &str =
    "https://raw.githubusercontent.com/Rick1330/ibexharness-benchmark-bot/main/docs/brand/ibex-mark-light.png";
const BRAND_MARK_DARK: &str =
    "https://raw.githubusercontent.com/Rick1330/ibexharness-benchmark-bot/main/docs/brand/ibex-mark-dark.png";

const BRAND_NAME: &str = "IBEX Benchmark Bot";
const BRAND_LOGO_PX: u32 = 32;
const P99_SLA_MS: f64 = 20.0;

#[derive(Clone, Debug)]
struct SuiteMeta {
    id: String,
    name: String,
    primary: String,
    delta: String,
    status: String,
}

pub fn render_pr_comment(data: &BenchmarkData, gate: &GateResult) -> Result<String, String> {
    merge_proxy_into_comment("", data, gate)
}

/// Merge the Proxy suite into the shared sticky comment (preserves Memory HNSW).
pub fn merge_proxy_into_comment(
    existing: &str,
    data: &BenchmarkData,
    gate: &GateResult,
) -> Result<String, String> {
    let section = render_proxy_section(data, gate)?;
    let env = render_env_section(data);
    let mut body = ensure_comment_shell(existing);
    body = upsert_section(&body, PROXY_SECTION_START, PROXY_SECTION_END, &section);
    if let Some(env) = env {
        body = upsert_section(&body, ENV_SECTION_START, ENV_SECTION_END, &env);
    }
    Ok(rebuild_comment_shell(&body))
}

pub fn render_data_pr_body(
    data: &BenchmarkData,
    run_url: Option<&str>,
    run_number: Option<i64>,
) -> String {
    render_combined_data_pr_body(CombinedDataPrInput {
        proxy: Some(data),
        proxy_run_url: run_url,
        proxy_run_number: run_number,
        hnsw: None,
        hnsw_run_url: None,
        hnsw_run_number: None,
    })
}

/// Inputs for a single shared data PR that may carry proxy and/or HNSW suites.
pub struct CombinedDataPrInput<'a> {
    pub proxy: Option<&'a BenchmarkData>,
    pub proxy_run_url: Option<&'a str>,
    pub proxy_run_number: Option<i64>,
    pub hnsw: Option<&'a HnswBenchmarkData>,
    pub hnsw_run_url: Option<&'a str>,
    pub hnsw_run_number: Option<i64>,
}

pub fn render_combined_data_pr_body(input: CombinedDataPrInput<'_>) -> String {
    let mut suites: Vec<&str> = Vec::new();
    if input.proxy.is_some() {
        suites.push("Proxy");
    }
    if input.hnsw.is_some() {
        suites.push("Memory HNSW");
    }
    let suites_list = if suites.is_empty() {
        "_none_".to_string()
    } else {
        suites.join(", ")
    };

    let mut lines = vec![
        render_compact_brand(),
        String::new(),
        "## What and Why".to_string(),
        String::new(),
        format!(
            "Automated publish of public benchmark history for ibex-harness. \
             Suites in this PR: **{suites_list}**."
        ),
        String::new(),
        "Keeps `/benchmarks` and `/benchmarks/memory` history current without \
         contributor-authored commits."
            .to_string(),
        String::new(),
        "## Tracking issue".to_string(),
        String::new(),
        "N/A (GitHub App data publish)".to_string(),
        String::new(),
        "## How".to_string(),
        String::new(),
    ];

    if let Some(data) = input.proxy {
        lines.push(render_proxy_data_section(
            data,
            input.proxy_run_url,
            input.proxy_run_number,
        ));
        lines.push(String::new());
    }
    if let Some(data) = input.hnsw {
        lines.push(render_hnsw_data_section(
            data,
            input.hnsw_run_url,
            input.hnsw_run_number,
        ));
        lines.push(String::new());
    }

    lines.push("## Testing".to_string());
    lines.push(String::new());
    lines.push("- [ ] Bot workflow validation passed (suite(s) above)".to_string());
    lines.push("- [ ] Harness CI green on this PR".to_string());
    lines.push("- [ ] `/benchmarks` and/or `/benchmarks/memory` preview updated".to_string());
    lines.push(String::new());
    lines.push("## Performance".to_string());
    lines.push(String::new());
    lines.push(
        "No runtime change — history JSON / badge only. Review suite tables above for SLA deltas."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("## Security".to_string());
    lines.push(String::new());
    lines.push(
        "No secrets; App-signed commits; artifacts re-verified via Actions API before publish."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("## Migrations / Ops".to_string());
    lines.push(String::new());
    lines.push("None.".to_string());
    lines.push(String::new());
    lines.push("## Docs".to_string());
    lines.push(String::new());
    lines.push(
        "Public benchmark history only (`web/public/benchmarks/*`). Suite JSON files stay separate."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("## Checklist (Definition of Done)".to_string());
    lines.push(String::new());
    lines.push("- [ ] Correct files only (proxy JSON+badge and/or HNSW JSON)".to_string());
    lines.push("- [ ] Labels match files (`benchmark-data` / `hnsw-benchmark-data`)".to_string());
    lines.push("- [ ] Single shared data PR branch `chore/bench-data-publish`".to_string());
    lines.join("\n")
}

fn render_proxy_data_section(
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
        "`{}`",
        format_latency_ms(run.and_then(|r| r.k6.as_ref()).and_then(|k| k.p99_ms))
    );
    let proxy_overhead = run
        .and_then(proxy_overhead_ns)
        .map(|ns| format!("`{}`", format_ns_per_op(Some(ns))))
        .unwrap_or_else(|| "—".to_string());

    let mut lines = vec![
        "### Proxy suite".to_string(),
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
                vec!["Proxy p99 (k6)".to_string(), p99],
                vec!["Proxy overhead (Go)".to_string(), proxy_overhead],
                vec![
                    "Paths".to_string(),
                    format!(
                        "`{}`, `{}`",
                        crate::config::BENCHMARK_DATA_PATH,
                        crate::config::BADGE_PATH
                    ),
                ],
            ],
        ),
        String::new(),
        "_k6 p99 is the authoritative SLA metric. Stage breakdown and ns/op values are \
         synthetic Go microbenchmarks._"
            .to_string(),
    ];
    if let Some(url) = run_url {
        lines.push(String::new());
        lines.push(format!("- [Harness Benchmarks workflow run]({url})"));
    }
    lines.join("\n")
}

/// Compact Memory HNSW section merged into the shared sticky comment.
pub fn render_hnsw_pr_comment(data: &HnswBenchmarkData) -> Result<String, String> {
    merge_hnsw_into_comment("", data)
}

/// Merge the Memory HNSW suite into the shared sticky comment (preserves Proxy).
pub fn merge_hnsw_into_comment(existing: &str, data: &HnswBenchmarkData) -> Result<String, String> {
    let section = render_hnsw_section(data)?;
    let mut body = ensure_comment_shell(existing);
    body = upsert_section(&body, HNSW_SECTION_START, HNSW_SECTION_END, &section);
    Ok(rebuild_comment_shell(&body))
}

fn ensure_comment_shell(existing: &str) -> String {
    let normalized = existing.replace(COMMENT_MARKER_HNSW, COMMENT_MARKER);
    if normalized.contains(COMMENT_MARKER) {
        return normalized;
    }
    if normalized.trim().is_empty() {
        return COMMENT_MARKER.to_string();
    }
    format!("{COMMENT_MARKER}\n\n{normalized}")
}

fn upsert_section(body: &str, start: &str, end: &str, inner: &str) -> String {
    let block = format!("{start}\n{}\n{end}", inner.trim());
    if let Some(start_idx) = body.find(start) {
        if let Some(end_rel) = body[start_idx..].find(end) {
            let end_idx = start_idx + end_rel + end.len();
            return format!("{}{}{}", &body[..start_idx], block, &body[end_idx..]);
        }
    }
    format!("{}\n\n{block}\n", body.trim_end())
}

fn rebuild_comment_shell(body: &str) -> String {
    let metas = parse_suite_metas(body);
    let mut parts = vec![
        COMMENT_MARKER.to_string(),
        String::new(),
        render_comment_header(&metas),
    ];
    for (start, end) in [
        (PROXY_SECTION_START, PROXY_SECTION_END),
        (HNSW_SECTION_START, HNSW_SECTION_END),
        (ENV_SECTION_START, ENV_SECTION_END),
    ] {
        if let Some(block) = extract_section_block(body, start, end) {
            parts.push(String::new());
            parts.push(block);
        }
    }
    parts.push(String::new());
    parts.push(
        "<div align=\"right\">\n  <sub>Generated by IBEX Benchmark Bot</sub>\n</div>".to_string(),
    );
    parts.join("\n")
}

fn extract_section_block(body: &str, start: &str, end: &str) -> Option<String> {
    let start_idx = body.find(start)?;
    let end_rel = body[start_idx..].find(end)?;
    let end_idx = start_idx + end_rel + end.len();
    Some(body[start_idx..end_idx].trim().to_string())
}

fn render_comment_header(metas: &[SuiteMeta]) -> String {
    let (title, tldr) = global_verdict(metas);
    let mut lines = vec![
        format!(r#"<p align="right"><a href="{DOCS_BASE}">View IBEX dashboard →</a></p>"#),
        String::new(),
        format!("### {title}"),
        format!("**TL;DR:** {tldr}"),
    ];
    if !metas.is_empty() {
        lines.push(String::new());
        lines.push("#### Suite matrix".to_string());
        lines.push(String::new());
        lines.push(render_suite_matrix(metas));
    }
    lines.join("\n")
}

fn global_verdict(metas: &[SuiteMeta]) -> (String, String) {
    if metas.is_empty() {
        return (
            "IBEX benchmarks".to_string(),
            "Waiting for suite results.".to_string(),
        );
    }
    let has_fail = metas.iter().any(|m| m.status == "fail");
    let has_regression = metas.iter().any(|m| m.status == "regression");
    let has_warn = metas.iter().any(|m| m.status == "warn");
    let all_pass = metas.iter().all(|m| m.status == "pass");

    if has_fail {
        return (
            "❌ Benchmarks failed".to_string(),
            "At least one suite failed its SLA gates. Expand the failing deep dive before merging."
                .to_string(),
        );
    }
    if has_regression {
        return (
            "⚠️ Benchmarks regressed".to_string(),
            "A suite crossed the regression threshold. Review the delta before merging."
                .to_string(),
        );
    }
    if has_warn {
        return (
            "⚠️ Benchmarks warning".to_string(),
            "At least one suite reported a warning. Expand the deep dive before merging."
                .to_string(),
        );
    }
    if all_pass {
        let proxy = metas.iter().find(|m| m.id == "proxy");
        let tldr = match proxy.map(|m| m.delta.as_str()) {
            Some(delta) if delta != "—" && delta != "n/a" => {
                format!("All collected suites passed. Proxy P99 vs baseline: {delta}.")
            }
            _ => "All collected suites passed.".to_string(),
        };
        return ("✅ All benchmarks passed".to_string(), tldr);
    }
    (
        "IBEX benchmarks".to_string(),
        "Suite results updated; status is incomplete or unknown.".to_string(),
    )
}

fn render_suite_matrix(metas: &[SuiteMeta]) -> String {
    let mut ordered = metas.to_vec();
    ordered.sort_by_key(|m| suite_sort_key(&m.id));
    let rows: Vec<Vec<String>> = ordered
        .iter()
        .map(|m| {
            vec![
                format!("**{}**", m.name),
                m.primary.clone(),
                m.delta.clone(),
                format!("{} {}", status_emoji(&m.status), m.status.to_uppercase()),
            ]
        })
        .collect();
    markdown_table_raw(&["Suite", "Primary SLA", "vs Baseline", "Status"], &rows)
}

fn suite_sort_key(id: &str) -> u8 {
    match id {
        "proxy" => 0,
        "hnsw" => 1,
        _ => 9,
    }
}

fn encode_suite_meta(meta: &SuiteMeta) -> String {
    format!(
        "{SUITE_META_PREFIX}{}|{}|{}|{}|{} -->",
        scrub_meta(&meta.id),
        scrub_meta(&meta.name),
        scrub_meta(&meta.primary),
        scrub_meta(&meta.delta),
        scrub_meta(&meta.status)
    )
}

fn scrub_meta(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if c == '|' { '/' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

fn normalize_status(raw: &str) -> &'static str {
    match raw {
        "pass" => "pass",
        "fail" => "fail",
        "warn" => "warn",
        "regression" => "regression",
        _ => "unknown",
    }
}

fn parse_suite_metas(body: &str) -> Vec<SuiteMeta> {
    let mut metas = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix(SUITE_META_PREFIX) else {
            continue;
        };
        let Some(rest) = rest.strip_suffix(" -->") else {
            continue;
        };
        let parts: Vec<&str> = rest.splitn(5, '|').collect();
        if parts.len() != 5 {
            continue;
        }
        metas.push(SuiteMeta {
            id: parts[0].to_string(),
            name: parts[1].to_string(),
            primary: parts[2].to_string(),
            delta: parts[3].to_string(),
            status: normalize_status(parts[4]).to_string(),
        });
    }
    metas.sort_by_key(|m| suite_sort_key(&m.id));
    metas.dedup_by(|a, b| a.id == b.id);
    metas
}

fn details_open_attr(status: &str) -> &'static str {
    match status {
        "fail" | "regression" | "warn" => " open",
        _ => "",
    }
}

fn render_proxy_section(data: &BenchmarkData, gate: &GateResult) -> Result<String, String> {
    let run = data
        .runs
        .as_ref()
        .and_then(|runs| runs.first())
        .ok_or_else(|| "benchmark data has no runs".to_string())?;
    let derived = if count_gate_failures(gate) > 0 {
        "fail"
    } else if matches!(gate.status.as_deref(), Some("fail") | Some("warn")) {
        gate.status.as_deref().unwrap_or("unknown")
    } else {
        run.status.as_deref().unwrap_or("unknown")
    };
    let status = normalize_status(derived);
    let p99 = format_latency_ms(run.k6.as_ref().and_then(|k| k.p99_ms));
    let throughput = format_throughput(run.k6.as_ref().and_then(|k| k.req_per_s));
    let error_rate = sanitize::format_number_precise(
        run.k6
            .as_ref()
            .and_then(|k| k.error_rate)
            .map(|v| v * 100.0),
        2,
    );
    let delta = format_latency_delta(run.regression_vs_baseline_pct);
    let meta = SuiteMeta {
        id: "proxy".into(),
        name: "Proxy".into(),
        primary: format!("`{p99}` (P99)"),
        delta,
        status: status.to_string(),
    };

    let mut body = vec![
        encode_suite_meta(&meta),
        format!(
            "<details{}>\n<summary>Deep Dive: Proxy</summary>\n<br>\n",
            details_open_attr(status)
        ),
        "**Load test (k6)**".to_string(),
        format!("* **P99:** `{p99}` *(SLA < {P99_SLA_MS} ms)*"),
        format!("* **Throughput:** `{throughput}`"),
        format!("* **Error rate:** `{error_rate}%`"),
        String::new(),
        "**Synthetic overhead (Go)**".to_string(),
    ];
    if let Some(ns) = proxy_overhead_ns(run) {
        body.push(format!("* **Total:** `{}`", format_ns_per_op(Some(ns))));
    } else {
        body.push("* **Total:** —".to_string());
    }
    if let Some(stages) = render_stage_chips(run.stages.as_ref()) {
        body.push(format!("* **Stages:** {stages}"));
    }
    let failures = count_gate_failures(gate);
    if failures > 0 {
        body.push(String::new());
        body.push(format!("**Failed gates ({failures})**"));
        body.push(String::new());
        body.push(render_failed_gates_only(gate));
    }
    body.push(String::new());
    body.push(
        "_k6 P99 is the SLA. Stage / ns/op values are synthetic Go microbenchmarks, not live traces._"
            .to_string(),
    );
    body.push("</details>".to_string());
    Ok(body.join("\n"))
}

fn render_stage_chips(stages: Option<&StageMetrics>) -> Option<String> {
    let stages = stages?;
    let chips: Vec<String> = [
        ("Auth LRU", stages.auth_lru_p99_ms),
        ("gRPC", stages.auth_grpc_p99_ms),
        ("Rate limit", stages.rate_limit_p99_ms),
        ("Directive", stages.directive_resolve_p99_ms),
        ("Prompt", stages.prompt_inject_p99_ms),
    ]
    .into_iter()
    .filter_map(|(name, value)| {
        value
            .filter(|ms| *ms > 0.0)
            .map(|ms| format!("{name} `{}`", format_latency_ms(Some(ms))))
    })
    .collect();
    if chips.is_empty() {
        None
    } else {
        Some(chips.join(" · "))
    }
}

fn render_failed_gates_only(gate: &GateResult) -> String {
    let Some(checks) = gate.checks.as_ref() else {
        return "_No gate checks available._".to_string();
    };
    let rows: Vec<Vec<String>> = checks
        .iter()
        .filter(|check| !check.ok.unwrap_or(false))
        .map(gate_check_row)
        .collect();
    if rows.is_empty() {
        return "_No failed gates._".to_string();
    }
    markdown_table(&["Check", "Value", "Threshold", "Result"], &rows)
}

fn render_hnsw_section(data: &HnswBenchmarkData) -> Result<String, String> {
    let run = data
        .runs
        .as_ref()
        .and_then(|runs| runs.first())
        .ok_or_else(|| "hnsw benchmark data has no runs".to_string())?;
    let status = normalize_status(run.status.as_deref().unwrap_or("unknown"));
    let mean = run
        .mean_recall_at_10
        .map(|v| format!("{v:.3}"))
        .unwrap_or_else(|| "n/a".into());
    let deferred_1m = run
        .gate_summary
        .as_ref()
        .and_then(|g| g.get("has_1m"))
        .and_then(|v| v.as_bool())
        == Some(false);
    let meta = SuiteMeta {
        id: "hnsw".into(),
        name: "Memory HNSW".into(),
        primary: format!("`{mean}` (Recall@10)"),
        delta: "—".into(),
        status: status.to_string(),
    };

    let mut body = vec![
        encode_suite_meta(&meta),
        format!(
            "<details{}>\n<summary>Deep Dive: Memory HNSW</summary>\n<br>\n",
            details_open_attr(status)
        ),
    ];
    for result in run.results.as_deref().unwrap_or(&[]) {
        let size = result
            .corpus_size
            .map(format_corpus_size)
            .unwrap_or_else(|| "—".into());
        let recall = result
            .recall_at_10
            .map(|v| format!("`{v:.3}`"))
            .unwrap_or_else(|| "`n/a`".into());
        let p95 = result
            .latency_ms_p95
            .map(|v| format!("`{v:.1} ms`"))
            .unwrap_or_else(|| "`n/a`".into());
        body.push(format!("* **{size}:** Recall {recall} · p95 {p95}"));
    }
    if deferred_1m {
        body.push("* **1M:** Deferred *(smoke/fast — Sunday/full only)*".to_string());
    }
    body.push("* **Knobs:** `ef=40`, `min_sim=0.70`, `iter=off`, bulk index".to_string());
    if let Some(note) = run
        .gate_summary
        .as_ref()
        .and_then(|g| g.get("note"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        body.push(format!("\n> {}", escape_cell(Some(note))));
    }
    body.push("</details>".to_string());
    Ok(body.join("\n"))
}

fn render_env_section(data: &BenchmarkData) -> Option<String> {
    let run = data.runs.as_ref().and_then(|runs| runs.first())?;
    let baseline_sha = data.baseline_sha.as_deref();
    Some(render_env_details(run, baseline_sha))
}

fn format_corpus_size(size: i64) -> String {
    if size >= 1_000_000 {
        format!("{}M", size / 1_000_000)
    } else if size >= 1_000 {
        format!("{}K", size / 1_000)
    } else {
        size.to_string()
    }
}

/// Branded body for automated HNSW / memory data PRs (mirrors proxy data PR shape).
pub fn render_hnsw_data_pr_body(
    data: &HnswBenchmarkData,
    run_url: Option<&str>,
    run_number: i64,
) -> String {
    render_combined_data_pr_body(CombinedDataPrInput {
        proxy: None,
        proxy_run_url: None,
        proxy_run_number: None,
        hnsw: Some(data),
        hnsw_run_url: run_url,
        hnsw_run_number: Some(run_number),
    })
}

fn render_hnsw_data_section(
    data: &HnswBenchmarkData,
    run_url: Option<&str>,
    run_number: Option<i64>,
) -> String {
    let latest = data.runs.as_ref().and_then(|runs| runs.first());
    let short_sha = latest
        .and_then(|run| run.short_sha.as_deref().or(run.sha.as_deref()))
        .map(|sha| sanitize_sha(Some(sha)))
        .unwrap_or_else(|| sanitize_sha(None));
    let mean = latest
        .and_then(|run| run.mean_recall_at_10)
        .map(|v| format!("`{v:.4}`"))
        .unwrap_or_else(|| "`n/a`".into());
    let number = run_number
        .or(latest.and_then(|r| r.run_number))
        .map(|v| v.to_string())
        .unwrap_or_else(|| "?".to_string());

    let mut lines = vec![
        "### Memory HNSW suite".to_string(),
        String::new(),
        markdown_table(
            &["Field", "Value"],
            &[
                vec!["Run number".to_string(), number],
                vec!["Head SHA".to_string(), short_sha],
                vec!["Mean recall@10".to_string(), mean],
                vec![
                    "Path".to_string(),
                    format!("`{}`", crate::config::HNSW_BENCHMARK_DATA_PATH),
                ],
            ],
        ),
        String::new(),
        "#### Corpus cells".to_string(),
        String::new(),
        render_hnsw_cells_table(latest.and_then(|run| run.results.as_deref()).unwrap_or(&[])),
        String::new(),
        "_Production-like knobs expected (`ef_search=40`, `min_similarity≈0.70`, \
         `iterative_scan=off`, bulk index build)._"
            .to_string(),
    ];
    if let Some(url) = run_url {
        lines.push(String::new());
        lines.push(format!("- [Harness Memory Benchmarks workflow run]({url})"));
    }
    lines.join("\n")
}

fn render_hnsw_cells_table(results: &[HnswSizeResult]) -> String {
    if results.is_empty() {
        return "_No result cells in latest run._".to_string();
    }
    let rows: Vec<Vec<String>> = results
        .iter()
        .map(|r| {
            vec![
                r.corpus_size
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "—".into()),
                r.recall_at_10
                    .map(|v| format!("{v:.4}"))
                    .unwrap_or_else(|| "—".into()),
                r.latency_ms_p95
                    .map(|v| format!("{v:.1}ms"))
                    .unwrap_or_else(|| "—".into()),
                r.ef_search
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".into()),
                r.min_similarity
                    .map(|v| format!("{v:.2}"))
                    .unwrap_or_else(|| "—".into()),
                r.iterative_scan.clone().unwrap_or_else(|| "—".into()),
                r.index_build_mode.clone().unwrap_or_else(|| "—".into()),
            ]
        })
        .collect();
    markdown_table(
        &[
            "Size",
            "Recall@10",
            "p95",
            "ef",
            "min_sim",
            "iterative",
            "build",
        ],
        &rows,
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

fn proxy_overhead_ns(run: &BenchmarkRun) -> Option<f64> {
    run.go_benchmarks
        .as_ref()
        .and_then(|value| value.get("BenchmarkProxyOverhead"))
        .and_then(|v| v.get("ns_per_op"))
        .and_then(|v| v.as_f64())
        .filter(|ns| *ns > 0.0)
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
    let baseline = baseline_sha
        .map(|sha| sanitize_sha(Some(sha)))
        .filter(|sha| sha != "invalid" && sha != "unknown")
        .map(|sha| format!("[`{sha}`]({HARNESS_REPO}/commit/{sha})"))
        .unwrap_or_else(|| "`main`".to_string());

    format!(
        "<details>\n<summary>Environment & meta</summary>\n<br>\n\n\
         * **Runner:** {runner} · **Go:** `{go}` · **k6:** `{k6}`\n\
         * **Baseline:** {baseline}\n\
         </details>",
        go = escape_cell(run.go_version.as_deref()),
        k6 = escape_cell(run.k6_version.as_deref()),
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
    markdown_table_with_escape(headers, rows, true)
}

fn markdown_table_raw(headers: &[&str], rows: &[Vec<String>]) -> String {
    markdown_table_with_escape(headers, rows, false)
}

fn markdown_table_with_escape(headers: &[&str], rows: &[Vec<String>], escape_body: bool) -> String {
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
            .map(|cell| {
                if escape_body {
                    escape_cell(Some(cell.as_str()))
                } else {
                    cell.clone()
                }
            })
            .collect();
        lines.push(format!("| {} |", cells.join(" | ")));
    }
    lines.join("\n")
}
