# Design spec: ibexharness-benchmark-bot

**Date:** 2026-07-04  
**Status:** Approved for implementation  
**Authors:** IBEX Harness team

## Summary

External GitHub App repository that publishes validated benchmark data to ibex-harness `main` via pull request, triggered by `repository_dispatch` from harness with independent Actions API verification. Includes a shared comment-renderer package for rich PR benchmark comments.

## Problem

PR #177 removed in-PR branch writes. Post-merge publishing still used interim `open-benchmark-data-pr` with harness `GITHUB_TOKEN`. PR comments were a minimal 3-row markdown table.

## Solution

| Component | Responsibility |
| --- | --- |
| `repository_dispatch` trigger | Harness notifies bot after main benchmark success |
| `verify_dispatch.py` | Re-verify run via API; never trust payload alone |
| `publish_benchmark_data.py` | App token → branch + commit + PR on harness |
| `packages/comment-renderer` | Rich markdown for PR comments and data PR bodies |
| GitHub App | Attributable bot identity, least-privilege write |

## Deployment

**Chosen:** GitHub App + GitHub Actions only ($0). No Cloudflare Workers, no self-hosted runners, no paid SaaS.

**Rejected:** Cross-repo `workflow_run` (impossible on GitHub).

## Security

See [`THREAT_MODEL.md`](../THREAT_MODEL.md). Key rules:

- App private key only in bot repo
- Dispatch PAT only in harness repo
- No `github.actor` or author-email bypass
- Pin renderer SHA in harness workflow

## Success criteria

- Main benchmark → bot data PR within ~5 minutes
- Rich PR comments (gate checks, stages, go bench)
- Zero harness branch writes from bot
- `$0` monthly cost

## References

- [ADR-0024](https://github.com/Rick1330/ibex-harness/blob/main/docs/app/content/docs/adr/0024-benchmark-data-publishing-model.mdx)
- [BENCHMARK_BOT.md](../BENCHMARK_BOT.md)
- [APP_SETUP.md](../APP_SETUP.md)
