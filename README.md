# IBEX Harness Benchmark Bot

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/ibex-mark-dark.png">
    <img alt="IBEX Harness" src="docs/brand/ibex-mark-light.png" width="120">
  </picture>
</p>

External **Rust** GitHub App that publishes validated benchmark data to [ibex-harness](https://github.com/Rick1330/ibex-harness) `main` via pull request, and renders rich PR benchmark comments.

**Harness:** [Rick1330/ibex-harness](https://github.com/Rick1330/ibex-harness)  
**ADR:** [ADR-0024](https://github.com/Rick1330/ibex-harness/blob/main/docs/app/content/docs/adr/0024-benchmark-data-publishing-model.mdx)  
**Security:** [THREAT_MODEL.md](docs/THREAT_MODEL.md)

## Architecture

Single Rust binary (`ibex-benchmark-bot`) with subcommands:

| Command | Purpose |
| --- | --- |
| `verify-dispatch` | Re-verify `repository_dispatch` payload via Actions API |
| `publish` | Download artifact, validate, open data PR on harness |
| `render-pr-comment` | JSON → rich markdown (stdout) |
| `post-pr-comment` | Render + post PR comment (used by harness CI) |

Deployment: **GitHub Actions only** ($0). No JavaScript, no Python runtime in the bot repo.

## Security model

- Dispatch payloads are **untrusted**; every publish re-verifies the harness workflow run via the GitHub Actions API.
- Publish workflows build only from pinned `BOT_RELEASE_SHA` (never floating `main`).
- Artifact JSON is schema-validated, cross-checked against the verified run, and replay-protected against stale `run_number` / duplicate SHAs.
- See [THREAT_MODEL.md](docs/THREAT_MODEL.md) for full threat analysis.

## Setup

1. Follow [`docs/APP_SETUP.md`](docs/APP_SETUP.md) — create GitHub App, store secrets.
2. Set harness `BENCHMARK_BOT_ENABLED=true` and `BENCHMARK_BOT_DISPATCH_TOKEN`.
3. Pin harness variable `BENCHMARK_BOT_SHA` and bot variable `BOT_RELEASE_SHA` to the same reviewed commit.

## Development

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
cargo build --release
```

## Repository layout

```text
src/
  artifact/   # Safe zip extract + badge.svg validation
  config/     # Harness repo lock + path constants
  github/     # App JWT, GitHub API client
  render/     # PR + data-PR markdown sanitization
  validate/   # benchmark-data.json validation
  verify/     # dispatch verification
  publish/    # artifact download + PR creation
docs/
  brand/      # IBEX Harness mark assets
  APP_SETUP.md
  THREAT_MODEL.md
  RUNBOOK.md
```
