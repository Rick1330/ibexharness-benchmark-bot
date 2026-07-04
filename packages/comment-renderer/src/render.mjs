import {
  escapeCell,
  formatDelta,
  formatNumber,
  markdownTable,
  sanitizeBranch,
  sanitizeSha,
  statusEmoji,
} from "./sanitize.mjs";

const DOCS_BASE = "https://docs.ibexharness.com/benchmarks/history";

export function renderPrComment(benchmarkData, gateResult) {
  const run = benchmarkData?.runs?.[0];
  if (!run) {
    throw new Error("benchmark data has no runs");
  }

  const shortSha = sanitizeSha(run.short_sha ?? run.sha?.slice(0, 7) ?? "unknown");
  const branch = sanitizeBranch(run.branch ?? "unknown");
  const status = String(run.status ?? "unknown");
  const emoji = statusEmoji(status);
  const runNumber = run.run_number ?? "?";
  const delta = run.regression_vs_baseline_pct;
  const k6 = run.k6 ?? {};
  const stages = run.stages ?? {};
  const goBench = run.go_benchmarks?.BenchmarkProxyOverhead ?? {};

  const sections = [
    `## Benchmark Results — Run #${runNumber}`,
    "",
    `**Status:** ${emoji} **${status.toUpperCase()}** | Commit: \`${shortSha}\` | Branch: \`${branch}\` | [View dashboard →](${DOCS_BASE}/${shortSha})`,
    "",
    "### Regression gate",
    "",
    renderGateTable(gateResult),
    "",
    "### Load test (k6)",
    "",
    markdownTable(
      ["Metric", "Value", "Delta vs baseline"],
      [
        ["p50", `${formatNumber(k6.p50_ms)} ms`, "—"],
        ["p95", `${formatNumber(k6.p95_ms)} ms`, "—"],
        ["p99", `${formatNumber(k6.p99_ms)} ms`, formatDelta(delta)],
        ["p999", `${formatNumber(k6.p999_ms)} ms`, "—"],
        ["Throughput", `${formatNumber(k6.req_per_s, 2)} req/s`, "—"],
        ["Error rate", `${formatNumber((k6.error_rate ?? 0) * 100, 3)}%`, "—"],
        ["Check rate", `${formatNumber((k6.check_rate ?? 0) * 100, 1)}%`, "—"],
      ],
    ),
    "",
    "### Stage breakdown (p99)",
    "",
    markdownTable(
      ["Stage", "p99 (ms)"],
      [
        ["Auth LRU", formatNumber(stages.auth_lru_p99_ms)],
        ["Auth gRPC", formatNumber(stages.auth_grpc_p99_ms)],
        ["Rate limit", formatNumber(stages.rate_limit_p99_ms)],
        ["Directive resolve", formatNumber(stages.directive_resolve_p99_ms)],
        ["Prompt inject", formatNumber(stages.prompt_inject_p99_ms)],
        ["Total overhead", formatNumber(stages.total_overhead_p99_ms)],
      ],
    ),
    "",
    renderStageMermaid(stages),
    "",
    "### Go microbench (BenchmarkProxyOverhead)",
    "",
    markdownTable(
      ["Metric", "Value"],
      [
        ["ns/op", formatNumber(goBench.ns_per_op, 0)],
        ["allocs/op", formatNumber(goBench.allocs_per_op, 1)],
        ["bytes/op", formatNumber(goBench.bytes_per_op, 0)],
        ["95% CI low", formatNumber(goBench.ci_95_low, 0)],
        ["95% CI high", formatNumber(goBench.ci_95_high, 0)],
      ],
    ),
    "",
    "<details>",
    "<summary>Environment</summary>",
    "",
    markdownTable(
      ["Field", "Value"],
      [
        ["Go version", escapeCell(run.go_version)],
        ["Runner OS", escapeCell(run.runner_os)],
        ["Runner CPU", escapeCell(run.runner_cpu)],
        ["vCPUs", escapeCell(run.runner_vcpus)],
        ["RAM (GB)", escapeCell(run.runner_ram_gb ?? "—")],
        ["k6 version", escapeCell(run.k6_version ?? "—")],
      ],
    ),
    "",
    "</details>",
    "",
    "> Regression threshold: >10% degradation on proxy p99 fails CI.",
  ];

  return sections.join("\n");
}

function renderGateTable(gateResult) {
  const checks = gateResult?.checks;
  if (!Array.isArray(checks) || checks.length === 0) {
    return "_No gate checks available._";
  }
  return markdownTable(
    ["Check", "Value", "Limit", "Result"],
    checks.map((check) => [
      check.name ?? "—",
      formatNumber(check.value, 6),
      formatNumber(check.limit, 6),
      check.ok ? "PASS" : "FAIL",
    ]),
  );
}

function renderStageMermaid(stages) {
  const entries = [
    ["Auth LRU", stages.auth_lru_p99_ms],
    ["Auth gRPC", stages.auth_grpc_p99_ms],
    ["Rate limit", stages.rate_limit_p99_ms],
    ["Directive", stages.directive_resolve_p99_ms],
    ["Prompt", stages.prompt_inject_p99_ms],
    ["Total", stages.total_overhead_p99_ms],
  ].filter(([, value]) => typeof value === "number");

  if (entries.length === 0) {
    return "";
  }

  const lines = ["```mermaid", "xychart-beta", '    title "Stage p99 (ms)"', "    x-axis [" + entries.map(([name]) => name).join('", "') + '"]', "    y-axis \"ms\"", "    bar [" + entries.map(([, v]) => formatNumber(v)).join(", ") + "]", "```"];
  return lines.join("\n");
}

export function renderDataPrBody(benchmarkData, context) {
  const run = benchmarkData?.runs?.[0] ?? {};
  const runNumber = context?.run_number ?? run.run_number ?? "?";
  const runUrl = context?.run_url ?? run.run_url ?? "";
  const shortSha = sanitizeSha(run.short_sha ?? "unknown");

  return [
    "## Automated benchmark data update",
    "",
    `**Status:** ${statusEmoji(run.status)} **${String(run.status ?? "unknown").toUpperCase()}**`,
    "",
    markdownTable(
      ["Field", "Value"],
      [
        ["Run number", runNumber],
        ["Head SHA", shortSha],
        ["Proxy p99", `${formatNumber(run.k6?.p99_ms)} ms`],
        ["Throughput", `${formatNumber(run.k6?.req_per_s, 2)} req/s`],
        ["Regression vs baseline", formatDelta(run.regression_vs_baseline_pct)],
      ],
    ),
    "",
    runUrl ? `- [Harness benchmark workflow run](${runUrl})` : "",
    "",
    "### Reviewer checklist",
    "",
    "- [ ] Validation passed in bot workflow",
    "- [ ] Harness CI green",
    "- [ ] Docs preview shows updated history",
  ]
    .filter(Boolean)
    .join("\n");
}
