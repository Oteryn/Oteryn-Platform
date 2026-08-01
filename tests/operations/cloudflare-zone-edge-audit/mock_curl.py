#!/usr/bin/env python3
import json
import os
import sys
from urllib.parse import urlparse


def parse_args(argv):
    method = "GET"
    output = None
    url = None
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "--request":
            method = argv[i + 1]
            i += 2
            continue
        if arg == "--output":
            output = argv[i + 1]
            i += 2
            continue
        if arg in {"--header", "--connect-timeout", "--max-time", "--write-out"}:
            i += 2
            continue
        if arg.startswith("http://") or arg.startswith("https://"):
            url = arg
        i += 1
    return method, output, url


def response_for(path):
    account = "a" * 32
    zone = "b" * 32
    if path in {f"/accounts/{account}/tokens/verify", "/user/tokens/verify"}:
        return {"success": True, "result": {"status": "active"}}
    if path.startswith(f"/zones/{zone}/ssl/certificate_packs"):
        return {
            "success": True,
            "result": [
                {
                    "status": "active",
                    "hosts": ["molehill.cloud", "*.molehill.cloud"],
                    "certificates": [
                        {"status": "active", "hosts": ["molehill.cloud", "*.molehill.cloud"]}
                    ],
                }
            ],
            "result_info": {"total_pages": 1},
        }
    if path == f"/zones/{zone}/ssl/universal/settings":
        return {"success": True, "result": {"enabled": True}}
    if path == f"/zones/{zone}/ssl/verification":
        return {"success": True, "result": [{"certificate_status": "active"}]}
    if path == f"/zones/{zone}/settings":
        values = {
            "ssl": "full",
            "min_tls_version": "1.3",
            "tls_1_3": "on",
            "always_use_https": "off",
            "security_level": "under_attack",
            "browser_check": "on",
            "challenge_ttl": 1800,
            "security_header": {
                "strict_transport_security": {
                    "enabled": False,
                    "max_age": 0,
                    "include_subdomains": True,
                    "preload": True,
                }
            },
        }
        return {"success": True, "result": [{"id": k, "value": v} for k, v in values.items()]}
    if path.startswith(f"/zones/{zone}/rulesets?"):
        return {
            "success": True,
            "result": [{"id": "zone-rule-1", "phase": "http_request_firewall_custom"}],
            "result_info": {"total_pages": 1},
        }
    if path == f"/zones/{zone}/rulesets/zone-rule-1":
        return {
            "success": True,
            "result": {
                "rules": [
                    {
                        "enabled": True,
                        "action": "managed_challenge",
                        "expression": '(http.host eq "oteryn.molehill.cloud")',
                    }
                ]
            },
        }
    if path.startswith(f"/accounts/{account}/rulesets?"):
        return {"success": True, "result": [], "result_info": {"total_pages": 1}}
    if path == f"/zones/{zone}/bot_management":
        return {
            "success": True,
            "result": {
                "fight_mode": True,
                "enable_js": True,
                "ai_bots_protection": "disabled",
                "content_bots_protection": "disabled",
            },
        }
    if path.startswith(f"/accounts/{account}/access/apps?"):
        return {"success": True, "result": [], "result_info": {"total_pages": 1}}
    if path.startswith(f"/zones/{zone}/pagerules?"):
        return {"success": True, "result": [], "result_info": {"total_pages": 1}}
    return {"success": False, "errors": [{"code": 1000, "message": "unhandled mock path"}]}


def main():
    method, output, url = parse_args(sys.argv[1:])
    if not output or not url:
        raise SystemExit("mock curl received incomplete arguments")
    path = urlparse(url).path
    if path.startswith("/client/v4"):
        path = path[len("/client/v4") :]
    if urlparse(url).query:
        path += "?" + urlparse(url).query
    log_path = os.environ.get("MOCK_CURL_LOG")
    if log_path:
        with open(log_path, "a", encoding="utf-8") as handle:
            handle.write(f"{method} {path}\n")
    response = response_for(path)
    status = "200" if response.get("success") else "404"
    with open(output, "w", encoding="utf-8") as handle:
        json.dump(response, handle)
    sys.stdout.write(status)


if __name__ == "__main__":
    main()
