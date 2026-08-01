#!/usr/bin/env python3
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os

ACCOUNT = "a" * 32
ZONE = "b" * 32
TOKEN_ID = "c" * 32
RULESET_ID = "67ca2e19272a4c7d97c2a53681d0eb2f"
RULE_ID = "e0f91939eb494d4490d975498a9a9724"


class Handler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass

    def send(self, result, status=200, errors=None):
        body = json.dumps(
            {"success": status < 300, "result": result, "errors": errors or []}
        ).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        path = self.path
        if path == f"/accounts/{ACCOUNT}/tokens/verify":
            return self.send({"id": TOKEN_ID, "status": "active"})
        if path == "/user/tokens/verify":
            return self.send({"id": TOKEN_ID, "status": "active"})
        if path == f"/accounts/{ACCOUNT}/tokens/{TOKEN_ID}":
            return self.send(
                {
                    "id": TOKEN_ID,
                    "name": "oteryn automation",
                    "status": "active",
                    "policies": [
                        {
                            "effect": "allow",
                            "permission_groups": [
                                {"id": "p1", "name": "Account API Tokens Read"},
                                {"id": "p2", "name": "Cloudflare Tunnel Write"},
                            ],
                            "resources": {"com.cloudflare.api.account.*": "*"},
                        }
                    ],
                }
            )
        if path == f"/accounts/{ACCOUNT}/tokens/permission_groups":
            return self.send(
                [
                    {
                        "id": "p1",
                        "name": "Account API Tokens Read",
                        "scopes": ["com.cloudflare.api.account"],
                    },
                    {
                        "id": "p3",
                        "name": "Account API Tokens Write",
                        "scopes": ["com.cloudflare.api.account"],
                    },
                ]
            )
        if path == f"/zones/{ZONE}/ssl/certificate_packs/quota":
            return self.send(
                {
                    "advanced": {"allocated": 0, "used": 0},
                    "custom": {"allocated": 0, "used": 0},
                }
            )
        if path.startswith(f"/zones/{ZONE}/ssl/certificate_packs"):
            return self.send(
                [
                    {
                        "id": "cert1",
                        "type": "universal",
                        "status": "active",
                        "hosts": ["molehill.cloud", "*.molehill.cloud"],
                    }
                ]
            )
        if f"/zones/{ZONE}/settings/" in path:
            name = path.rsplit("/", 1)[-1]
            values = {
                "always_use_https": "off",
                "min_tls_version": "1.3",
                "security_level": "under_attack",
                "browser_check": "on",
                "security_header": {
                    "strict_transport_security": {"enabled": False, "max_age": 0}
                },
            }
            return self.send({"id": name, "value": values[name], "editable": True})
        if path == f"/zones/{ZONE}/rulesets":
            return self.send(
                [
                    {
                        "id": "r1",
                        "phase": "http_request_dynamic_redirect",
                        "kind": "zone",
                        "name": "redirects",
                    },
                    {
                        "id": RULESET_ID,
                        "phase": "http_request_firewall_custom",
                        "kind": "zone",
                        "name": "waf",
                    },
                ]
            )
        if path == f"/zones/{ZONE}/rulesets/r1":
            return self.send(
                {
                    "rules": [
                        {
                            "id": "x1",
                            "ref": "oteryn-http-redirect",
                            "action": "redirect",
                            "enabled": True,
                            "expression": 'http.host eq "oteryn.molehill.cloud"',
                            "action_parameters": {"from_value": {}},
                        }
                    ]
                }
            )
        if path == f"/zones/{ZONE}/rulesets/{RULESET_ID}":
            return self.send(
                {
                    "rules": [
                        {
                            "id": RULE_ID,
                            "ref": "broad-bot-challenge",
                            "description": "test broad rule",
                            "action": "managed_challenge",
                            "enabled": True,
                            "expression": 'ip.src.country ne "PL" and cf.client.bot',
                        },
                        {
                            "id": "x3",
                            "ref": "other-host-block",
                            "action": "block",
                            "enabled": True,
                            "expression": 'http.host eq "unrelated.example"',
                        },
                        {
                            "id": "x4",
                            "ref": "retired-login-block",
                            "action": "block",
                            "enabled": False,
                            "expression": 'http.host eq "login.oteryn.molehill.cloud"',
                        },
                    ]
                }
            )
        if path == f"/zones/{ZONE}/bot_management":
            return self.send({"fight_mode": True, "enable_js": True})
        if path.startswith(f"/accounts/{ACCOUNT}/access/apps"):
            return self.send(
                [
                    {
                        "id": "app1",
                        "domain": "oteryn.molehill.cloud",
                        "type": "self_hosted",
                    }
                ]
            )
        return self.send(
            None, status=404, errors=[{"code": 1000, "message": "not found"}]
        )


HTTPServer(("127.0.0.1", int(os.environ.get("PORT", "18080"))), Handler).serve_forever()
