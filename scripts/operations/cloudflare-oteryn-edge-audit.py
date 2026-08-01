#!/usr/bin/env python3
"""Read-only, sanitized Cloudflare edge audit for canonical Oteryn hosts."""

from __future__ import annotations

import hashlib
import json
import os
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

API_BASE = os.environ.get("CLOUDFLARE_API_BASE_URL", "https://api.cloudflare.com/client/v4").rstrip("/")
WWW_HOST = "oteryn.molehill.cloud"
LOGIN_HOST = "login.oteryn.molehill.cloud"
CANONICAL_HOSTS = (WWW_HOST, LOGIN_HOST)
RELEVANT_PHASES = {
    "http_request_firewall_custom",
    "http_request_sbfm",
    "http_request_dynamic_redirect",
    "http_request_redirect",
    "http_response_headers_transform",
    "http_config_settings",
}
CHALLENGE_ACTIONS = {"block", "challenge", "js_challenge", "managed_challenge"}
OUT = Path(os.environ.get("CLOUDFLARE_EDGE_AUDIT_OUT", "cloudflare-edge-audit"))


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def digest(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode()).hexdigest()


def hash_id(value: Any) -> str | None:
    if not value:
        return None
    return hashlib.sha256(str(value).encode()).hexdigest()[:16]


def env_required(name: str) -> str:
    value = os.environ.get(name, "")
    if not value:
        raise SystemExit(f"required environment variable is missing: {name}")
    return value


@dataclass
class ApiResult:
    path: str
    ok: bool
    http_status: int
    result: Any = None
    errors: list[dict[str, Any]] | None = None
    transport_error: str | None = None

    @property
    def permission_denied(self) -> bool:
        return self.http_status in {401, 403}

    def sanitized_status(self) -> dict[str, Any]:
        return {
            "ok": self.ok,
            "http_status": self.http_status,
            "permission_denied": self.permission_denied,
            "transport_error": self.transport_error,
            "errors": [
                {"code": item.get("code"), "message": str(item.get("message", ""))[:300]}
                for item in (self.errors or [])[:5]
            ],
        }


