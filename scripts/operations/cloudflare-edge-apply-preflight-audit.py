#!/usr/bin/env python3
"""GET-only preflight for the fixed-scope Oteryn Cloudflare edge repair."""
from __future__ import annotations

import hashlib
import json
import os
import re
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

BASE = os.getenv("CLOUDFLARE_API_BASE_URL", "https://api.cloudflare.com/client/v4").rstrip("/")
TOKEN = os.getenv("CLOUDFLARE_API_TOKEN", "")
ZONE = os.getenv("CLOUDFLARE_ZONE_ID", "")
RULESET_ID = "67ca2e19272a4c7d97c2a53681d0eb2f"
RULE_ID = "e0f91939eb494d4490d975498a9a9724"
EXPECTED_EXPRESSION_SHA256 = "3f5a9e27f91d9cfe4fb6f77ede8c1e91997ef32a91a443cd1e6b61211ff13c45"
OUT = Path(os.getenv("CLOUDFLARE_EDGE_AUDIT_OUT", "cloudflare-edge-audit"))
FIELD_PREFIXES = (
    "http.",
    "ip.",
    "cf.",
    "ssl.",
    "raw.",
)
OPERATORS = (
    "eq",
    "ne",
    "in",
    "contains",
    "matches",
    "wildcard",
    "strict wildcard",
    "lt",
    "le",
    "gt",
    "ge",
    "not",
    "and",
    "or",
)


