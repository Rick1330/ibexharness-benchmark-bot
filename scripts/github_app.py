"""GitHub App JWT and installation token helpers."""

from __future__ import annotations

import json
import os
import time
import urllib.error
import urllib.request
from typing import Any


API_VERSION = "2022-11-28"
DEFAULT_API = "https://api.github.com"


def _require_env(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise RuntimeError(f"missing required env: {name}")
    return value


def create_app_jwt(app_id: str, private_key_pem: str) -> str:
    try:
        import jwt
    except ImportError as exc:
        raise RuntimeError("PyJWT is required; pip install PyJWT[crypto]") from exc

    now = int(time.time())
    payload = {"iat": now - 60, "exp": now + 600, "iss": app_id}
    token = jwt.encode(payload, private_key_pem, algorithm="RS256")
    if isinstance(token, bytes):
        return token.decode("ascii")
    return token


def api_request(
    method: str,
    path: str,
    token: str,
    body: dict[str, Any] | None = None,
    api_base: str = DEFAULT_API,
) -> Any:
    url = f"{api_base}{path}" if path.startswith("/") else path
    data = None
    headers = {
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": API_VERSION,
        "User-Agent": "ibexharness-benchmark-bot",
    }
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(url, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            raw = response.read().decode("utf-8")
            if not raw:
                return None
            return json.loads(raw)
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"GitHub API {method} {path} failed ({exc.code}): {detail}") from exc


def get_installation_token(
    app_id: str | None = None,
    private_key: str | None = None,
    installation_id: str | None = None,
) -> str:
    app_id = app_id or _require_env("APP_ID")
    private_key = private_key or _require_env("APP_PRIVATE_KEY")
    installation_id = installation_id or _require_env("INSTALLATION_ID")
    jwt_token = create_app_jwt(app_id, private_key)
    result = api_request(
        "POST",
        f"/app/installations/{installation_id}/access_tokens",
        jwt_token,
    )
    if not isinstance(result, dict) or "token" not in result:
        raise RuntimeError("installation token response missing token field")
    return str(result["token"])


def parse_repo(full_name: str) -> tuple[str, str]:
    if "/" not in full_name:
        raise ValueError(f"invalid repo: {full_name}")
    owner, repo = full_name.split("/", 1)
    return owner, repo