class CloudflareClient:
    def __init__(self, token: str, account_id: str, zone_id: str) -> None:
        self._token = token
        self.account_id = account_id
        self.zone_id = zone_id
        self.requested_paths: list[str] = []

    def get(self, path: str, *, optional: bool = False) -> ApiResult:
        if not path.startswith("/"):
            raise ValueError("Cloudflare path must be absolute")
        self.requested_paths.append(path)
        request = urllib.request.Request(
            f"{API_BASE}{path}",
            method="GET",
            headers={
                "Authorization": f"Bearer {self._token}",
                "Accept": "application/json",
                "User-Agent": "Oteryn-Cloudflare-Edge-Audit/1.0",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                status = response.status
                payload = json.loads(response.read().decode("utf-8"))
        except urllib.error.HTTPError as exc:
            status = exc.code
            try:
                payload = json.loads(exc.read().decode("utf-8"))
            except (json.JSONDecodeError, UnicodeDecodeError):
                payload = {"success": False, "errors": [{"code": status, "message": "non-JSON API error"}]}
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            if optional:
                return ApiResult(path, False, 0, transport_error=f"{type(exc).__name__}: {exc}")
            raise SystemExit(f"Cloudflare API transport failed for GET {path}: {type(exc).__name__}") from exc

        ok = 200 <= status < 300 and payload.get("success") is True
        result = ApiResult(
            path=path,
            ok=ok,
            http_status=status,
            result=payload.get("result"),
            errors=payload.get("errors") or [],
        )
        if not ok and not optional:
            summary = result.sanitized_status()
            raise SystemExit(f"Cloudflare API required GET failed for {path}: {canonical_json(summary)}")
        return result


def exact_host_matches(value: str) -> list[str]:
    lowered = value.lower()
    return [host for host in CANONICAL_HOSTS if host in lowered]


def certificate_name_covers(hostname: str, certificate_name: str) -> bool:
    hostname = hostname.lower().rstrip(".")
    certificate_name = certificate_name.lower().rstrip(".")
    if hostname == certificate_name:
        return True
    if not certificate_name.startswith("*."):
        return False
    suffix = certificate_name[2:]
    if not hostname.endswith(f".{suffix}"):
        return False
    return hostname.count(".") == suffix.count(".") + 1


def sanitize_certificate_packs(result: Any) -> dict[str, Any]:
    packs = result if isinstance(result, list) else []
    matched: list[dict[str, Any]] = []
    coverage = {host: False for host in CANONICAL_HOSTS}
    for pack in packs:
        if not isinstance(pack, dict):
            continue
        hosts = [str(item).lower() for item in (pack.get("hosts") or [])]
        certificates = pack.get("certificates") or []
        cert_hosts: list[str] = []
        cert_statuses: list[str] = []
        for cert in certificates:
            if not isinstance(cert, dict):
                continue
            cert_hosts.extend(str(item).lower() for item in (cert.get("hosts") or []))
            if cert.get("status"):
                cert_statuses.append(str(cert["status"]))
        all_hosts = sorted(set(hosts + cert_hosts))
        canonical = sorted(
            {host for host in CANONICAL_HOSTS if any(certificate_name_covers(host, name) for name in all_hosts)}
        )
        if not canonical:
            continue
        pack_status = str(pack.get("status") or "")
        active = pack_status.lower() in {"active", "ready"} or any(
            status.lower() in {"active", "ready"} for status in cert_statuses
        )
        for host in canonical:
            coverage[host] = coverage[host] or active
        matched.append(
            {
                "id_hash": hash_id(pack.get("id")),
                "type": pack.get("type"),
                "status": pack_status,
                "canonical_hosts_covered": canonical,
                "certificate_names": [
                    name for name in all_hosts if any(certificate_name_covers(host, name) for host in canonical)
                ],
                "certificate_statuses": sorted(set(cert_statuses)),
                "validation_method": pack.get("validation_method"),
                "validity_days": pack.get("validity_days"),
                "certificate_authority": pack.get("certificate_authority"),
            }
        )
    return {"matched_packs": matched, "active_hostname_coverage": coverage, "total_pack_count": len(packs)}


def sanitize_ssl_verification(result: Any) -> dict[str, Any]:
    records = result if isinstance(result, list) else []
    matched = []
    for item in records:
        if not isinstance(item, dict):
            continue
        hostname = str(item.get("hostname") or item.get("host") or "").lower()
        if hostname not in CANONICAL_HOSTS:
            continue
        matched.append(
            {
                "hostname": hostname,
                "status": item.get("status"),
                "validation_method": item.get("validation_method"),
                "certificate_status": item.get("certificate_status"),
                "brand_check": item.get("brand_check"),
            }
        )
    return {"matched": matched, "total_record_count": len(records)}


def sanitize_bot_management(result: Any) -> dict[str, Any]:
    if not isinstance(result, dict):
        return {}
    keys = {
        "fight_mode",
        "sbfm_likely_automated",
        "sbfm_definitely_automated",
        "sbfm_verified_bots",
        "sbfm_static_resource_protection",
        "ai_bots_protection",
        "content_bots_protection",
        "using_latest_model",
        "stale_zone_configuration",
    }
    return {key: result.get(key) for key in sorted(keys) if key in result}


def sanitize_zone_setting(setting_id: str, result: Any) -> dict[str, Any]:
    if not isinstance(result, dict):
        return {"id": setting_id, "value": None}
    return {
        "id": setting_id,
        "value": result.get("value"),
        "editable": result.get("editable"),
        "modified_on": result.get("modified_on"),
    }


def sanitize_action_parameters(value: Any, *, canonical_scope: bool) -> Any:
    if value is None:
        return None
    if canonical_scope:
        text = canonical_json(value)
        if len(text) <= 2000 and not any(
            token in text.lower() for token in ("token", "secret", "password", "credential")
        ):
            return value
    return {"sha256": digest(value)}


def sanitize_ruleset(ruleset: dict[str, Any]) -> dict[str, Any]:
    rules = ruleset.get("rules") if isinstance(ruleset.get("rules"), list) else []
    sanitized_rules: list[dict[str, Any]] = []
    for rule in rules:
        if not isinstance(rule, dict):
            continue
        expression = str(rule.get("expression") or "")
        matches = exact_host_matches(expression)
        canonical_scope = bool(matches)
        description = str(rule.get("description") or "")
        sanitized_rules.append(
            {
                "id_hash": hash_id(rule.get("id")),
                "action": rule.get("action"),
                "enabled": rule.get("enabled", True),
                "description": description[:300] if canonical_scope else None,
                "description_sha256": digest(description),
                "canonical_hosts": matches,
                "scope": "canonical" if canonical_scope else "broad_or_unrelated",
                "expression": expression[:2000] if canonical_scope else None,
                "expression_sha256": digest(expression),
                "action_parameters": sanitize_action_parameters(
                    rule.get("action_parameters"), canonical_scope=canonical_scope
                ),
            }
        )
    ruleset_name = str(ruleset.get("name") or "")
    ruleset_description = str(ruleset.get("description") or "")
    return {
        "id_hash": hash_id(ruleset.get("id")),
        "name": None,
        "name_sha256": digest(ruleset_name),
        "description": None,
        "description_sha256": digest(ruleset_description),
        "kind": ruleset.get("kind"),
        "phase": ruleset.get("phase"),
        "version": ruleset.get("version"),
        "rules": sanitized_rules,
    }


def sanitize_access_apps(result: Any) -> dict[str, Any]:
    apps = result if isinstance(result, list) else []
    matched = []
    for app in apps:
        if not isinstance(app, dict):
            continue
        domain = str(app.get("domain") or "").lower().rstrip("/")
        if not any(domain == host or domain.startswith(f"{host}/") for host in CANONICAL_HOSTS):
            continue
        matched.append(
            {
                "id_hash": hash_id(app.get("id")),
                "domain": domain,
                "type": app.get("type"),
                "session_duration": app.get("session_duration"),
                "app_launcher_visible": app.get("app_launcher_visible"),
            }
        )
    return {"matched": matched, "total_app_count": len(apps)}


def classify(audit: dict[str, Any]) -> dict[str, Any]:
    coverage = audit.get("certificate_packs", {}).get("active_hostname_coverage", {})
    bot = audit.get("bot_management", {})
    settings = audit.get("zone_settings", {})
    rulesets = audit.get("rulesets", [])
    access_apps = audit.get("access_apps", {}).get("matched", [])

    challenge_candidates: list[dict[str, Any]] = []
    for ruleset in rulesets:
        for rule in ruleset.get("rules", []):
            if rule.get("enabled") and rule.get("action") in CHALLENGE_ACTIONS:
                challenge_candidates.append(
                    {
                        "phase": ruleset.get("phase"),
                        "action": rule.get("action"),
                        "scope": rule.get("scope"),
                        "canonical_hosts": rule.get("canonical_hosts"),
                        "rule_id_hash": rule.get("id_hash"),
                    }
                )

    bot_challenge = bool(bot.get("fight_mode")) or any(
        bot.get(key) not in {None, "allow", "disabled", False}
        for key in ("sbfm_likely_automated", "sbfm_definitely_automated", "content_bots_protection")
    )
    always_https = settings.get("always_use_https", {}).get("value") == "on"
    hsts_value = settings.get("security_header", {}).get("value") or {}
    hsts = hsts_value.get("strict_transport_security") if isinstance(hsts_value, dict) else {}
    hsts_enabled = bool(isinstance(hsts, dict) and hsts.get("enabled") and (hsts.get("max_age") or 0) > 0)

    return {
        "certificate_hostname_coverage": {
            WWW_HOST: bool(coverage.get(WWW_HOST)),
            LOGIN_HOST: bool(coverage.get(LOGIN_HOST)),
        },
        "bot_challenge_candidate": bot_challenge,
        "ruleset_challenge_candidates": challenge_candidates,
        "access_app_matches": access_apps,
        "always_use_https": always_https,
        "positive_hsts": hsts_enabled,
    }


def main() -> None:
    token = env_required("CLOUDFLARE_API_TOKEN")
    account_id = env_required("CLOUDFLARE_ACCOUNT_ID")
    zone_id = env_required("CLOUDFLARE_ZONE_ID")
    client = CloudflareClient(token, account_id, zone_id)
    OUT.mkdir(parents=True, exist_ok=True)

    token_path = f"/accounts/{account_id}/tokens/verify" if token.startswith("cfat_") else "/user/tokens/verify"
    token_result = client.get(token_path)
    zone_result = client.get(f"/zones/{zone_id}")
    if not isinstance(zone_result.result, dict):
        raise SystemExit("Cloudflare zone lookup returned an unexpected result")
    zone_account = ((zone_result.result.get("account") or {}).get("id") or "").lower()
    if zone_account and zone_account != account_id.lower():
        raise SystemExit("CLOUDFLARE_ZONE_ID does not belong to CLOUDFLARE_ACCOUNT_ID")

    endpoint_results: dict[str, ApiResult] = {
        "certificate_packs": client.get(f"/zones/{zone_id}/ssl/certificate_packs?per_page=100", optional=True),
        "ssl_verification": client.get(f"/zones/{zone_id}/ssl/verification", optional=True),
        "total_tls": client.get(f"/zones/{zone_id}/acm/total_tls", optional=True),
        "bot_management": client.get(f"/zones/{zone_id}/bot_management", optional=True),
        "rulesets": client.get(f"/zones/{zone_id}/rulesets?per_page=50", optional=True),
        "access_apps": client.get(f"/accounts/{account_id}/access/apps?per_page=100", optional=True),
    }
    setting_ids = [
        "always_use_https",
        "security_header",
        "security_level",
        "browser_check",
        "tls_1_3",
        "min_tls_version",
    ]
    setting_results = {
        setting_id: client.get(f"/zones/{zone_id}/settings/{setting_id}", optional=True)
        for setting_id in setting_ids
    }

    sanitized_rulesets: list[dict[str, Any]] = []
    ruleset_detail_permission_denied: list[str] = []
    ruleset_listing = endpoint_results["rulesets"]
    if ruleset_listing.ok and isinstance(ruleset_listing.result, list):
        for item in ruleset_listing.result:
            if not isinstance(item, dict) or item.get("phase") not in RELEVANT_PHASES:
                continue
            ruleset_id = str(item.get("id") or "")
            if not ruleset_id:
                continue
            detail = client.get(f"/zones/{zone_id}/rulesets/{ruleset_id}", optional=True)
            if detail.ok and isinstance(detail.result, dict):
                sanitized_rulesets.append(sanitize_ruleset(detail.result))
            else:
                if detail.permission_denied:
                    ruleset_detail_permission_denied.append(hash_id(ruleset_id) or "unknown")
                sanitized_rulesets.append(
                    {
                        "id_hash": hash_id(ruleset_id),
                        "name": item.get("name"),
                        "kind": item.get("kind"),
                        "phase": item.get("phase"),
                        "detail_status": detail.sanitized_status(),
                        "rules": [],
                    }
                )

    audit: dict[str, Any] = {
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "read_only": True,
        "canonical_hosts": list(CANONICAL_HOSTS),
        "identity": {
            "token_status": (token_result.result or {}).get("status") if isinstance(token_result.result, dict) else None,
            "account_id_hash": hash_id(account_id),
            "zone_id_hash": hash_id(zone_id),
            "zone_name": zone_result.result.get("name"),
            "zone_status": zone_result.result.get("status"),
            "zone_plan": ((zone_result.result.get("plan") or {}).get("name")),
        },
        "endpoint_status": {
            name: result.sanitized_status() for name, result in endpoint_results.items()
        },
        "certificate_packs": sanitize_certificate_packs(endpoint_results["certificate_packs"].result)
        if endpoint_results["certificate_packs"].ok
        else {},
        "ssl_verification": sanitize_ssl_verification(endpoint_results["ssl_verification"].result)
        if endpoint_results["ssl_verification"].ok
        else {},
        "total_tls": endpoint_results["total_tls"].result if endpoint_results["total_tls"].ok else None,
        "bot_management": sanitize_bot_management(endpoint_results["bot_management"].result)
        if endpoint_results["bot_management"].ok
        else {},
        "zone_settings": {
            setting_id: sanitize_zone_setting(setting_id, result.result)
            if result.ok
            else {"id": setting_id, "status": result.sanitized_status()}
            for setting_id, result in setting_results.items()
        },
        "rulesets": sanitized_rulesets,
        "access_apps": sanitize_access_apps(endpoint_results["access_apps"].result)
        if endpoint_results["access_apps"].ok
        else {},
        "requested_get_paths_sha256": digest(client.requested_paths),
        "requested_get_count": len(client.requested_paths),
    }
    audit["classification"] = classify(audit)
    audit["missing_permissions"] = sorted(
        [name for name, result in endpoint_results.items() if result.permission_denied]
        + [f"setting:{name}" for name, result in setting_results.items() if result.permission_denied]
        + [f"ruleset_detail:{item}" for item in ruleset_detail_permission_denied]
    )

    serialized = canonical_json(audit)
    if token in serialized:
        raise SystemExit("secret redaction failure")
    (OUT / "audit.json").write_text(json.dumps(audit, indent=2, sort_keys=True) + "\n")

    classification = audit["classification"]
    lines = [
        "# Cloudflare Oteryn edge policy audit",
        "",
        f"Observed at: `{audit['observed_at_utc']}`",
        "",
        "## Classification",
        "",
        f"- WWW hostname certificate coverage: `{classification['certificate_hostname_coverage'][WWW_HOST]}`",
        f"- Gateway hostname certificate coverage: `{classification['certificate_hostname_coverage'][LOGIN_HOST]}`",
        f"- Bot challenge candidate: `{classification['bot_challenge_candidate']}`",
        f"- Matching Access applications: `{len(classification['access_app_matches'])}`",
        f"- Challenge/block ruleset candidates: `{len(classification['ruleset_challenge_candidates'])}`",
        f"- Always Use HTTPS: `{classification['always_use_https']}`",
        f"- Positive HSTS: `{classification['positive_hsts']}`",
        f"- Missing permission groups: `{', '.join(audit['missing_permissions']) or 'none'}`",
        "",
        "## Endpoint status",
        "",
    ]
    for name, status in audit["endpoint_status"].items():
        lines.append(
            f"- `{name}`: ok `{status['ok']}`, HTTP `{status['http_status']}`, permission denied `{status['permission_denied']}`"
        )
    lines.extend(
        [
            "",
            "The full sanitized machine-readable audit is stored in audit.json. No write request was made.",
            "",
        ]
    )
    (OUT / "summary.md").write_text("\n".join(lines))
    print((OUT / "summary.md").read_text())


if __name__ == "__main__":
    main()
