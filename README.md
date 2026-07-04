# ibexharness-benchmark-bot

Rust GitHub App that publishes validated benchmark data to [ibex-harness](https://github.com/Rick1330/ibex-harness) and posts branded PR benchmark comments.

| Command | Purpose |
| --- | --- |
| `verify-dispatch` | Re-verify dispatch payload via Actions API |
| `publish` | Validate artifact and open data PR |
| `post-pr-comment` | Post rich benchmark comment (App identity when App secrets are set) |

Setup: [docs/APP_SETUP.md](docs/APP_SETUP.md) · Operations: [docs/RUNBOOK.md](docs/RUNBOOK.md)

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --all
```

Pin `BOT_RELEASE_SHA` (this repo) and `BENCHMARK_BOT_SHA` (harness) to the same reviewed commit.