def die(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def get(path: str) -> dict:
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
        return {
            "status": 0,
            "state": "error",
            "errors": [{"message": f"{type(exc).__name__}: {exc}"}],
        }

    try:
        data = json.loads(raw)
    except Exception:
        return {
            "status": status,
            "state": "error",
            "errors": [{"message": "non-JSON response"}],
        }

    readable = 200 <= status < 300 and data.get("success") is True
    state = (
        "readable"
        if readable
        else "permission_denied"
        if status in (401, 403)
        else "not_found_or_unavailable"
        if status == 404
        else "error"
    )
    errors = [
        {"code": item.get("code"), "message": str(item.get("message", ""))[:300]}
        for item in data.get("errors", [])
        if isinstance(item, dict)
    ]
    return {
        "status": status,
        "state": state,
        "result": data.get("result") if readable else None,
        "errors": errors,
    }


def scalar_summary(value: object) -> object:
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if isinstance(value, dict):
        output: dict[str, object] = {}
        for key, item in value.items():
            if item is None or isinstance(item, (bool, int, float, str)):
                output[str(key)] = item
            elif isinstance(item, list):
                output[str(key)] = {
                    "count": len(item),
                    "scalar_values": [
                        element
                        for element in item
                        if element is None or isinstance(element, (bool, int, float, str))
                    ][:20],
                }
            elif isinstance(item, dict):
                output[str(key)] = scalar_summary(item)
        return output
    if isinstance(value, list):
        return {"count": len(value)}
    return str(type(value).__name__)


def quota_summary(response: dict) -> dict:
    output = {"state": response["state"], "http_status": response["status"]}
    if response["state"] == "readable":
        output["safe_result"] = scalar_summary(response.get("result"))
    else:
        output["errors"] = response["errors"]
    return output


def expression_signals(expression: str) -> dict:
    normalized = expression.lower()
    identifiers = sorted(
        {
            token.lower()
            for token in re.findall(r"[A-Za-z_][A-Za-z0-9_.]*", expression)
            if token.lower().startswith(FIELD_PREFIXES)
        }
    )
    operator_signals = sorted(
        operator
        for operator in OPERATORS
        if re.search(rf"(?<![A-Za-z0-9_]){re.escape(operator)}(?![A-Za-z0-9_])", normalized)
    )
    return {
        "fields": identifiers,
        "operators": operator_signals,
        "contains_country_field": any("country" in field for field in identifiers),
        "contains_ip_source_field": any(field.startswith("ip.src") for field in identifiers),
        "contains_bot_field": any(field.startswith("cf.bot") or field == "cf.client.bot" for field in identifiers),
        "contains_user_agent_field": any("user_agent" in field for field in identifiers),
        "contains_path_field": any("uri.path" in field for field in identifiers),
        "contains_method_field": any("request.method" in field for field in identifiers),
        "contains_host_field": "http.host" in identifiers,
        "literal_ipv4_count": len(re.findall(r"(?<!\d)(?:\d{1,3}\.){3}\d{1,3}(?!\d)", expression)),
        "quoted_literal_count": len(re.findall(r'"(?:[^"\\]|\\.)*"', expression)),
        "expression_length": len(expression),
    }


def safe_action_parameters(value: object) -> dict:
    if not isinstance(value, dict):
        return {}
    output: dict[str, object] = {}
    for key in ("ruleset", "phase"):
        item = value.get(key)
        if isinstance(item, str):
            output[key] = item
    for key in ("phases", "products"):
        item = value.get(key)
        if isinstance(item, list):
            output[key] = [str(element) for element in item[:20]]
    return output


def rule_summary(response: dict) -> dict:
    output = {"state": response["state"], "http_status": response["status"]}
    if response["state"] != "readable" or not isinstance(response.get("result"), dict):
        output["errors"] = response["errors"]
        return output

    result = response["result"]
    rules = result.get("rules", [])
    rules = rules if isinstance(rules, list) else []
    matching = [
        item
        for item in rules
        if isinstance(item, dict) and str(item.get("id")) == RULE_ID
    ]
    if len(matching) != 1:
        output.update(rule_found=False, matching_rule_count=len(matching))
        return output

    rule = matching[0]
    expression = str(rule.get("expression", ""))
    expression_hash = hashlib.sha256(expression.encode("utf-8")).hexdigest()
    output.update(
        rule_found=True,
        ruleset_id=RULESET_ID,
        rule_id=RULE_ID,
        action=rule.get("action"),
        enabled=rule.get("enabled", True),
        ref=rule.get("ref"),
        expression_sha256=expression_hash,
        expected_expression_sha256=EXPECTED_EXPRESSION_SHA256,
        expression_fingerprint_matches=expression_hash == EXPECTED_EXPRESSION_SHA256,
        expression_signals=expression_signals(expression),
        action_parameter_keys=(
            sorted(str(key) for key in rule.get("action_parameters", {}).keys())
            if isinstance(rule.get("action_parameters"), dict)
            else []
        ),
        safe_action_parameters=safe_action_parameters(rule.get("action_parameters")),
        description_sha256=hashlib.sha256(
            str(rule.get("description", "")).encode("utf-8")
        ).hexdigest(),
    )
    return output


def main() -> None:
    if not TOKEN:
        die("CLOUDFLARE_API_TOKEN is missing")
    if not re.fullmatch(r"[0-9a-fA-F]{32}", ZONE):
        die("invalid CLOUDFLARE_ZONE_ID")

    verification = get("/user/tokens/verify")
    if (
        verification["state"] != "readable"
        or not isinstance(verification.get("result"), dict)
        or verification["result"].get("status") != "active"
    ):
        die(f"user token verification failed: HTTP {verification['status']}")

    evidence = {
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "classification": "READ_ONLY_CLOUDFLARE_EDGE_APPLY_PREFLIGHT",
        "certificate_pack_quota": quota_summary(
            get(f"/zones/{ZONE}/ssl/certificate_packs/quota")
        ),
        "blocking_rule": rule_summary(get(f"/zones/{ZONE}/rulesets/{RULESET_ID}")),
        "mutation": "none",
    }

    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "apply-preflight.json").write_text(
        json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    quota = evidence["certificate_pack_quota"]
    rule = evidence["blocking_rule"]
    text = "\n".join(
        [
            "# Cloudflare Oteryn edge apply preflight",
            "",
            f"Observed at: `{evidence['observed_at_utc']}`",
            "",
            f"- certificate pack quota: `{quota['state']}`",
            f"- blocking rule readable: `{rule['state']}`",
            f"- blocking rule found: `{rule.get('rule_found', False)}`",
            f"- expression fingerprint matches: `{rule.get('expression_fingerprint_matches', False)}`",
            "",
            "No raw rule expression or token value is emitted.",
            "This preflight performs GET requests only.",
            "",
        ]
    )
    (OUT / "apply-preflight-summary.md").write_text(text, encoding="utf-8")
    print(text)
    if os.getenv("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(text)


if __name__ == "__main__":
    main()
