#!/usr/bin/env python3
"""GET-only audit of whether the current account token can inspect its own policies."""
from __future__ import annotations

import json
import os
import re
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

BASE = os.environ.get("CLOUDFLARE_API_BASE_URL", "https://api.cloudflare.com/client/v4").rstrip("/")
TOKEN = os.environ.get("CLOUDFLARE_API_TOKEN", "")
ACCOUNT = os.environ.get("CLOUDFLARE_ACCOUNT_ID", "")
OUT = Path(os.environ.get("CLOUDFLARE_EDGE_AUDIT_OUT", "cloudflare-edge-audit"))


def fail(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def get(path: str) -> dict[str, Any]:
    request = urllib.request.Request(
        BASE + path,
        headers={"Authorization": f"Bearer {TOKEN}", "Accept": "application/json"},
        method="GET",
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            status = response.status
            raw = response.read(2_000_000)
    except urllib.error.HTTPError as exc:
        status = exc.code
        raw = exc.read(2_000_000)
    except Exception as exc:
        return {"state": "error", "http_status": 0, "errors": [{"message": f"{type(exc).__name__}: {exc}"}]}

    try:
        payload = json.loads(raw.decode("utf-8"))
    except Exception:
        return {"state": "error", "http_status": status, "errors": [{"message": "non-JSON response"}]}

    readable = 200 <= status < 300 and payload.get("success") is True
    state = "readable" if readable else "permission_denied" if status in (401, 403) else "not_found_or_unavailable" if status == 404 else "error"
    errors = [
        {"code": item.get("code"), "message": str(item.get("message", ""))[:300]}
        for item in payload.get("errors", [])
        if isinstance(item, dict)
    ]
    return {"state": state, "http_status": status, "result": payload.get("result") if readable else None, "errors": errors}


def main() -> None:
    if not TOKEN.startswith("cfat_"):
        fail("This capability audit requires the account-owned cfat_ token used by production-cloudflare")
    if not re.fullmatch(r"[0-9a-fA-F]{32}", ACCOUNT):
        fail("invalid CLOUDFLARE_ACCOUNT_ID")

    verification = get(f"/accounts/{ACCOUNT}/tokens/verify")
    if verification["state"] != "readable" or not isinstance(verification.get("result"), dict):
        fail(f"account token verification failed: HTTP {verification['http_status']}")
    token_id = verification["result"].get("id")
    if not isinstance(token_id, str) or not re.fullmatch(r"[0-9a-fA-F]{32}", token_id):
        fail("verified account token did not return a valid token identifier")

    details = get(f"/accounts/{ACCOUNT}/tokens/{token_id}")
    permission_groups = get(f"/accounts/{ACCOUNT}/tokens/permission_groups")

    assigned_names: list[str] = []
    if details["state"] == "readable" and isinstance(details.get("result"), dict):
        for policy in details["result"].get("policies", []):
            if not isinstance(policy, dict):
                continue
            for group in policy.get("permission_groups", []):
                if isinstance(group, dict) and isinstance(group.get("name"), str):
                    assigned_names.append(group["name"])

    assigned_names = sorted(set(assigned_names))
    evidence = {
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "classification": "READ_ONLY_CLOUDFLARE_TOKEN_CAPABILITY_AUDIT",
        "account_token_active": True,
        "self_details": {
            "state": details["state"],
            "http_status": details["http_status"],
            "assigned_permission_group_names": assigned_names,
            "has_account_api_tokens_read": any(name in {"Account API Tokens Read", "Account API Tokens Write"} for name in assigned_names),
            "has_account_api_tokens_write": "Account API Tokens Write" in assigned_names,
            "errors": details.get("errors", []),
        },
        "permission_group_catalog": {
            "state": permission_groups["state"],
            "http_status": permission_groups["http_status"],
            "errors": permission_groups.get("errors", []),
        },
        "mutation": "none",
    }

    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "token-capability.json").write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary = "\n".join(
        [
            "# Cloudflare account-token capability audit",
            "",
            f"Observed at: `{evidence['observed_at_utc']}`",
            "",
            f"- self_details: `{evidence['self_details']['state']}`",
            f"- permission_group_catalog: `{evidence['permission_group_catalog']['state']}`",
            f"- Account API Tokens Read proven: `{evidence['self_details']['has_account_api_tokens_read']}`",
            f"- Account API Tokens Write proven: `{evidence['self_details']['has_account_api_tokens_write']}`",
            "- mutation: `none`",
            "",
        ]
    )
    (OUT / "token-capability-summary.md").write_text(summary, encoding="utf-8")
    print(summary)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(summary)


if __name__ == "__main__":
    main()
