# GitHub App setup — ibexharness-benchmark-bot

Follow these steps once to enable the publish workflow. All steps are **$0** on public repositories.

## 1. Create the GitHub App

1. Go to **GitHub → Settings → Developer settings → GitHub Apps → New GitHub App**.
2. **Name:** `ibexharness-benchmark-bot`
3. **Homepage URL:** `https://github.com/Rick1330/ibexharness-benchmark-bot`
4. **Webhook:** Uncheck **Active** (not needed — we use `repository_dispatch`, not webhooks).
5. **Repository permissions:**
   - Contents: **Read and write**
   - Pull requests: **Read and write**
   - Actions: **Read-only**
   - Metadata: **Read-only** (automatic)
6. **Where can this GitHub App be installed?** Only on this account.
7. Create the app. Note the **App ID**.
8. Generate a **private key** (.pem). Download and store securely.

## 2. Install on ibex-harness only

1. App settings → **Install App** → select **Rick1330/ibex-harness** only.
2. Note the **Installation ID** from:
   ```bash
   gh api /users/Rick1330/installations --jq '.installations[] | select(.app_slug=="ibexharness-benchmark-bot") | .id'
   ```
   Or from the installation URL: `.../installations/{INSTALLATION_ID}`.

## 3. Bot repo secrets

In `Rick1330/ibexharness-benchmark-bot` → **Settings → Secrets → Actions**:

| Secret | Value |
| --- | --- |
| `APP_ID` | GitHub App ID |
| `APP_PRIVATE_KEY` | Full PEM contents (including `BEGIN`/`END` lines) |
| `INSTALLATION_ID` | Installation ID for ibex-harness |
| `HARNESS_REPO` | `Rick1330/ibex-harness` (optional override; default in workflow) |

## 4. Dispatch token (harness repo)

Create a **fine-grained PAT** (or classic PAT with minimal scope):

- Resource owner: Rick1330
- Repository access: `ibexharness-benchmark-bot` only
- Permissions: **Contents: Read**, **Metadata: Read** (needed for `repository_dispatch`)

Store in **ibex-harness** repo secret:

| Secret | Value |
| --- | --- |
| `BENCHMARK_BOT_DISPATCH_TOKEN` | The PAT |

## 5. Enable harness integration

In **ibex-harness** → **Settings → Secrets and variables → Actions → Variables**:

| Variable | Value |
| --- | --- |
| `BENCHMARK_BOT_ENABLED` | `true` |
| `BENCHMARK_BOT_SHA` | Pinned commit SHA of this repo (supply-chain pin for PR comments) |

## 6. Verify

1. Run ibex-harness **Benchmarks** workflow on `main` (workflow_dispatch).
2. Confirm `notify-benchmark-bot` job succeeds.
3. Confirm **publish-benchmark-data** workflow runs in this repo.
4. Merge the opened data PR on ibex-harness after CI passes.

## Key rotation

See [`RUNBOOK.md`](RUNBOOK.md#private-key-rotation).
