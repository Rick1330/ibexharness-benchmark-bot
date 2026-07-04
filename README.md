# ibexharness-benchmark-bot

External GitHub App and automation for publishing IBEX Harness benchmark data to `main` after merge, plus a shared PR comment renderer consumed by [ibex-harness](https://github.com/Rick1330/ibex-harness).

**Harness repo:** [Rick1330/ibex-harness](https://github.com/Rick1330/ibex-harness)

**Related ADR:** [ADR-0024 — Benchmark data publishing model](https://github.com/Rick1330/ibex-harness/blob/main/docs/app/content/docs/adr/0024-benchmark-data-publishing-model.mdx)

## Status

Implementation complete. Configure the GitHub App per [`docs/APP_SETUP.md`](docs/APP_SETUP.md) before the publish workflow can run.

## Repository layout

```text
ibexharness-benchmark-bot/
  README.md
  docs/
    APP_SETUP.md
    BENCHMARK_BOT.md
    RUNBOOK.md
    THREAT_MODEL.md
    superpowers/specs/
  packages/
    comment-renderer/     # shared JSON → Markdown (PR + data PR)
  scripts/
    github_app.py
    verify_dispatch.py
    publish_benchmark_data.py
    vendor/               # pinned validate_published_data.py
  .github/workflows/
    ci.yml
    publish-benchmark-data.yml
```

## Quick start

1. Follow [`docs/APP_SETUP.md`](docs/APP_SETUP.md) to create the GitHub App and store secrets.
2. Add `BENCHMARK_BOT_DISPATCH_TOKEN` to ibex-harness repo secrets.
3. Set harness variable `BENCHMARK_BOT_ENABLED=true`.
4. Main benchmark run → `repository_dispatch` → bot opens data PR on ibex-harness.

## Development

```bash
# Comment renderer tests
cd packages/comment-renderer && npm test

# Python unit tests
python -m unittest discover -s scripts/tests -p 'test_*.py'

# Action pin validation
bash .github/scripts/validate-action-pins.sh
```
