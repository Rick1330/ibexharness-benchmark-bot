<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/ibex-mark-dark.png">
    <img alt="IBEX Harness Benchmark Bot" src="docs/brand/ibex-mark-light.png" width="96" height="96">
  </picture>
</p>

<h1 align="center">IBEX Harness Benchmark Bot</h1>

<p align="center">
GitHub App (Rust) · publishes benchmark data to <a href="https://github.com/Rick1330/ibex-harness">ibex-harness</a> · posts branded PR comments
</p>

| Command | Purpose |
| --- | --- |
| `verify-dispatch` | Re-verify dispatch payload via Actions API |
| `publish` | Validate artifact and open data PR |
| `post-pr-comment` | Post rich benchmark comment |

[Setup](docs/APP_SETUP.md) · [Runbook](docs/RUNBOOK.md) · [Threat model](docs/THREAT_MODEL.md)

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --all
```

Pin `BOT_RELEASE_SHA` and harness `BENCHMARK_BOT_SHA` to the same reviewed commit.
