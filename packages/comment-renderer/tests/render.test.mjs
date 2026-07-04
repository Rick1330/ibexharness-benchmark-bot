import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { renderPrComment, renderDataPrBody } from "../src/render.mjs";
import { sanitizeSha } from "../src/sanitize.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const fixtures = path.join(__dirname, "fixtures");

test("sanitizeSha rejects invalid input", () => {
  assert.equal(sanitizeSha("not-a-sha"), "invalid");
  assert.equal(sanitizeSha("bfc0a75"), "bfc0a75");
});

test("renderPrComment includes gate checks and stages", () => {
  const benchmarkData = JSON.parse(
    fs.readFileSync(path.join(fixtures, "benchmark-data.json"), "utf8"),
  );
  const gateResult = JSON.parse(
    fs.readFileSync(path.join(fixtures, "gate-result.json"), "utf8"),
  );
  const body = renderPrComment(benchmarkData, gateResult);

  assert.match(body, /## Benchmark Results/);
  assert.match(body, /Regression gate/);
  assert.match(body, /Load test \(k6\)/);
  assert.match(body, /Stage breakdown/);
  assert.match(body, /Go microbench/);
  assert.match(body, /k6 p99 SLA/);
  assert.match(body, /```mermaid/);
  assert.doesNotMatch(body, /\|\s*invalid\s*\|/);
});

test("renderDataPrBody includes checklist", () => {
  const benchmarkData = JSON.parse(
    fs.readFileSync(path.join(fixtures, "benchmark-data.json"), "utf8"),
  );
  const body = renderDataPrBody(benchmarkData, {
    run_number: 16,
    run_url: "https://github.com/Rick1330/ibex-harness/actions/runs/1",
  });

  assert.match(body, /Reviewer checklist/);
  assert.match(body, /Run number/);
});
