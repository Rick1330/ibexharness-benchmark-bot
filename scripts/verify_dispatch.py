#!/usr/bin/env python3
"""Verify repository_dispatch payload against GitHub Actions API."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

from github_app import api_request, get_installation_token, parse_repo


EXPECTED_WORKFLOW = "Benchmarks"
EXPECTED_BRANCH = "main"
EXPECTED_CONCLUSION = "success"


def fail(message: str) -> None:
    print(f"verify_dispatch: {message}", file=sys.stderr)
    raise SystemExit(1)


def require_int(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        if isinstance(value, str) and value.isdigit():
            return int(value)
        fail(f"{label} must be an integer")
    return int(value)


def require_sha(value: Any, label: str) -> str:
    if not isinstance(value, str) or len(value) < 7:
        fail(f"{label} must be a git sha string")
    cleaned = value.strip().lower()
    if not all(ch in "0123456789abcdef" for ch in cleaned):
        fail(f"{label} must be hexadecimal")
    return cleaned


def load_payload(raw: str | None) -> dict[str, Any]:
    if raw:
        try:
            parsed = json.loads(raw)
        except json.JSONDecodeError as exc:
            fail(f"invalid JSON payload: {exc}")
        if not isinstance(parsed, dict):
            fail("payload must be a JSON object")
        return parsed
    run_id = os.environ.get("RUN_ID")
    head_sha = os.environ.get("HEAD_SHA")
    run_number = os.environ.get("RUN_NUMBER")
    if not run_id or not head_sha or not run_number:
        fail("provide --payload JSON or RUN_ID, HEAD_SHA, RUN_NUMBER env vars")
    return {"run_id": run_id, "head_sha": head_sha, "run_number": run_number}


def verify_run(
    token: str,
    repo_full: str,
    run_id: int,
    head_sha: str,
    run_number: int,
) -> dict[str, Any]:
    owner, repo = parse_repo(repo_full)
    run = api_request("GET", f"/repos/{owner}/{repo}/actions/runs/{run_id}", token)
    if not isinstance(run, dict):
        fail("unexpected run response")

    conclusion = str(run.get("conclusion", ""))
    if conclusion != EXPECTED_CONCLUSION:
        fail(f"run conclusion is {conclusion!r}, expected {EXPECTED_CONCLUSION!r}")

    head_branch = str(run.get("head_branch", ""))
    if head_branch != EXPECTED_BRANCH:
        fail(f"run head_branch is {head_branch!r}, expected {EXPECTED_BRANCH!r}")

    api_sha = str(run.get("head_sha", "")).lower()
    if api_sha != head_sha:
        fail(f"head_sha mismatch: payload={head_sha} api={api_sha}")

    api_run_number = run.get("run_number")
    if require_int(api_run_number, "run.run_number") != run_number:
        fail(f"run_number mismatch: payload={run_number} api={api_run_number}")

    workflow_name = ""
    path = run.get("path")
    if isinstance(path, str) and path:
        workflow_name = Path(path).stem.replace("_", " ").replace("-", " ")
    name = str(run.get("name", ""))
    if name != EXPECTED_WORKFLOW and workflow_name != EXPECTED_WORKFLOW:
        # GitHub sets run.name from workflow `name:` field
        if EXPECTED_WORKFLOW.lower() not in name.lower():
            fail(f"workflow name is {name!r}, expected {EXPECTED_WORKFLOW!r}")

    return run


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify benchmark dispatch payload")
    parser.add_argument("--payload", help="JSON client_payload from repository_dispatch")
    parser.add_argument("--repo", default=os.environ.get("HARNESS_REPO", "Rick1330/ibex-harness"))
    args = parser.parse_args()

    payload = load_payload(args.payload)
    run_id = require_int(payload.get("run_id"), "run_id")
    head_sha = require_sha(payload.get("head_sha"), "head_sha")
    run_number = require_int(payload.get("run_number"), "run_number")

    token = get_installation_token()
    run = verify_run(token, args.repo, run_id, head_sha, run_number)

    print(
        json.dumps(
            {
                "ok": True,
                "run_id": run_id,
                "head_sha": head_sha,
                "run_number": run_number,
                "run_url": run.get("html_url"),
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
