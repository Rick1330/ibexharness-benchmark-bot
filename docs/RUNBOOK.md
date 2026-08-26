# IBEX Harness Benchmark Bot — runbook

## Normal operation

### Two separate paths (do not confuse)

| Path | Where | Cadence | Outcome |
| --- | --- | --- | --- |
| **Proxy PR quality comment** | ibex-harness `Benchmarks` on `pull_request` | Every matching PR (**smoke** profile) | App posts sticky **Proxy suite** comment (`IBEX_BOT_COMMENT`): Suite/Status/Regressions/P99 badges, performance summary, promoted **Auth & proxy stages** table, k6/regression details. **Never** opens a data PR. |
| **Memory HNSW PR comment** | ibex-harness `Memory Benchmarks` on `pull_request` | Every matching PR (**smoke** = 10K) | App posts sticky **Memory HNSW suite** comment (`IBEX_BOT_COMMENT_HNSW`): aligned badge row, corpus table, Coverage section. Missing 1M on smoke/fast shows informational **1M deferred**, not WARN. **Never** opens a data PR. |
| **Daily proxy data publish** | ibex-harness `notify-benchmark-bot` → this repo `Publish benchmark data` | Daily 04:00 UTC + main push collects; Sunday uses `full` profile | Bot opens a `chore(bench): …` PR updating `benchmark-data.json` + `badge.svg`. |
| **HNSW data publish** | ibex-harness `Memory Benchmarks` → this repo `Publish HNSW benchmark data` | Sunday 05:00 UTC + main push / dispatch | Bot opens a data PR updating **only** `hnsw-benchmark-data.json` (never proxy files). |

Suite comments are independent sticky threads. Do not reuse markers across suites.

### Daily publish flow (proxy)

1. ibex-harness **Benchmarks** completes on `main` via **schedule**, **workflow_dispatch**, or a path-triggered **push**.
2. `notify-benchmark-bot` sends `repository_dispatch` (`benchmark_main_complete`).
3. **publish-benchmark-data** checks out `vars.BOT_RELEASE_SHA`, verifies the harness run, validates the artifact, and commits JSON+badge in **one** Git Data API commit (DCO trailer included).
4. Maintainer merges the data PR after harness CI is green.

### HNSW publish flow

1. ibex-harness **Memory Benchmarks** (`.github/workflows/memory-benchmark.yml`) completes on `main`.
2. `notify-hnsw-benchmark-bot` sends `repository_dispatch` (`memory_benchmark_main_complete`).
3. **publish-hnsw-benchmark-data** verifies workflow name/path (`Memory Benchmarks` / `.github/workflows/memory-benchmark.yml`), downloads artifact `hnsw-benchmark-data`, validates, and commits `web/public/benchmarks/hnsw-benchmark-data.json` only.
4. Result cells may include methodology knobs (`ef_search`, `min_similarity`, `iterative_scan`, `index_build_mode`, plan/buffer stats). Validation requires core latency/recall fields; optional knobs are range-checked when present.
5. Maintainer merges; pin `BOT_RELEASE_SHA` / harness `BENCHMARK_BOT_SHA` together after bot code merges.

## Contribution / merge policy

- Every change lands via PR using `.github/pull_request_template.md`.
- Do **not** push commits directly to `main`.
- Do **not** merge while `validate` (fmt / clippy / audit / test) is red.
- Prefer squash-merge; pin the resulting `main` commit SHA (not the PR head).

## Release pinning (`BOT_RELEASE_SHA` + harness tag)

After each security-reviewed, **green** merge to `main`:

1. Note the squash merge commit SHA on `main`.
2. Set bot repo variable `BOT_RELEASE_SHA` to that SHA.
3. Set harness variable `BENCHMARK_BOT_SHA` to the same SHA (comment / publish pin).
4. Tag that SHA as `bot-<7-char-sha>` and run **Release binary** (`workflow_dispatch` with that tag) so `ibex-benchmark-bot-linux-amd64` is attached.
5. Set harness variable `BENCHMARK_BOT_RELEASE_TAG` to the same `bot-<7-char-sha>` (setup action rejects tags that do not match the pin short SHA).
6. Confirm Memory collect can resolve `post-hnsw-pr-comment` from the release (or cargo-build fallback).
7. Optional: `workflow_dispatch` dry-run publish to confirm the pinned binary works.

