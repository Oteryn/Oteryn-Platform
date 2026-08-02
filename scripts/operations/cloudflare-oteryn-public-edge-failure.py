#!/usr/bin/env python3
"""Classify a Cloudflare repair failure into fixed, non-secret status fields."""
from __future__ import annotations

import re
import sys
from pathlib import Path

HTTP_FAILURE = re.compile(
    r"Cloudflare\s+(GET|POST|PUT|DELETE)\s+(\S+)\s+failed:\s+HTTP\s+(\d+);\s+codes=([0-9A-Za-z_,.-]+)"
)


def phase(method: str, path: str) -> str:
    if method == "POST" and path.endswith("/rules") and "/rulesets/" in path:
        return "create_waf_skip_rule"
    if method == "DELETE" and "/rulesets/" in path and "/rules/" in path:
        return "delete_waf_skip_rule"
    if method == "PUT" and path.endswith("/bot_management"):
        return "update_bot_fight_mode"
    if method == "GET" and path.endswith("/bot_management"):
        return "read_bot_management"
    if method == "GET" and "/rulesets/" in path:
        return "read_waf_ruleset"
    return "cloudflare_request"


def classify(text: str) -> dict[str, str]:
    match = HTTP_FAILURE.search(text)
    if match:
        method, path, status, codes = match.groups()
        rollback = "reported_complete" if "partial changes were rolled back" in text else "not_reported"
        return {
            "failure_phase": phase(method, path),
            "error_class": "cloudflare_http_error",
            "http_status": status,
            "error_codes": codes[:200],
            "rollback_claim": rollback,
        }

    known = (
        ("Cloudflare did not return the created rule", "create_waf_skip_rule", "unexpected_api_response"),
        ("unexpected created rule reference", "create_waf_skip_rule", "unexpected_api_response"),
        ("candidate ID does not match", "preflight", "audited_rule_drift"),
        ("expression hash does not match", "preflight", "audited_rule_drift"),
        ("multiple Oteryn repair rules", "preflight", "ambiguous_state"),
        ("expected exactly one broad country block candidate", "preflight", "ambiguous_state"),
        ("repair rule is not exact/current", "preflight", "managed_rule_drift"),
        ("apply confirmation is invalid", "authorization", "invalid_confirmation"),
        ("rollback confirmation is invalid", "authorization", "invalid_confirmation"),
        ("transport error", "cloudflare_request", "transport_error"),
        ("non-JSON HTTP", "cloudflare_request", "non_json_response"),
    )
    for marker, failure_phase, error_class in known:
        if marker in text:
            return {
                "failure_phase": failure_phase,
                "error_class": error_class,
                "http_status": "unknown",
                "error_codes": "unknown",
                "rollback_claim": (
                    "reported_complete" if "partial changes were rolled back" in text else "not_reported"
                ),
            }
    return {
        "failure_phase": "unknown",
        "error_class": "unclassified_failure",
        "http_status": "unknown",
        "error_codes": "unknown",
        "rollback_claim": "not_reported",
    }


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: cloudflare-oteryn-public-edge-failure.py RAW_OUTPUT")
    text = Path(sys.argv[1]).read_text(encoding="utf-8", errors="replace")[-20_000:]
    for key, value in classify(text).items():
        print(f"{key}={value}")


if __name__ == "__main__":
    main()
