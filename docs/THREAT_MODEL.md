# Threat model — ibexharness-benchmark-bot

## Assets

| Asset | Location | Impact if compromised |
| --- | --- | --- |
| App private key | Bot repo secret `APP_PRIVATE_KEY` | Attacker can write branches/PRs on ibex-harness |
| Installation token | Ephemeral in workflow | Same as App key, 1-hour window |
| Dispatch PAT | Harness secret `BENCHMARK_BOT_DISPATCH_TOKEN` | Attacker can trigger bot publish workflow |
| Benchmark artifact | Harness Actions artifact | Tampered metrics in docs dashboard |

## Trust boundaries

```text
Untrusted: fork PR code, dispatch client_payload, git author metadata
Trusted:   Actions API run record, validated artifact JSON, App committer identity
```

## Threats and mitigations

### T1: Fork PR + write token

**Risk:** Malicious code runs with credentials that can write to harness.

**Mitigation:** Bot never runs on `pull_request` events. Publish workflow only on `repository_dispatch` / `workflow_dispatch`. Harness fork path remains read-only.

### T2: Spoofed repository_dispatch

**Risk:** Attacker triggers publish with fake run_id/sha.

**Mitigation:** `verify.rs` re-fetches run via App token. Requires `conclusion=success`, `head_branch=main`, matching `head_sha`, exact workflow name `Benchmarks`, and workflow path `.github/workflows/benchmark.yml`. Payload alone is never sufficient.

### T3: Artifact tampering

**Risk:** Modified benchmark JSON committed to main.

**Mitigation:** `validate.rs` schema + bounds checks. `cross_check_artifact_run` binds `runs[0]` to verified workflow metadata. `artifact.rs` allowlists zip entries and validates `badge.svg`. Manual PR review + CODEOWNERS on harness.

### T4: Stolen App private key

**Risk:** Persistent write access to harness via App.

**Mitigation:** Key only in bot repo secrets. Branch protection on bot repo for workflow/credential changes. Documented rotation in RUNBOOK. Minimal App permissions. Publish workflow checks out `vars.BOT_RELEASE_SHA` only.

### T5: Supply chain — comment renderer

**Risk:** Malicious code in renderer executed during harness CI.

**Mitigation:** Harness pins bot repo commit SHA via `BENCHMARK_BOT_SHA` (no `main` fallback). Renderer is pure JSON→Markdown; no network, no secrets. Unit tests + CODEOWNERS on renderer changes.

### T6: Markdown injection in PR comments

**Risk:** User-controlled SHA/branch/gate names break comment formatting or phishing links.

**Mitigation:** `render/sanitize.rs` strips control chars, escapes markdown-active characters, validates gate names with allowlist regex, validates SHAs as hex; URLs built from fixed templates only.

### T7: Replay dispatch

**Risk:** Duplicate or stale data PRs for same benchmark run.

**Mitigation:** Idempotency checks open PR by branch `chore/bench-data-{run_number}` and label `benchmark-data` with matching SHA in body. `ensure_not_replay` rejects `run_number` not newer than published max or duplicate `head_sha` on main. Workflow concurrency cancels in-progress duplicate jobs.

### T8: github.actor / author-email bypass

**Risk:** Skip security checks via forgeable metadata.

**Mitigation:** Never used for security gates. Removed from harness in PR #177.

### T9: Auto-merge bypass

**Risk:** Bad data merged without review.

**Mitigation:** Manual merge initially. Optional auto-merge only after 4+ weeks stable operation.

## Out of scope

- Compromise of GitHub platform itself
- Maintainer social engineering to merge malicious data PR without review

## Review checklist (before each release)

- [ ] App permissions unchanged and minimal
- [ ] No secrets in logs or workflow outputs
- [ ] `persist-credentials: false` on untrusted checkouts
- [ ] Action pins are full 40-char SHAs
- [ ] `BOT_RELEASE_SHA` and harness `BENCHMARK_BOT_SHA` updated deliberately
- [ ] `cargo audit` clean in CI
- [ ] Security unit tests pass (`verify`, `validate`, `sanitize`, `artifact`)
