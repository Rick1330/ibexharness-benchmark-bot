# GitHub App setup

## 1. Create the App

1. **GitHub → Settings → Developer settings → GitHub Apps → New GitHub App**
2. **Name:** `IBEX Harness Benchmark Bot` (slug: `ibexharness-benchmark-bot`)
3. **Homepage URL:** `https://github.com/Rick1330/ibexharness-benchmark-bot`
4. **Webhook:** inactive (not used)
5. **Logo:** upload `docs/brand/android-chrome-512x512.png` (IBEX mark shown beside bot comments)
6. **Permissions:** Contents R/W, Pull requests R/W, Actions read, Metadata read
7. **Install on:** this account only
8. Note **App ID**; generate and save **private key** (.pem)

## 2. Install on ibex-harness

Install the App on **Rick1330/ibex-harness** only. Note the **Installation ID** from the installation URL or:

```bash
gh api /users/Rick1330/installations --jq '.installations[] | select(.app_slug=="ibexharness-benchmark-bot") | .id'
```

## 3. Bot repo secrets

`Rick1330/ibexharness-benchmark-bot` → Settings → Secrets → Actions:

| Secret | Value |
| --- | --- |
| `APP_ID` | App ID |
| `APP_PRIVATE_KEY` | PEM private key |
| `INSTALLATION_ID` | Installation ID |

Set repo variable `BOT_RELEASE_SHA` to a reviewed commit on `main` after each release.

## 4. Harness repo secrets and variables

**Secrets** (`ibex-harness`):

| Secret | Value |
| --- | --- |
| `BENCHMARK_BOT_DISPATCH_TOKEN` | Fine-grained PAT: read on `ibexharness-benchmark-bot` (for `repository_dispatch`) |
| `BENCHMARK_BOT_APP_ID` | Same App ID |
| `BENCHMARK_BOT_APP_PRIVATE_KEY` | Same PEM (posts PR comments as the App, not `github-actions[bot]`) |
| `BENCHMARK_BOT_INSTALLATION_ID` | Same installation ID |

**Variables:**

| Variable | Value |
| --- | --- |
| `BENCHMARK_BOT_ENABLED` | `true` |
| `BENCHMARK_BOT_SHA` | Same pinned commit as `BOT_RELEASE_SHA` |

## 5. Verify

1. Run harness **Benchmarks** on `main`.
2. Confirm bot **publish-benchmark-data** workflow opens a data PR.
3. Open a harness PR benchmark run — comment should show the IBEX App avatar and mark in the body.

Key rotation: [RUNBOOK.md](RUNBOOK.md).
