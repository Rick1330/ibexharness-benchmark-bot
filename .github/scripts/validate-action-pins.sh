#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

fail=0

while IFS= read -r ref; do
  [[ -z "$ref" ]] && continue
  if [[ ! "$ref" =~ ^[a-f0-9]{40}$ ]]; then
    echo "Unpinned or invalid action ref (expected 40-char SHA): ${ref}"
    fail=1
  fi
done < <(
  grep -rhoE 'uses:[[:space:]]*[^#[:space:]]+' .github/workflows/ \
    | sed -E 's#^uses:[[:space:]]*##' \
    | grep -v '^\./' \
    | sed -E 's#^[^@]+@##' \
    | sed 's/[[:space:]]*$//' \
    | sort -u
)

exit "$fail"
