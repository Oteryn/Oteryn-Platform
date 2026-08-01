#!/usr/bin/env python3
"""GET-only sanitized audit of the remaining Oteryn Cloudflare edge controls."""
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
ACCESS_TOKEN = os.getenv("CLOUDFLARE_ACCESS_API_TOKEN", TOKEN)
ACCOUNT = os.getenv("CLOUDFLARE_ACCOUNT_ID", "")
ZONE = os.getenv("CLOUDFLARE_ZONE_ID", "")
WWW = "oteryn.molehill.cloud"
GATEWAY = "gateway.molehill.cloud"
LEGACY_GATEWAY = "login.oteryn.molehill.cloud"
OUT = Path(os.getenv("CLOUDFLARE_EDGE_AUDIT_OUT", "cloudflare-edge-audit"))
PHASES = {
    "http_request_dynamic_redirect",
    "http_request_firewall_custom",
    "http_response_headers_transform",
    "http_config_settings",
}
CHALLENGE_ACTIONS = {"block", "challenge", "js_challenge", "managed_challenge"}


def die(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def call(path: str, token: str | None = None) -> dict:
    selected_token = TOKEN if token is None else token
    if not selected_token:
        return {"status": 0, "state": "error", "errors": [{"message": "token is missing"}]}
    request = urllib.request.Request(
        BASE + path,
        headers={"Authorization": f"Bearer {selected_token}", "Accept": "application/json"},
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
        return {"status": status, "state": "error", "errors": [{"message": "non-JSON response"}]}

    readable = 200 <= status < 300 and data.get("success") is True
    if readable:
        state = "readable"
    elif status in (401, 403):
        state = "permission_denied"
    elif status == 404:
        state = "not_found_or_unavailable"
    else:
        state = "error"
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


def hostname_covered(pattern: str, hostname: str) -> bool:
    """Match exact hosts and one-label wildcard certificate names only."""
    pattern = pattern.lower().rstrip(".")
    hostname = hostname.lower().rstrip(".")
    if pattern == hostname:
        return True
    if not pattern.startswith("*."):
        return False
    suffix = pattern[2:]
    if not hostname.endswith("." + suffix):
        return False
    prefix = hostname[: -(len(suffix) + 1)]
    return bool(prefix) and "." not in prefix


def certs(response: dict) -> dict:
    output = {"state": response["state"], "http_status": response["status"]}
    if response["state"] != "readable":
        output["errors"] = response["errors"]
        return output

    packs = response["result"] if isinstance(response["result"], list) else []
    summaries = []
    gateway_matching = []
    for item in packs:
        if not isinstance(item, dict):
            continue
        hosts = [str(host).lower().rstrip(".") for host in item.get("hosts", []) if isinstance(host, str)]
        summary = {
            "id": item.get("id"),
            "type": item.get("type"),
            "status": item.get("status"),
            "host_count": len(hosts),
            "covers_www": any(hostname_covered(host, WWW) for host in hosts),
            "covers_gateway": any(hostname_covered(host, GATEWAY) for host in hosts),
            "covers_legacy_gateway": any(hostname_covered(host, LEGACY_GATEWAY) for host in hosts),
        }
        summaries.append(summary)
        if summary["covers_gateway"]:
            gateway_matching.append(summary)

    output.update(
        pack_count=len(packs),
        pack_summaries=summaries,
        gateway_matching_packs=gateway_matching,
        active_gateway_coverage=any(
            str(item.get("status", "")).lower() == "active" for item in gateway_matching
        ),
        active_legacy_gateway_coverage=any(
            str(item.get("status", "")).lower() == "active" and item["covers_legacy_gateway"]
            for item in summaries
        ),
    )
    return output


def setting(name: str, response: dict) -> dict:
    output = {"state": response["state"], "http_status": response["status"]}
    if response["state"] == "readable" and isinstance(response["result"], dict):
        output.update(
            id=response["result"].get("id", name),
            value=response["result"].get("value"),
            editable=response["result"].get("editable"),
        )
    else:
        output["errors"] = response["errors"]
    return output


def rulesets(response: dict) -> tuple[dict, list[dict]]:
    output = {"state": response["state"], "http_status": response["status"]}
    selected: list[dict] = []
    if response["state"] != "readable":
        output["errors"] = response["errors"]
        return output, selected

    for item in response["result"] if isinstance(response["result"], list) else []:
        if isinstance(item, dict) and item.get("phase") in PHASES:
            selected.append({key: item.get(key) for key in ("id", "phase", "kind", "name")})
    output["relevant_rulesets"] = selected
    return output, selected


def sanitized_rule(rule: dict) -> dict:
    expression = str(rule.get("expression", ""))
    normalized = expression.lower()
    mentions_http_host = "http.host" in normalized
    mentions_zone_domain = "molehill.cloud" in normalized
    matches_www = WWW in normalized
    matches_gateway = GATEWAY in normalized
    matches_legacy_gateway = LEGACY_GATEWAY in normalized

    if matches_www or matches_gateway:
        host_scope = "explicit_canonical_host"
    elif matches_legacy_gateway:
        host_scope = "explicit_retired_host"
    elif mentions_http_host and mentions_zone_domain:
        host_scope = "zone_domain_scope"
    elif mentions_http_host:
        host_scope = "other_host_scope"
    else:
        host_scope = "broad_no_host_predicate"

    action = str(rule.get("action", ""))
    action_parameters = rule.get("action_parameters")
    action_parameter_keys = (
        sorted(str(key) for key in action_parameters.keys())
        if isinstance(action_parameters, dict)
        else []
    )
    potentially_applies = host_scope not in {"other_host_scope", "explicit_retired_host"}
    return {
        "id": rule.get("id"),
        "ref": rule.get("ref"),
        "action": action,
        "enabled": rule.get("enabled", True),
        "host_scope": host_scope,
        "mentions_http_host": mentions_http_host,
        "mentions_zone_domain": mentions_zone_domain,
        "matches_www": matches_www,
        "matches_gateway": matches_gateway,
        "matches_legacy_gateway": matches_legacy_gateway,
        "potentially_applies_to_oteryn": potentially_applies,
        "challenge_or_block_action": action in CHALLENGE_ACTIONS,
        "action_parameter_keys": action_parameter_keys,
        "expression_sha256": hashlib.sha256(expression.encode("utf-8")).hexdigest(),
    }


def rules_detail(metadata: dict, response: dict) -> dict:
    output = {
        "id": metadata.get("id"),
        "phase": metadata.get("phase"),
        "state": response["state"],
        "http_status": response["status"],
    }
    if response["state"] != "readable" or not isinstance(response["result"], dict):
        output["errors"] = response["errors"]
        return output

    rules = response["result"].get("rules", [])
    rules = rules if isinstance(rules, list) else []
    sanitized = [sanitized_rule(rule) for rule in rules if isinstance(rule, dict)]
    output.update(
        rule_count=len(rules),
        sanitized_rules=sanitized,
        oteryn_matching_rules=[
            rule for rule in sanitized if rule["matches_www"] or rule["matches_gateway"]
        ],
        retired_gateway_rules=[rule for rule in sanitized if rule["matches_legacy_gateway"]],
        oteryn_candidate_rules=[
            rule
            for rule in sanitized
            if rule["enabled"]
            and rule["potentially_applies_to_oteryn"]
            and rule["challenge_or_block_action"]
        ],
    )
    return output


def bot(response: dict) -> dict:
    output = {"state": response["state"], "http_status": response["status"]}
    keys = (
        "fight_mode",
        "sbfm_likely_automated",
        "sbfm_definitely_automated",
        "sbfm_verified_bots",
        "sbfm_static_resource_protection",
        "enable_js",
    )
    if response["state"] != "readable" or not isinstance(response["result"], dict):
        output["errors"] = response["errors"]
        return output
    output["settings"] = {
        key: response["result"].get(key) for key in keys if key in response["result"]
    }
    return output


def access(response: dict) -> dict:
    output = {"state": response["state"], "http_status": response["status"]}
    if response["state"] != "readable":
        output["errors"] = response["errors"]
        return output
    applications = response["result"] if isinstance(response["result"], list) else []
    matching = []
    retired = []
    for item in applications:
        if not isinstance(item, dict):
            continue
        domain = str(item.get("domain", "")).lower()
        summary = {"id": item.get("id"), "domain": domain, "type": item.get("type")}
        if domain in (WWW, GATEWAY) or domain.startswith(WWW + "/") or domain.startswith(GATEWAY + "/"):
            matching.append(summary)
        if domain == LEGACY_GATEWAY or domain.startswith(LEGACY_GATEWAY + "/"):
            retired.append(summary)
    output.update(
        application_count=len(applications),
        oteryn_applications=matching,
        retired_gateway_applications=retired,
    )
    return output


def main() -> None:
    if not TOKEN:
        die("CLOUDFLARE_API_TOKEN is missing")
    if not re.fullmatch(r"[0-9a-fA-F]{32}", ACCOUNT):
        die("invalid CLOUDFLARE_ACCOUNT_ID")
    if not re.fullmatch(r"[0-9a-fA-F]{32}", ZONE):
        die("invalid CLOUDFLARE_ZONE_ID")

    verification_path = (
        f"/accounts/{ACCOUNT}/tokens/verify" if TOKEN.startswith("cfat_") else "/user/tokens/verify"
    )
    verification = call(verification_path)
    if (
        verification["state"] != "readable"
        or not isinstance(verification["result"], dict)
        or verification["result"].get("status") != "active"
    ):
        die(f"token verification failed: HTTP {verification['status']}")

    settings = {
        name: setting(name, call(f"/zones/{ZONE}/settings/{name}"))
        for name in (
            "always_use_https",
            "min_tls_version",
            "security_level",
            "browser_check",
            "security_header",
        )
    }
    ruleset_summary, selected = rulesets(call(f"/zones/{ZONE}/rulesets"))
    details = [
        rules_detail(item, call(f"/zones/{ZONE}/rulesets/{item['id']}"))
        for item in selected
        if item.get("id")
    ]
    evidence = {
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "classification": "READ_ONLY_CLOUDFLARE_EDGE_AUDIT",
        "canonical_hosts": [WWW, GATEWAY],
        "retired_hosts": [LEGACY_GATEWAY],
        "token": {
            "active": True,
            "verification_scope": "account" if TOKEN.startswith("cfat_") else "user",
            "access_token_separate": ACCESS_TOKEN != TOKEN,
        },
        "certificate_packs": certs(
            call(f"/zones/{ZONE}/ssl/certificate_packs?status=all&per_page=100")
        ),
        "zone_settings": settings,
        "rulesets": ruleset_summary,
        "ruleset_details": details,
        "bot_management": bot(call(f"/zones/{ZONE}/bot_management")),
        "access_applications": access(
            call(f"/accounts/{ACCOUNT}/access/apps?per_page=100", ACCESS_TOKEN)
        ),
        "mutation": "none",
    }

    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "evidence.json").write_text(
        json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    candidate_count = sum(len(item.get("oteryn_candidate_rules", [])) for item in details)
    certificate_packs = evidence["certificate_packs"]
    lines = [
        "# Cloudflare Oteryn edge audit",
        "",
        f"Observed at: `{evidence['observed_at_utc']}`",
        "",
        f"- certificate_packs: `{certificate_packs['state']}`; active Gateway coverage: `{certificate_packs.get('active_gateway_coverage', 'unknown')}`; active retired-host coverage: `{certificate_packs.get('active_legacy_gateway_coverage', 'unknown')}`",
        f"- rulesets: `{ruleset_summary['state']}`; relevant count: `{len(ruleset_summary.get('relevant_rulesets', []))}`; challenge/block candidates: `{candidate_count}`",
        f"- bot_management: `{evidence['bot_management']['state']}`",
        f"- access_applications: `{evidence['access_applications']['state']}`",
    ]
    lines += [
        f"- zone setting `{name}`: `{item['state']}`; value: `{item.get('value', 'unknown')}`"
        for name, item in settings.items()
    ]
    lines += [
        "",
        "Certificate wildcard matching is limited to exactly one label.",
        "Expressions are never emitted; only safe classifications and SHA-256 fingerprints are stored.",
        "This audit performs GET requests only and writes a sanitized artifact.",
        "",
    ]
    text = "\n".join(lines)
    (OUT / "summary.md").write_text(text, encoding="utf-8")
    print(text)
    if os.getenv("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(text)


if __name__ == "__main__":
    main()
