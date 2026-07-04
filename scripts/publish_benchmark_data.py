#!/usr/bin/env python3
"""Download artifact, validate, and open benchmark data PR on ibex-harness."""

from __future__ import annotations

import argparse
import base64
import io
import json
import os
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

from github_app import api_request, get_installation_token, parse_repo
from verify_dispatch import verify_run, require_int, require_sha, load_payload


ARTIFACT_NAME = "benchmark-data"
BRANCH_PREFIX = "chore/bench-data-"
JSON_PATH = "docs/app/public/benchmarks/benchmark-data.json"
BADGE_PATH = "docs/app/public/benchmarks/badge.svg"
VALIDATE_SCRIPT = Path(__file__).resolve().parent / "vendor" / "validate_published_data.py"


def fail(message: str) -> None:
    print(f"publish_benchmark_data: {message}", file=sys.stderr)
    raise SystemExit(1)


def download_artifact_zip(token: str, repo_full: str, run_id: int) -> bytes:
    owner, repo = parse_repo(repo_full)
    artifacts = api_request("GET", f"/repos/{owner}/{repo}/actions/runs/{run_id}/artifacts", token)
    if not isinstance(artifacts, dict):
        fail("invalid artifacts response")
    items = artifacts.get("artifacts", [])
    if not isinstance(items, list):
        fail("artifacts list missing")
    match = next((a for a in items if isinstance(a, dict) and a.get("name") == ARTIFACT_NAME), None)
    if not match:
        fail(f"artifact {ARTIFACT_NAME!r} not found for run {run_id}")
    artifact_id = match.get("id")
    if not isinstance(artifact_id, int):
        fail("artifact id missing")
    url = f"https://api.github.com/repos/{owner}/{repo}/actions/artifacts/{artifact_id}/zip"
    import urllib.request

    request = urllib.request.Request(
        url,
        headers={
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github+json",
            "User-Agent": "ibexharness-benchmark-bot",
        },
        method="GET",
    )
    with urllib.request.urlopen(request, timeout=120) as response:
        return response.read()


