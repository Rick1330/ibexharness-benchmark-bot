# Runbook — ibexharness-benchmark-bot

## Normal operation

1. ibex-harness **Benchmarks** completes on `main` (schedule, push, or manual).
2. `notify-benchmark-bot` sends `repository_dispatch` to this repo.
3. **publish-benchmark-data** workflow verifies run, validates artifact, opens PR on ibex-harness.
4. Maintainer merges data PR after harness CI is green.

## Manual re-publish

When a publish failed but the harness benchmark run succeeded:

1. Open **ibexharness-benchmark-bot** → Actions → **Publish benchmark data** → **Run workflow**.
2. Inputs:
   - `run_id`: harness Actions run ID
   - `head_sha`: commit SHA from that run
   - `run_number`: workflow run number (not run ID)
3. Workflow verifies and opens PR (or skips if idempotent duplicate).

## Failure: verify_dispatch rejected run

**Symptoms:** Workflow fails at "Verify dispatch payload".

**Checks:**
- Run exists and `conclusion == success`
- Run is on `main` branch
- Workflow file name is `Benchmarks`
- `head_sha` matches payload

**Fix:** Re-dispatch with correct payload or use manual `workflow_dispatch`.

## Failure: artifact download

**Symptoms:** No `benchmark-data` artifact.

**Checks:**
- Harness `collect-proxy-benchmarks` completed and uploaded artifact
- App has **Actions: Read** on ibex-harness installation

## Failure: validation rejected JSON

**Symptoms:** `validate_published_data.py` exit non-zero.

**Checks:**
- `run_number` is workflow number, not run ID
- k6 p99 within bounds
- Schema version == 1

**Fix:** Fix harness benchmark pipeline; do not bypass validation.

## Private key rotation

1. GitHub App settings → **Generate a new private key**.
2. Update bot repo secret `APP_PRIVATE_KEY` with new PEM.
3. Run a test `workflow_dispatch` publish.
4. Revoke old private key in App settings.

## Dispatch token rotation

1. Create new fine-grained PAT with same minimal scopes.
2. Update harness secret `BENCHMARK_BOT_DISPATCH_TOKEN`.
3. Revoke old PAT.

## Disable bot temporarily

Set ibex-harness variable `BENCHMARK_BOT_ENABLED` to `false`. Notify job skips; no dispatches sent.

## Alerts

Monitor:
- Failed **publish-benchmark-data** workflow runs
- Open `benchmark-data` PRs older than 7 days unmerged

No external paid alerting service required — use GitHub email notifications for workflow failures.

## Cutover verification (post-deploy)

After enabling the bot:

1. Confirm two weekly benchmark cycles produce bot PRs.
2. Confirm docs site history page shows new runs after merge.
3. Confirm PR benchmark comments use shared renderer (rich format).
