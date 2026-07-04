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

**Mitigation:** `verify_dispatch.py` re-fetches run via App token. Requires `conclusion=success`, `head_branch=main`, matching `head_sha`, workflow name `Benchmarks`. Payload alone is never sufficient.

### T3: Artifact tampering

**Risk:** Modified benchmark JSON committed to main.

**Mitigation:** `validate_published_data.py` schema + bounds checks. Run metadata cross-check (`run_url`, `sha`, `run_number`). Manual PR review + CODEOWNERS on harness.

### T4: Stolen App private key

**Risk:** Persistent write access to harness via App.

**Mitigation:** Key only in bot repo secrets. Branch protection on bot repo for workflow/credential changes. Documented rotation in RUNBOOK. Minimal App permissions.

### T5: Supply chain — comment renderer

**Risk:** Malicious code in renderer executed during harness CI.

**Mitigation:** Harness pins bot repo commit SHA (not branch/tag). Renderer is pure JSON→Markdown; no network, no secrets, no `eval`. Unit tests + CODEOWNERS on renderer changes.

### T6: Markdown injection in PR comments

**Risk:** User-controlled SHA/branch breaks comment formatting or phishing links.

**Mitigation:** `sanitize.mjs` strips control chars; SHAs validated as hex; URLs built from fixed templates only.

### T7: Replay dispatch

**Risk:** Duplicate data PRs for same benchmark run.

**Mitigation:** Idempotency: skip if open PR exists for branch `chore/bench-data-{run_number}` or label `benchmark-data` + matching SHA in body.

### T8: github.actor / author-email bypass

**Risk:** Skip security checks via forgeable metadata.

**Mitigation:** Never used for security gates (Sonar S8232). Removed from harness in PR #177.

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
- [ ] Renderer pin updated deliberately in harness workflow
