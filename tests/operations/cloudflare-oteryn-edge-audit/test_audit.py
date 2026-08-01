from __future__ import annotations

import ast
import json
import os
import runpy
import subprocess
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlparse

SCRIPT = Path(__file__).resolve().parents[3] / "scripts/operations/cloudflare-oteryn-edge-audit.py"
MODULE = runpy.run_path(str(SCRIPT), run_name="cloudflare_edge_audit_test")


class AuditUnitTests(unittest.TestCase):
    def test_wildcard_matches_only_one_label(self) -> None:
        covers = MODULE["certificate_name_covers"]
        self.assertTrue(covers("oteryn.molehill.cloud", "*.molehill.cloud"))
        self.assertFalse(covers("login.oteryn.molehill.cloud", "*.molehill.cloud"))
        self.assertTrue(covers("login.oteryn.molehill.cloud", "login.oteryn.molehill.cloud"))

    def test_certificate_pack_coverage(self) -> None:
        sanitize = MODULE["sanitize_certificate_packs"]
        result = sanitize(
            [
                {"id": "pack-1", "status": "active", "hosts": ["*.molehill.cloud"]},
                {"id": "pack-2", "status": "pending_validation", "hosts": ["login.oteryn.molehill.cloud"]},
            ]
        )
        coverage = result["active_hostname_coverage"]
        self.assertTrue(coverage["oteryn.molehill.cloud"])
        self.assertFalse(coverage["login.oteryn.molehill.cloud"])

    def test_unrelated_rules_are_redacted(self) -> None:
        sanitize = MODULE["sanitize_ruleset"]
        output = sanitize(
            {
                "id": "ruleset-secret-id",
                "name": "Internal customer rules",
                "description": "mentions private.example.net",
                "kind": "zone",
                "phase": "http_request_firewall_custom",
                "rules": [
                    {
                        "id": "rule-private",
                        "action": "block",
                        "description": "private tenant policy",
                        "expression": 'http.host eq "private.example.net"',
                        "action_parameters": {"private": "value"},
                    },
                    {
                        "id": "rule-oteryn",
                        "action": "managed_challenge",
                        "description": "Oteryn challenge",
                        "expression": 'http.host eq "oteryn.molehill.cloud"',
                    },
                ],
            }
        )
        private, canonical = output["rules"]
        self.assertIsNone(private["expression"])
        self.assertIsNone(private["description"])
        self.assertEqual(private["action_parameters"].keys(), {"sha256"})
        self.assertIn("oteryn.molehill.cloud", canonical["expression"])
        self.assertNotIn("private.example.net", json.dumps(output))

    def test_access_apps_are_exact_host_scoped(self) -> None:
        sanitize = MODULE["sanitize_access_apps"]
        output = sanitize(
            [
                {"id": "1", "domain": "oteryn.molehill.cloud/admin", "type": "self_hosted"},
                {"id": "2", "domain": "oteryn.molehill.cloud.evil.example", "type": "self_hosted"},
                {"id": "3", "domain": "private.example.net", "type": "self_hosted"},
            ]
        )
        self.assertEqual([item["domain"] for item in output["matched"]], ["oteryn.molehill.cloud/admin"])
        self.assertEqual(output["total_app_count"], 3)

    def test_source_contains_get_only_api_requests(self) -> None:
        tree = ast.parse(SCRIPT.read_text())
        methods: list[str] = []
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue
            if isinstance(node.func, ast.Attribute) and node.func.attr == "Request":
                for keyword in node.keywords:
                    if keyword.arg == "method" and isinstance(keyword.value, ast.Constant):
                        methods.append(keyword.value.value)
        self.assertEqual(methods, ["GET"])


