# IBEX Benchmark Bot — specification

This document defines the **external GitHub App** that publishes benchmark data to [ibex-harness](https://github.com/Rick1330/ibex-harness) after merge. Implementation lives in **this repository**.

Related: [ADR-0024 — Benchmark data publishing model](https://github.com/Rick1330/ibex-harness/blob/main/docs/app/content/docs/adr/0024-benchmark-data-publishing-model.mdx).

## Goals

- Publish validated `docs/app/public/benchmarks/benchmark-data.json` and `badge.svg` to `main` via **pull request**, never by pushing to contributor PR branches.
- Provide **cryptographically attributable** bot identity (GitHub App), not spoofable git author strings.
- Use **least-privilege** tokens with short lifetimes and documented rotation.
- Provide a **shared comment renderer** consumed by ibex-harness for rich PR benchmark comments.

## Non-goals

- Writing benchmark JSON onto feature PR branches.
- Running benchmark collection inside the bot repo (collection stays in ibex-harness `Benchmarks` workflow).

## Architecture

```text
ibex-harness                          ibexharness-benchmark-bot (this repo)
────────────────                      ────────────────────────────────────
Benchmarks workflow ──artifact──►     repository_dispatch (benchmark_main_complete)
  (main / schedule)                         │
                                            ▼
                                      Verify run via Actions API (never trust dispatch alone)
                                            │
                                            ▼
                                      Validate artifact (vendor validate_published_data.py)
                                            │
                                            ▼
                                      GitHub App installation token
                                            │
                                            ▼
                                      Branch chore/bench-data-{run_number}
                                      PR: chore(bench): weekly benchmark data update
```

**Note:** `workflow_run` cannot watch another repository's workflows. Cross-repo coordination uses `repository_dispatch` from harness plus independent API verification in this repo.

## GitHub App permissions (minimal)

| Permission | Access | Reason |
| --- | --- | --- |
| Contents | Read & write | Create branch, commit JSON + badge on bot branch |
| Pull requests | Read & write | Open/update data PR |
| Actions | Read | Locate successful benchmark workflow run + download artifact |
| Metadata | Read | Required by GitHub Apps |

Do **not** grant administration, workflows write, or org-level scopes beyond the single target repository.

## Token lifecycle

1. Generate a GitHub App; store **App ID** and **private key** in this repo's secrets only.
2. Install the app on **`Rick1330/ibex-harness`** only.
3. Bot workflow exchanges JWT → **installation access token** (1-hour TTL) per job; never log token values.
4. Harness stores a separate **`BENCHMARK_BOT_DISPATCH_TOKEN`** (fine-grained PAT) to call `repository_dispatch` on this repo — not the App private key.

## Identity verification

- Commits created via the App show `committer.type == "Bot"` and login `ibexharness-benchmark-bot[bot]`.
- Harness workflows must not treat arbitrary author emails as proof of automation.
- Do **not** use `github.actor` or git author email in workflow `if` conditions for security decisions.

## Trigger

`repository_dispatch` event type `benchmark_main_complete` with payload:

```json
{
  "run_id": "28697279483",
  "head_sha": "b953161761abfca666fadbfc36153bb89a7aac1a",
  "run_number": "16"
}
```

Bot workflow re-verifies via Actions API before publishing.

## Publish flow

1. Download `benchmark-data` artifact from the verified harness run.
2. Run schema validation (`scripts/vendor/validate_published_data.py`).
3. Create branch `chore/bench-data-{run_number}` from latest `main`.
4. Commit files under `docs/app/public/benchmarks/`.
5. Open PR with rich body from `packages/comment-renderer` (`mode=data-pr`).
6. Apply labels: `automated`, `benchmark-data`.
7. Manual merge after harness CI passes (auto-merge optional after stability period).

## Shared comment renderer

Harness checks out this repo at a **pinned commit SHA** and runs:

```bash
node packages/comment-renderer/cli.mjs pr-comment
```

See `packages/comment-renderer/README.md`.

## Migration from interim workflow

The harness `open-benchmark-data-pr` job is removed. Publishing is exclusively via this bot once `BENCHMARK_BOT_ENABLED=true` and secrets are configured.

## References

- [`docs/THREAT_MODEL.md`](THREAT_MODEL.md)
- [`docs/RUNBOOK.md`](RUNBOOK.md)
- [`docs/APP_SETUP.md`](APP_SETUP.md)
