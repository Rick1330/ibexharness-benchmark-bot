#!/usr/bin/env node
/**
 * CLI for benchmark markdown rendering.
 * Usage:
 *   node cli.mjs pr-comment
 *   node cli.mjs data-pr
 */
import fs from "node:fs";
import process from "node:process";
import { renderDataPrBody, renderPrComment } from "./src/render.mjs";

const DEFAULT_BENCHMARK = "benchmarks/output/benchmark-data.json";
const DEFAULT_GATE = "benchmarks/output/gate-result.json";

function readJson(path) {
  if (!fs.existsSync(path)) {
    console.error(`comment-renderer: file not found: ${path}`);
    process.exit(1);
  }
  return JSON.parse(fs.readFileSync(path, "utf8"));
}

function main() {
  const mode = process.argv[2];
  if (!mode || !["pr-comment", "data-pr"].includes(mode)) {
    console.error("usage: node cli.mjs <pr-comment|data-pr>");
    process.exit(1);
  }

  const benchmarkPath = process.env.BENCHMARK_DATA_PATH ?? DEFAULT_BENCHMARK;
  const gatePath = process.env.GATE_RESULT_PATH ?? DEFAULT_GATE;
  const benchmarkData = readJson(benchmarkPath);

  let body;
  if (mode === "pr-comment") {
    const gateResult = fs.existsSync(gatePath) ? readJson(gatePath) : { checks: [] };
    body = renderPrComment(benchmarkData, gateResult);
  } else {
    body = renderDataPrBody(benchmarkData, {
      run_number: process.env.RUN_NUMBER,
      run_url: process.env.RUN_URL,
    });
  }

  if (process.env.OUTPUT_PATH) {
    fs.writeFileSync(process.env.OUTPUT_PATH, body, "utf8");
  } else {
    process.stdout.write(body);
  }
}

main();