class FakeCloudflareHandler(BaseHTTPRequestHandler):
    requests_seen: list[tuple[str, str]] = []

    def log_message(self, _format: str, *_args: object) -> None:
        return

    def _reply(self, result: object, status: int = 200) -> None:
        payload = json.dumps({"success": status < 300, "result": result, "errors": []}).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self) -> None:
        type(self).requests_seen.append(("GET", self.path))
        path = urlparse(self.path).path
        if path.endswith("/tokens/verify"):
            return self._reply({"status": "active"})
        if path == "/client/v4/zones/22222222222222222222222222222222":
            return self._reply(
                {
                    "id": "22222222222222222222222222222222",
                    "name": "molehill.cloud",
                    "status": "active",
                    "account": {"id": "11111111111111111111111111111111"},
                    "plan": {"name": "test"},
                }
            )
        if path.endswith("/ssl/certificate_packs"):
            return self._reply(
                [
                    {
                        "id": "wildcard-pack",
                        "type": "universal",
                        "status": "active",
                        "hosts": ["molehill.cloud", "*.molehill.cloud"],
                    },
                    {
                        "id": "login-pack",
                        "type": "advanced",
                        "status": "pending_validation",
                        "hosts": ["login.oteryn.molehill.cloud", "unrelated.example.net"],
                    },
                ]
            )
        if path.endswith("/ssl/verification"):
            return self._reply(
                [
                    {"hostname": "login.oteryn.molehill.cloud", "status": "pending_validation"},
                    {"hostname": "private.example.net", "status": "active"},
                ]
            )
        if path.endswith("/acm/total_tls"):
            return self._reply({"enabled": True, "certificate_authority": "google"})
        if path.endswith("/bot_management"):
            return self._reply({"fight_mode": True, "sbfm_definitely_automated": "managed_challenge"})
        if path.endswith("/rulesets"):
            return self._reply(
                [
                    {
                        "id": "rs-1",
                        "name": "zone custom firewall",
                        "kind": "zone",
                        "phase": "http_request_firewall_custom",
                    }
                ]
            )
        if path.endswith("/rulesets/rs-1"):
            return self._reply(
                {
                    "id": "rs-1",
                    "name": "private and Oteryn rules",
                    "description": "private.example.net plus Oteryn",
                    "kind": "zone",
                    "phase": "http_request_firewall_custom",
                    "version": "1",
                    "rules": [
                        {
                            "id": "r-private",
                            "action": "block",
                            "description": "private.example.net",
                            "expression": 'http.host eq "private.example.net"',
                        },
                        {
                            "id": "r-oteryn",
                            "action": "managed_challenge",
                            "description": "challenge Oteryn",
                            "expression": 'http.host eq "oteryn.molehill.cloud"',
                        },
                    ],
                }
            )
        if path.endswith("/access/apps"):
            return self._reply(
                [
                    {"id": "app-1", "domain": "oteryn.molehill.cloud/admin", "type": "self_hosted"},
                    {"id": "app-private", "domain": "private.example.net", "type": "self_hosted"},
                ]
            )
        if "/settings/" in path:
            setting = path.rsplit("/", 1)[-1]
            values = {
                "always_use_https": "off",
                "security_header": {
                    "strict_transport_security": {
                        "enabled": False,
                        "max_age": 0,
                        "include_subdomains": True,
                        "preload": True,
                    }
                },
                "security_level": "medium",
                "browser_check": "on",
                "tls_1_3": "on",
                "min_tls_version": "1.2",
            }
            return self._reply({"id": setting, "value": values[setting], "editable": True})
        return self._reply(None, status=404)

    def do_POST(self) -> None:
        type(self).requests_seen.append(("POST", self.path))
        self._reply(None, status=405)

    do_PUT = do_POST
    do_PATCH = do_POST
    do_DELETE = do_POST


class AuditIntegrationTests(unittest.TestCase):
    def test_main_is_read_only_and_sanitized(self) -> None:
        FakeCloudflareHandler.requests_seen = []
        server = ThreadingHTTPServer(("127.0.0.1", 0), FakeCloudflareHandler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            with tempfile.TemporaryDirectory() as tmp:
                secret = "cfat_super_secret_test_token"
                env = os.environ.copy()
                env.update(
                    {
                        "CLOUDFLARE_API_BASE_URL": f"http://127.0.0.1:{server.server_port}/client/v4",
                        "CLOUDFLARE_API_TOKEN": secret,
                        "CLOUDFLARE_ACCOUNT_ID": "11111111111111111111111111111111",
                        "CLOUDFLARE_ZONE_ID": "22222222222222222222222222222222",
                        "CLOUDFLARE_EDGE_AUDIT_OUT": str(Path(tmp) / "out"),
                    }
                )
                completed = subprocess.run(
                    [sys.executable, str(SCRIPT)],
                    env=env,
                    text=True,
                    capture_output=True,
                    timeout=30,
                    check=False,
                )
                self.assertEqual(completed.returncode, 0, completed.stderr)
                audit_path = Path(tmp) / "out" / "audit.json"
                summary_path = Path(tmp) / "out" / "summary.md"
                self.assertTrue(audit_path.is_file())
                self.assertTrue(summary_path.is_file())
                raw = audit_path.read_text() + summary_path.read_text()
                self.assertNotIn(secret, raw)
                self.assertNotIn("private.example.net", raw)
                audit = json.loads(audit_path.read_text())
                coverage = audit["classification"]["certificate_hostname_coverage"]
                self.assertTrue(coverage["oteryn.molehill.cloud"])
                self.assertFalse(coverage["login.oteryn.molehill.cloud"])
                self.assertTrue(audit["classification"]["bot_challenge_candidate"])
                self.assertFalse(audit["classification"]["always_use_https"])
                self.assertFalse(audit["classification"]["positive_hsts"])
                self.assertTrue(all(method == "GET" for method, _ in FakeCloudflareHandler.requests_seen))
                self.assertGreater(len(FakeCloudflareHandler.requests_seen), 5)
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=5)


if __name__ == "__main__":
    unittest.main()
