<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/ibex-mark-dark.png">
    <img alt="IBEX Harness" src="docs/brand/ibex-mark-light.png" width="48" height="48">
  </picture>
</p>

<pre align="center">
 _____ ____  ________   __  _    _                                
|_   _|  _ \|  ____\ \ / / | |  | |                               
  | | | |_) | |__   \ V /  | |__| | __ _ _ __ _ __   ___  ___ ___ 
  | | |  _ <|  __|   > <   |  __  |/ _` | '__| '_ \ / _ \/ __/ __|
 _| |_| |_) | |____ / . \  | |  | | (_| | |  | | | |  __/\__ \__ \
|_____|____/|______/_/ \_\ |_|  |_|\__,_|_|  |_| |_|\___||___/___/
</pre>

<p align="center"><strong>Benchmark Bot</strong></p>

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
