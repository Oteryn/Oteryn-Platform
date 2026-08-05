#!/usr/bin/env python3
"""Bounded staged HSTS audit/apply/rollback for the Oteryn Cloudflare zone."""
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

BASE = os.getenv("CLOUDFLARE_API_BASE_URL", "https://api.cloudflare.com/client/v4").rstrip("/")
TOKEN = os.getenv("CLOUDFLARE_API_TOKEN", "")
ZONE = os.getenv("CLOUDFLARE_ZONE_ID", "")
OUT = Path(os.getenv("CLOUDFLARE_HSTS_STAGE1_OUT", "cloudflare-hsts-stage1"))
CONFIRMATION = os.getenv("CLOUDFLARE_HSTS_CONFIRMATION", "")
APPLY_CONFIRMATION = "APPLY-OTERYN-HSTS-STAGE1"
ROLLBACK_CONFIRMATION = "ROLLBACK-OTERYN-HSTS-STAGE1"

# Exact live baseline proven before staged promotion. max_age=0 disables browser persistence.
BASELINE = {
    "enabled": True,
    "include_subdomains": True,
    "max_age": 0,
    "nosniff": True,
    "preload": True,
}

# Conservative first stage: one month, no parent-domain inheritance and no preload.
TARGET = {
    "enabled": True,
    "include_subdomains": False,
    "max_age": 2_592_000,
    "nosniff": True,
    "preload": False,
}


class HstsError(RuntimeError):
    pass