Never run publish workflows against a floating branch ref. Never leave `BENCHMARK_BOT_RELEASE_TAG` pointing at an older bot that lacks required subcommands.
## Manual re-publish

When a publish failed but the harness benchmark run succeeded:

1. Open **ibexharness-benchmark-bot** → Actions → **Publish benchmark data** → **Run workflow** (requires `publish` environment approval if configured).
2. Inputs:
   - `run_id`: harness Actions run ID
   - `head_sha`: commit SHA from that run
   - `run_number`: workflow run number (not run ID)
   - `dry_run`: `true` first to validate only
3. Workflow verifies and opens PR (or skips if idempotent duplicate).

To force a weekly-style publish mid-week: run harness **Benchmarks** with `workflow_dispatch` on `main` (that is the only non-Sunday path that notifies this bot).

For HNSW: use **Publish HNSW benchmark data** with the same input shape against a successful **Memory Benchmarks** run (`memory_benchmark_main_complete`).

## Failure: verify_dispatch rejected run

**Symptoms:** Workflow fails at verify step.

**Checks (proxy):**
- Run exists and `conclusion == success`
- Run is on `main` branch
- Workflow name is exactly `Benchmarks`
- Workflow path is `.github/workflows/benchmark.yml`
- `head_sha` and `run_number` match payload

**Checks (HNSW):** same, but workflow name `Memory Benchmarks` and path `.github/workflows/memory-benchmark.yml`.

**Fix:** Re-dispatch with correct payload or use manual `workflow_dispatch`.

## Failure: artifact download

**Symptoms:** No `benchmark-data` / `hnsw-benchmark-data` artifact.

**Checks:**
- Harness collect job completed and uploaded the matching artifact label
- App has **Actions: Read** on ibex-harness installation

## Failure: validation rejected JSON

**Symptoms:** `validate.rs` / `hnsw_validate.rs` / publish step exits non-zero.

**Checks (proxy):**
- `run_number` is workflow number, not run ID
- `runs[0]` sha/run_url match verified workflow run
- k6 p99 and `error_rate` within bounds
- Schema version == 1
- `badge.svg` passes SVG safety checks

**Checks (HNSW):**
- Artifact is only `hnsw-benchmark-data.json` (proxy schema must not be applied)
- `runs[0]` sha/run_url/run_number match the Memory Benchmarks run
- Corpus sizes and recall/latency fields parse cleanly

**Fix:** Fix harness benchmark pipeline; do not bypass validation.

## Private key rotation

1. GitHub App settings → **Generate a new private key**.
2. Update bot repo secret `APP_PRIVATE_KEY` with new PEM.
3. Run a test `workflow_dispatch` publish with `dry_run=true`.
4. Revoke old private key in App settings.

## Dispatch token rotation

1. Create new fine-grained PAT with same minimal scopes.
2. Update harness secret `BENCHMARK_BOT_DISPATCH_TOKEN`.
3. Revoke old PAT.

## Disable bot temporarily

Set ibex-harness variable `BENCHMARK_BOT_ENABLED` to `false`. Notify job skips; no dispatches sent. PR comments still require App secrets when benchmarks run on PRs.

## Incident response

If `APP_PRIVATE_KEY` or dispatch PAT may be compromised:

1. Set `BENCHMARK_BOT_ENABLED=false` immediately.
2. Revoke compromised credential.
3. Review open `benchmark-data` PRs and recent App audit log entries.
4. Rotate credentials per sections above before re-enabling.

## Alerts

Monitor:
- Failed **publish-benchmark-data** workflow runs
- Open `benchmark-data` PRs older than 7 days unmerged

Use GitHub email notifications for workflow failures.

## Cutover verification (post-deploy)

After enabling the bot:

1. Confirm every matching harness PR receives a **proxy** quality comment (no data PR).
2. Confirm Memory Benchmarks PRs receive a separate **HNSW** sticky comment (`IBEX_BOT_COMMENT_HNSW`).
3. Confirm Sunday cron (or one manual main `workflow_dispatch`) produces **one** bot data PR per suite that ran.
4. Confirm `/benchmarks` and `/benchmarks/memory` show new runs after those PRs merge.
5. Confirm PR comments use the pinned Rust renderer (rich format).
