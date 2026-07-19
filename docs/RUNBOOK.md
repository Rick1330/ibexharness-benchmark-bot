# IBEX Harness Benchmark Bot — runbook

## Normal operation

### Two separate paths (do not confuse)

| Path | Where | Cadence | Outcome |
| --- | --- | --- | --- |
| **PR quality comment** | ibex-harness `Benchmarks` on every `pull_request` | Every PR (`fast` profile) | App posts a benchmark comment on the PR. **Never** opens a data PR. |
| **Daily data publish** | ibex-harness `notify-benchmark-bot` → this repo `Publish benchmark data` | Daily 04:00 UTC + main push collects; Sunday uses `full` profile | Bot opens a `chore(bench): …` PR with a **single Signed-off-by** commit updating `web/public/benchmarks/`. |

### Daily publish flow

1. ibex-harness **Benchmarks** completes on `main` via **schedule**, **workflow_dispatch**, or a path-triggered **push**.
2. `notify-benchmark-bot` sends `repository_dispatch` (`benchmark_main_complete`).
3. **publish-benchmark-data** checks out `vars.BOT_RELEASE_SHA`, verifies the harness run, validates the artifact, and commits JSON+badge in **one** Git Data API commit (DCO trailer included).
4. Maintainer merges the data PR after harness CI is green.

## Release pinning (`BOT_RELEASE_SHA`)

After each security-reviewed merge to `main`:

1. Note the squash merge commit SHA on `main`.
2. Set bot repo variable `BOT_RELEASE_SHA` to that SHA.
3. Set harness variable `BENCHMARK_BOT_SHA` to the same SHA (comment renderer pin).
4. Optionally tag that SHA and run **Release binary** so harness can download `ibex-benchmark-bot-linux-amd64` instead of cargo-building on PRs.
5. Run a `workflow_dispatch` dry-run publish to confirm the pinned binary works.

Never run publish workflows against a floating branch ref.

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

## Failure: verify_dispatch rejected run

**Symptoms:** Workflow fails at verify step.

**Checks:**
- Run exists and `conclusion == success`
- Run is on `main` branch
- Workflow name is exactly `Benchmarks`
- Workflow path is `.github/workflows/benchmark.yml`
- `head_sha` and `run_number` match payload

**Fix:** Re-dispatch with correct payload or use manual `workflow_dispatch`.

## Failure: artifact download

**Symptoms:** No `benchmark-data` artifact.

**Checks:**
- Harness `collect-proxy-benchmarks` completed and uploaded artifact
- App has **Actions: Read** on ibex-harness installation

## Failure: validation rejected JSON

**Symptoms:** `validate.rs` / publish step exits non-zero.

**Checks:**
- `run_number` is workflow number, not run ID
- `runs[0]` sha/run_url match verified workflow run
- k6 p99 and `error_rate` within bounds
- Schema version == 1
- `badge.svg` passes SVG safety checks

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

1. Confirm every harness PR receives a benchmark quality comment (no data PR).
2. Confirm Sunday cron (or one manual main `workflow_dispatch`) produces **one** bot data PR.
3. Confirm docs site history page shows new runs after that PR merges.
4. Confirm PR comments use the pinned Rust renderer (rich format).
