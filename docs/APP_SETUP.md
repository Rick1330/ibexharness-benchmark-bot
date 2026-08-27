# GitHub App setup

## 1. Create the App

1. **GitHub → Settings → Developer settings → GitHub Apps → New GitHub App**
2. **Name:** `IBEX Harness Benchmark Bot` (slug: `ibexharness-benchmark-bot`)
3. **Homepage URL:** `https://github.com/Rick1330/ibexharness-benchmark-bot`
4. **Webhook:** inactive (not used)
5. **Logo:** upload `docs/brand/android-chrome-512x512.png` (IBEX mark shown beside bot comments)
6. **Permissions:** Contents R/W, Pull requests R/W, Commit statuses R/W, Actions read, Metadata read
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

Set repo variable `BOT_RELEASE_SHA` to a reviewed squash commit on `main` after each
**green** merge (never pin a commit whose CI failed).

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
| `BENCHMARK_BOT_RELEASE_TAG` | `bot-<7-char-sha>` matching that pin (after **Release binary** uploads the asset) |

Harness `.github/actions/setup-benchmark-bot` ignores a release tag that does not match
the pin short SHA, and can require a subcommand (e.g. `post-hnsw-pr-comment`) so a
stale binary cannot silently break Memory Benchmarks collect.

## 5. Verify

1. Open any harness PR → **Benchmarks** posts/updates the shared sticky comment (`IBEX_BOT_COMMENT`) with the suite matrix + **Proxy** deep dive. **No** data PR.
2. When **Memory Benchmarks** runs on the PR → App upserts the **Memory HNSW** matrix row + deep dive on the same comment via `post-hnsw-pr-comment`. **No** HNSW data PR from PR runs.
3. On schedule / main collect → bot publish workflows open one data PR per suite.
4. Confirm `BENCHMARK_BOT_ENABLED=true`, `BENCHMARK_BOT_SHA` == `BOT_RELEASE_SHA`, and `BENCHMARK_BOT_RELEASE_TAG` matches `bot-${SHA:0:7}` with a downloadable linux-amd64 asset.

Harness `notify-benchmark-bot` / `notify-hnsw-benchmark-bot` fire on successful main-branch collects (plus schedule / dispatch per workflow).

Key rotation: [RUNBOOK.md](RUNBOOK.md).