def extract_artifact_files(zip_bytes: bytes, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(io.BytesIO(zip_bytes)) as archive:
        archive.extractall(dest)
    json_candidates = list(dest.rglob("benchmark-data.json"))
    badge_candidates = list(dest.rglob("badge.svg"))
    if not json_candidates:
        fail("benchmark-data.json not in artifact")
    if not badge_candidates:
        fail("badge.svg not in artifact")


def validate_benchmark_json(work_dir: Path) -> Path:
    json_path = next(work_dir.rglob("benchmark-data.json"))
    rel = json_path.relative_to(work_dir)
    result = subprocess.run(
        [sys.executable, str(VALIDATE_SCRIPT), str(rel)],
        cwd=work_dir,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        fail(f"validation failed: {result.stderr.strip()}")
    return json_path


def find_badge(work_dir: Path) -> Path:
    return next(work_dir.rglob("badge.svg"))


def branch_name(run_number: int) -> str:
    return f"{BRANCH_PREFIX}{run_number}"


def ref_exists(token: str, owner: str, repo: str, ref: str) -> bool:
    try:
        api_request("GET", f"/repos/{owner}/{repo}/git/ref/{ref}", token)
        return True
    except RuntimeError as exc:
        if "404" in str(exc):
            return False
        raise


def find_open_pr_for_branch(token: str, owner: str, repo: str, branch: str) -> dict[str, Any] | None:
    pulls = api_request("GET", f"/repos/{owner}/{repo}/pulls?state=open&head={owner}:{branch}", token)
    if isinstance(pulls, list) and pulls:
        first = pulls[0]
        if isinstance(first, dict):
            return first
    return None


def get_main_sha(token: str, owner: str, repo: str) -> str:
    ref = api_request("GET", f"/repos/{owner}/{repo}/git/ref/heads/main", token)
    if not isinstance(ref, dict):
        fail("main ref response invalid")
    obj = ref.get("object", {})
    if not isinstance(obj, dict):
        fail("main ref object invalid")
    sha = obj.get("sha")
    if not isinstance(sha, str):
        fail("main sha missing")
    return sha


def create_branch(token: str, owner: str, repo: str, branch: str, from_sha: str) -> None:
    api_request(
        "POST",
        f"/repos/{owner}/{repo}/git/refs",
        token,
        body={"ref": f"refs/heads/{branch}", "sha": from_sha},
    )


def file_sha_on_branch(token: str, owner: str, repo: str, path: str, branch: str) -> str | None:
    try:
        meta = api_request("GET", f"/repos/{owner}/{repo}/contents/{path}?ref={branch}", token)
    except RuntimeError as exc:
        if "404" in str(exc):
            return None
        raise
    if isinstance(meta, dict) and isinstance(meta.get("sha"), str):
        return meta["sha"]
    return None


def put_file(
    token: str,
    owner: str,
    repo: str,
    path: str,
    branch: str,
    content_bytes: bytes,
    message: str,
    file_sha: str | None,
) -> None:
    body: dict[str, Any] = {
        "message": message,
        "content": base64.b64encode(content_bytes).decode("ascii"),
        "branch": branch,
    }
    if file_sha:
        body["sha"] = file_sha
    api_request("PUT", f"/repos/{owner}/{repo}/contents/{path}", token, body=body)


def render_data_pr_body(
    run_number: int,
    head_sha: str,
    run_url: str | None,
    benchmark_json: dict[str, Any],
) -> str:
    runs = benchmark_json.get("runs", [])
    latest = runs[0] if isinstance(runs, list) and runs else {}
    status = latest.get("status", "unknown")
    short_sha = latest.get("short_sha", head_sha[:7])
    p99 = latest.get("k6", {}).get("p99_ms", "—")
    lines = [
        "## Automated benchmark data update",
        "",
        f"| Field | Value |",
        f"| --- | --- |",
        f"| Status | **{status}** |",
        f"| Run number | {run_number} |",
        f"| Head SHA | `{short_sha}` |",
        f"| Proxy p99 | {p99} ms |",
        "",
    ]
    if run_url:
        lines.append(f"- [Harness benchmark workflow run]({run_url})")
    lines.extend(
        [
            "",
            "### Reviewer checklist",
            "",
            "- [ ] `validate_published_data.py` passed in bot workflow",
            "- [ ] Harness CI green on this PR",
            "- [ ] `run_number` is workflow number, not run ID",
            "- [ ] Docs preview shows updated benchmark history",
            "",
            "Labels: `automated`, `benchmark-data`",
        ]
    )
    return "\n".join(lines)


def open_pull_request(
    token: str,
    owner: str,
    repo: str,
    branch: str,
    run_number: int,
    body: str,
) -> dict[str, Any]:
    title = f"chore(bench): benchmark data update (run #{run_number})"
    result = api_request(
        "POST",
        f"/repos/{owner}/{repo}/pulls",
        token,
        body={
            "title": title,
            "head": branch,
            "base": "main",
            "body": body,
            "maintainer_can_modify": False,
        },
    )
    if not isinstance(result, dict):
        fail("pull request response invalid")
    return result


def ensure_labels(token: str, owner: str, repo: str, pr_number: int) -> None:
    for label in ("automated", "benchmark-data"):
        try:
            api_request(
                "POST",
                f"/repos/{owner}/{repo}/issues/{pr_number}/labels",
                token,
                body={"labels": [label]},
            )
        except RuntimeError:
            pass


def publish(
    repo_full: str,
    run_id: int,
    head_sha: str,
    run_number: int,
    dry_run: bool = False,
) -> dict[str, Any]:
    token = get_installation_token()
    verify_run(token, repo_full, run_id, head_sha, run_number)
    owner, repo = parse_repo(repo_full)
    branch = branch_name(run_number)

    existing = find_open_pr_for_branch(token, owner, repo, branch)
    if existing:
        return {
            "skipped": True,
            "reason": "open_pr_exists",
            "pr_url": existing.get("html_url"),
            "branch": branch,
        }

    if ref_exists(token, owner, repo, f"heads/{branch}"):
        existing = find_open_pr_for_branch(token, owner, repo, branch)
        if existing:
            return {
                "skipped": True,
                "reason": "branch_and_pr_exist",
                "pr_url": existing.get("html_url"),
            }

    zip_bytes = download_artifact_zip(token, repo_full, run_id)
    with tempfile.TemporaryDirectory() as tmp:
        work_dir = Path(tmp)
        extract_artifact_files(zip_bytes, work_dir)
        json_path = validate_benchmark_json(work_dir)
        badge_path = find_badge(work_dir)
        benchmark_json = json.loads(json_path.read_text(encoding="utf-8"))

        if dry_run:
            return {"ok": True, "dry_run": True, "run_id": run_id, "branch": branch}

        main_sha = get_main_sha(token, owner, repo)
        if not ref_exists(token, owner, repo, f"heads/{branch}"):
            create_branch(token, owner, repo, branch, main_sha)

        commit_msg = f"chore(bench): benchmark data update (run #{run_number})"
        json_sha = file_sha_on_branch(token, owner, repo, JSON_PATH, branch)
        badge_sha = file_sha_on_branch(token, owner, repo, BADGE_PATH, branch)
        put_file(
            token,
            owner,
            repo,
            JSON_PATH,
            branch,
            json_path.read_bytes(),
            commit_msg,
            json_sha,
        )
        put_file(
            token,
            owner,
            repo,
            BADGE_PATH,
            branch,
            badge_path.read_bytes(),
            commit_msg,
            badge_sha,
        )

    run_meta = api_request("GET", f"/repos/{owner}/{repo}/actions/runs/{run_id}", token)
    run_url = run_meta.get("html_url") if isinstance(run_meta, dict) else None
    body = render_data_pr_body(run_number, head_sha, run_url, benchmark_json)
    pr = open_pull_request(token, owner, repo, branch, run_number, body)
    pr_number = pr.get("number")
    if isinstance(pr_number, int):
        ensure_labels(token, owner, repo, pr_number)

    return {
        "ok": True,
        "branch": branch,
        "pr_url": pr.get("html_url"),
        "pr_number": pr_number,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Publish benchmark data PR on ibex-harness")
    parser.add_argument("--payload", help="JSON client_payload")
    parser.add_argument("--repo", default=os.environ.get("HARNESS_REPO", "Rick1330/ibex-harness"))
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    payload = load_payload(args.payload)
    run_id = require_int(payload.get("run_id"), "run_id")
    head_sha = require_sha(payload.get("head_sha"), "head_sha")
    run_number = require_int(payload.get("run_number"), "run_number")

    result = publish(args.repo, run_id, head_sha, run_number, dry_run=args.dry_run)
    print(json.dumps(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
