#!/usr/bin/env python3
"""Emit a fixed allowlist summary from sanitized Cloudflare and public-edge evidence."""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


def scalar(value: Any, default: str = "unknown") -> str:
    if value is None:
        return default
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (str, int, float)):
        return str(value).replace("\n", " ")[:500]
    return default


def hsts_max_age(edge: dict[str, Any]) -> Any:
    value = edge.get("zone_settings", {}).get("security_header", {}).get("value")
    if isinstance(value, dict):
        sts = value.get("strict_transport_security")
        if isinstance(sts, dict):
            return sts.get("max_age")
    return None


def candidate_count(edge: dict[str, Any]) -> int:
    total = 0
    for item in edge.get("ruleset_details", []):
        if isinstance(item, dict):
            candidates = item.get("oteryn_candidate_rules")
            if isinstance(candidates, list):
                total += len(candidates)
    return total


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: oteryn-public-edge-result.py EDGE_JSON PUBLIC_JSON")
    edge = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    public = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
    cert = edge.get("certificate_packs", {})
    bot = edge.get("bot_management", {}).get("settings", {})
    access = edge.get("access_applications", {})
    settings = edge.get("zone_settings", {})
    acceptance = public.get("acceptance", {})
    fields = {
        "operation_status": "success",
        "edge_observed_at": edge.get("observed_at_utc"),
        "edge_mutation": edge.get("mutation"),
        "gateway_certificate_active": cert.get("active_gateway_coverage"),
        "retired_gateway_certificate_active": cert.get("active_legacy_gateway_coverage"),
        "waf_candidate_count": candidate_count(edge),
        "bot_fight_mode": bot.get("fight_mode"),
        "bot_js_detections": bot.get("enable_js"),
        "browser_check": settings.get("browser_check", {}).get("value"),
        "security_level": settings.get("security_level", {}).get("value"),
        "always_use_https": settings.get("always_use_https", {}).get("value"),
        "hsts_max_age": hsts_max_age(edge),
        "access_canonical_application_count": len(access.get("oteryn_applications", [])),
        "public_observed_at": public.get("observed_at_utc"),
        "public_verdict": public.get("verdict"),
        "failed_required_checks": ",".join(public.get("failed_required_checks", [])) or "none",
        "dns_www": acceptance.get("dns_www"),
        "dns_gateway": acceptance.get("dns_gateway"),
        "tls_www": acceptance.get("tls_www"),
        "tls_gateway": acceptance.get("tls_gateway"),
        "www_browser_public": acceptance.get("www_browser_public"),
        "gateway_health": acceptance.get("gateway_health"),
        "gateway_ready": acceptance.get("gateway_ready"),
        "gateway_version": acceptance.get("gateway_version"),
        "gateway_invalid_login": acceptance.get("gateway_invalid_login"),
        "gateway_no_www_cross_route": acceptance.get("gateway_no_www_cross_route"),
        "www_no_gateway_cross_route": acceptance.get("www_no_gateway_cross_route"),
        "http_redirect_www": acceptance.get("http_redirect_www"),
        "http_redirect_gateway": acceptance.get("http_redirect_gateway"),
        "positive_hsts_www": acceptance.get("positive_hsts_www"),
        "public_mutation": public.get("mutation"),
    }
    for key, value in fields.items():
        print(f"{key}={scalar(value)}")


if __name__ == "__main__":
    main()