def api(method: str, path: str, body: dict[str, Any] | None = None) -> dict[str, Any]:
    if not TOKEN:
        raise HstsError("CLOUDFLARE_API_TOKEN is missing")
    data = None
    headers = {"Authorization": f"Bearer {TOKEN}", "Accept": "application/json"}
    if body is not None:
        data = json.dumps(body, separators=(",", ":")).encode("utf-8")
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(BASE + path, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            status = response.status
            raw = response.read(2_000_000)
    except urllib.error.HTTPError as exc:
        status = exc.code
        raw = exc.read(2_000_000)
    except Exception as exc:  # pragma: no cover - network boundary
        raise HstsError(f"Cloudflare transport error: {type(exc).__name__}") from exc

    try:
        payload = json.loads(raw) if raw else {"success": 200 <= status < 300, "result": None}
    except Exception as exc:
        raise HstsError(f"Cloudflare returned non-JSON HTTP {status}") from exc
    if not (200 <= status < 300 and payload.get("success") is True):
        codes = [str(item.get("code")) for item in payload.get("errors", []) if isinstance(item, dict)]
        code_text = ",".join(codes[:5]) or "unknown"
        raise HstsError(f"Cloudflare {method} {path} failed: HTTP {status}; codes={code_text}")
    return {"result": payload.get("result"), "status": status}


def verify_token() -> None:
    path = "/user/tokens/verify" if not TOKEN.startswith("cfat_") else "/tokens/verify"
    result = api("GET", path).get("result")
    if not isinstance(result, dict) or result.get("status") != "active":
        raise HstsError("Cloudflare token is not proven active")


def normalize(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise HstsError("HSTS value is unavailable")
    required = {"enabled", "include_subdomains", "max_age", "nosniff", "preload"}
    if not required.issubset(value):
        raise HstsError("HSTS value is incomplete")
    normalized = {
        "enabled": value.get("enabled"),
        "include_subdomains": value.get("include_subdomains"),
        "max_age": value.get("max_age"),
        "nosniff": value.get("nosniff"),
        "preload": value.get("preload"),
    }
    if not all(isinstance(normalized[key], bool) for key in ("enabled", "include_subdomains", "nosniff", "preload")):
        raise HstsError("HSTS boolean fields are invalid")
    if not isinstance(normalized["max_age"], (int, float)) or isinstance(normalized["max_age"], bool):
        raise HstsError("HSTS max_age is invalid")
    normalized["max_age"] = int(normalized["max_age"])
    return normalized


def read_setting() -> dict[str, Any]:
    result = api("GET", f"/zones/{ZONE}/settings/security_header").get("result")
    if not isinstance(result, dict) or result.get("id") != "security_header":
        raise HstsError("security_header setting is unavailable")
    if result.get("editable") is False:
        raise HstsError("security_header setting is not editable for this zone")
    strict = result.get("value", {}).get("strict_transport_security") if isinstance(result.get("value"), dict) else None
    return normalize(strict)


def write_setting(value: dict[str, Any]) -> None:
    result = api(
        "PATCH",
        f"/zones/{ZONE}/settings/security_header",
        {"value": {"strict_transport_security": value}},
    ).get("result")
    if not isinstance(result, dict) or result.get("id") != "security_header":
        raise HstsError("Cloudflare did not return the updated security_header setting")
    strict = result.get("value", {}).get("strict_transport_security") if isinstance(result.get("value"), dict) else None
    if normalize(strict) != value:
        raise HstsError("Cloudflare returned an unexpected HSTS value")


def classify(value: dict[str, Any]) -> str:
    if value == BASELINE:
        return "baseline"
    if value == TARGET:
        return "staged"
    return "drift"


def restore_baseline_after_failure() -> str:
    observed = read_setting()
    state = classify(observed)
    if state == "baseline":
        return "already_baseline"
    if state != "staged":
        raise HstsError("automatic rollback refused unexpected HSTS drift")
    write_setting(BASELINE)
    if read_setting() != BASELINE:
        raise HstsError("automatic HSTS rollback verification failed")
    return "baseline_restored"


def apply() -> tuple[dict[str, Any], list[str]]:
    if CONFIRMATION != APPLY_CONFIRMATION:
        raise HstsError("apply confirmation is invalid")
    current = read_setting()
    state = classify(current)
    if state == "staged":
        return current, []
    if state != "baseline":
        raise HstsError("refusing HSTS apply from an unknown baseline")
    try:
        write_setting(TARGET)
        final = read_setting()
        if final != TARGET:
            raise HstsError("post-write HSTS verification did not reach the staged target")
        return final, ["hsts_stage1_enabled"]
    except Exception as exc:
        rollback = restore_baseline_after_failure()
        raise HstsError(f"HSTS apply failed and rollback completed ({rollback}): {exc}") from exc


def rollback() -> tuple[dict[str, Any], list[str]]:
    if CONFIRMATION != ROLLBACK_CONFIRMATION:
        raise HstsError("rollback confirmation is invalid")
    current = read_setting()
    state = classify(current)
    if state == "baseline":
        return current, []
    if state != "staged":
        raise HstsError("refusing HSTS rollback from an unknown state")
    write_setting(BASELINE)
    final = read_setting()
    if final != BASELINE:
        raise HstsError("post-rollback HSTS verification did not reach the baseline")
    return final, ["hsts_stage1_rolled_back"]


def evidence(mode: str, value: dict[str, Any], mutations: list[str]) -> dict[str, Any]:
    state = classify(value)
    return {
        "schema_version": 1,
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "classification": "CLOUDFLARE_HSTS_STAGE1",
        "operation_status": "success",
        "mode": mode,
        "state": state,
        "enabled": value["enabled"],
        "max_age": value["max_age"],
        "include_subdomains": value["include_subdomains"],
        "preload": value["preload"],
        "nosniff": value["nosniff"],
        "desired_state": state == "staged",
        "mutations": mutations,
        "mutation": "none" if not mutations else ",".join(mutations),
        "secrets_emitted": False,
    }


def emit(item: dict[str, Any]) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "evidence.json").write_text(json.dumps(item, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (OUT / "summary.md").write_text(
        "\n".join(
            [
                "# Cloudflare Oteryn HSTS stage 1",
                "",
                f"Observed at: `{item['observed_at_utc']}`",
                f"Mode: `{item['mode']}`",
                f"State: `{item['state']}`",
                f"Max age: `{item['max_age']}`",
                f"Include subdomains: `{item['include_subdomains']}`",
                f"Preload: `{item['preload']}`",
                f"Mutation: `{item['mutation']}`",
                "",
                "No token or raw API response is emitted.",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    for key in (
        "operation_status",
        "mode",
        "state",
        "enabled",
        "max_age",
        "include_subdomains",
        "preload",
        "nosniff",
        "desired_state",
        "mutation",
    ):
        value = item[key]
        if isinstance(value, bool):
            value = "true" if value else "false"
        print(f"{key}={value}")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] not in {"audit", "apply", "rollback"}:
        raise SystemExit("usage: cloudflare-oteryn-hsts-stage1.py audit|apply|rollback")
    if not re.fullmatch(r"[0-9a-fA-F]{32}", ZONE):
        raise SystemExit("ERROR: invalid CLOUDFLARE_ZONE_ID")
    mode = sys.argv[1]
    try:
        verify_token()
        if mode == "audit":
            value = read_setting()
            mutations: list[str] = []
        elif mode == "apply":
            value, mutations = apply()
        else:
            value, mutations = rollback()
        emit(evidence(mode, value, mutations))
    except HstsError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc


if __name__ == "__main__":
    main()
