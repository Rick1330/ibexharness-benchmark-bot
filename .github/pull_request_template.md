## What and Why

## Tracking

Link the related harness PR / issue when this change exists to support it
(e.g. `Harness: Rick1330/ibex-harness#600`).

## How

## Testing

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] Manual verification (describe), if behavior is user-visible

## Security

- [ ] No secrets in code, logs, fixtures, or PR body
- [ ] Threat-model notes updated if trust boundaries / pins change (`docs/THREAT_MODEL.md`)

## Ops / release

- [ ] Docs updated (`README.md`, `docs/APP_SETUP.md`, `docs/RUNBOOK.md`) if pins, commands, or flows change
- [ ] After merge (maintainer): set `BOT_RELEASE_SHA` to the squash commit on `main`
- [ ] After merge (maintainer): set harness `BENCHMARK_BOT_SHA` to the same SHA
- [ ] After merge (maintainer): tag `bot-<shortsha>`, run **Release binary**, set harness `BENCHMARK_BOT_RELEASE_TAG`
- [ ] Do **not** merge while required CI is red; do **not** push commits directly to `main`

## Checklist

- [ ] Lint / CI passes on this PR
- [ ] Suite comments still use one sticky marker (`IBEX_BOT_COMMENT`; Proxy + Memory HNSW + Ranking + Write + Extraction Quality sections)
- [ ] Data publishes upsert one shared PR (`chore/bench-data-publish`); suite JSON files stay separate
- [ ] Proxy, HNSW, and Extraction Quality artifact verify paths remain isolated
