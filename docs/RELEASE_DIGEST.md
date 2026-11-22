# Release digest

The harness pins this bot by **binary SHA-256** in
`.github/actions/setup-benchmark-bot/ibex-benchmark-bot-linux-amd64.sha256`, plus a tag whose short
SHA must match. This file records the artifact facts a consumer needs to move that pin safely.

## Artifact

| Field | Value |
| --- | --- |
| Asset | `ibex-benchmark-bot-linux-amd64` |
| Checksum asset | `ibex-benchmark-bot-linux-amd64.sha256` |
| Platform | linux-amd64, static build from `release-binary.yml` |
| Publish command | `gh release upload "$TAG" <asset> <asset>.sha256 --clobber` |

## Ranking-quality schema compatibility

| Published `schema_version` | Gold set | Required metrics | Bot support |
| --- | --- | --- | --- |
| 1 | v1 (set-overlap) | `precision_at_5`, `recall_at_10`, `mrr` | since first release |
| 2 | v2 (graded) | `ndcg_at_10`, optional `metrics.modes[]` | this release |

`schema_version` 3 and above are rejected rather than ignored: a bot that silently accepted a
newer payload would publish numbers it cannot explain.

## Verifying a pin

```bash
gh release download "$TAG" --repo Rick1330/ibexharness-benchmark-bot \
  --pattern 'ibex-benchmark-bot-linux-amd64*'
sha256sum -c ibex-benchmark-bot-linux-amd64.sha256   # against the digest pinned in setup-benchmark-bot
./ibex-benchmark-bot-linux-amd64 build-info          # build_sha must equal BENCHMARK_BOT_SHA
./ibex-benchmark-bot-linux-amd64 --help
```

The pinned digest is what establishes that the asset is the one this repository published; `build-info` then
confirms that asset was compiled from the commit named by `BENCHMARK_BOT_SHA`, so run it only after the
checksum matches. It is a consistency check between the two pins, not proof of origin — a substituted binary
can print any `build_sha`.

The release job uses it differently, on a binary it just built: it fails when `build-info` does not report
`git rev-parse HEAD`, and when the tag is not `bot-<7-char-sha>` of that commit.

The `setup-benchmark-bot` action additionally requires the binary to answer
`--require-subcommand <name>`, so a pin that predates a subcommand fails at setup rather than
mid-run.
