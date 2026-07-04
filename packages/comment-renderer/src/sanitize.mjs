/** Sanitize user-controlled strings for GitHub markdown. */

const SHA_PATTERN = /^[0-9a-f]{7,40}$/i;
const BRANCH_PATTERN = /^[a-zA-Z0-9._/-]{1,200}$/;

export function sanitizeSha(value) {
  if (typeof value !== "string") {
    return "unknown";
  }
  const trimmed = value.trim();
  if (!SHA_PATTERN.test(trimmed)) {
    return "invalid";
  }
  return trimmed.toLowerCase();
}

export function sanitizeBranch(value) {
  if (typeof value !== "string") {
    return "unknown";
  }
  const trimmed = value.trim();
  if (!BRANCH_PATTERN.test(trimmed)) {
    return "unknown";
  }
  return trimmed;
}

export function escapeCell(value) {
  if (value === null || value === undefined) {
    return "—";
  }
  return String(value).replace(/\|/g, "\\|").replace(/\r?\n/g, " ");
}

export function formatNumber(value, digits = 3) {
  if (typeof value !== "number" || Number.isNaN(value)) {
    return "—";
  }
  return value.toFixed(digits);
}

export function formatDelta(delta) {
  if (typeof delta !== "number" || Number.isNaN(delta)) {
    return "n/a";
  }
  const sign = delta > 0 ? "+" : "";
  return `${sign}${delta.toFixed(1)}%`;
}

export function statusEmoji(status) {
  if (status === "pass") {
    return "✅";
  }
  if (status === "regression") {
    return "⚠️";
  }
  if (status === "fail") {
    return "❌";
  }
  return "❔";
}

export function markdownTable(headers, rows) {
  const lines = [
    `| ${headers.map(escapeCell).join(" | ")} |`,
    `| ${headers.map(() => "---").join(" | ")} |`,
  ];
  for (const row of rows) {
    lines.push(`| ${row.map(escapeCell).join(" | ")} |`);
  }
  return lines.join("\n");
}
