<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/ibex-mark-dark.png">
    <img alt="IBEX Harness Benchmark Bot" src="docs/brand/ibex-mark-light.png" width="96" height="96">
  </picture>
</p>

<h1 align="center">IBEX Harness Benchmark Bot</h1>

<p align="center">
GitHub App (Rust) · publishes benchmark data to <a href="https://github.com/Rick1330/ibex-harness">ibex-harness</a> · posts branded, suite-aware PR comments
</p>

| Command | Purpose |
| --- | --- |
| `verify-dispatch` | Re-verify proxy Benchmarks dispatch via Actions API |
| `publish` | Validate proxy artifact; commit onto shared `chore/bench-data-publish`; open/update one data PR |
| `verify-hnsw-dispatch` | Re-verify Memory Benchmarks dispatch |
| `publish-hnsw` | Validate HNSW artifact; commit onto the **same** shared data PR (HNSW JSON only) |
| `post-pr-comment` | Upsert **Proxy** suite into the shared sticky comment (`IBEX_BOT_COMMENT` matrix + deep dive) |
| `render-pr-comment` | Render Proxy sticky comment to stdout |
| `post-hnsw-pr-comment` | Upsert **Memory HNSW** suite into the same sticky comment |
| `render-hnsw-pr-comment` | Render Memory HNSW sticky comment to stdout |

[Setup](docs/APP_SETUP.md) · [Runbook](docs/RUNBOOK.md) · [Threat model](docs/THREAT_MODEL.md)

## Contribution rules

- Open a **pull request** for every change. Do **not** push commits directly to `main`.
- Fill the [PR template](.github/pull_request_template.md) completely (What/Why, Testing, Ops/release).
- Merge only when CI (`validate`) is green.
- After merge, maintainers update pins + release tag together (see runbook). Never leave harness on a stale `BENCHMARK_BOT_RELEASE_TAG`.

```bash
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all
```

Pin `BOT_RELEASE_SHA` and harness `BENCHMARK_BOT_SHA` to the same reviewed commit on `main`.
Set harness `BENCHMARK_BOT_RELEASE_TAG` to `bot-<7-char-sha>` only after **Release binary** uploads `ibex-benchmark-bot-linux-amd64` and `ibex-benchmark-bot-linux-amd64.sha256` for that tag. Pin the digest in harness `.github/actions/setup-benchmark-bot/ibex-benchmark-bot-linux-amd64.sha256`.
