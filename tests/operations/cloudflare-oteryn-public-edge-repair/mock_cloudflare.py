#!/usr/bin/env python3
from __future__ import annotations

import json
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

ZONE = "b" * 32
RULESET_ID = "67ca2e19272a4c7d97c2a53681d0eb2f"
CANDIDATE_ID = "e0f91939eb494d4490d975498a9a9724"
REPAIR_ID = "repair00000000000000000000000001"
SCENARIO = os.getenv("MOCK_SCENARIO", "normal")
PORT_FILE = Path(os.environ["MOCK_PORT_FILE"])
LOG_FILE = Path(os.environ["MOCK_LOG_FILE"])


def repair_rule(*, baseline_on: bool = True) -> dict[str, Any]:
    return {
        "id": REPAIR_ID,
        "ref": "oteryn-public-edge-canonical-skip-v1",
        "action": "skip",
        "action_parameters": {"products": ["bic", "securityLevel"], "ruleset": "current"},
        "expression": 'http.host in {"oteryn.molehill.cloud" "gateway.molehill.cloud"}',
        "description": (
            "Oteryn canonical public edge exception v1 "
            f"[bot-baseline:{'on' if baseline_on else 'off'}]"
        ),
        "enabled": True,
        "logging": {"enabled": True},
    }


def base_rules() -> list[dict[str, Any]]:
    rules = [
        {
            "id": "616428125f9b4f9bbaee3e12ad671341",
            "ref": "616428125f9b4f9bbaee3e12ad671341",
            "action": "skip",
            "action_parameters": {"phases": ["http_request_firewall_managed"], "ruleset": "current"},
            "expression": 'http.host eq "other.molehill.cloud"',
            "description": "unrelated",
            "enabled": True,
        },
        {
            "id": CANDIDATE_ID,
            "ref": CANDIDATE_ID,
            "action": "block",
            "expression": 'ip.geoip.country ne "PL"',
            "description": "country restriction",
            "enabled": True,
        },
        {
            "id": "otherblock000000000000000000000001",
            "ref": "otherblock000000000000000000000001",
            "action": "block",
            "expression": 'http.host eq "admin.molehill.cloud"',
            "description": "other host",
            "enabled": True,
        },
    ]
    if SCENARIO == "ambiguous":
        rules.append(
            {
                "id": "ambiguous00000000000000000000001",
                "ref": "ambiguous00000000000000000000001",
                "action": "block",
                "expression": 'ip.src.country not in {"PL"}',
                "description": "second country restriction",
                "enabled": True,
            }
        )
    if SCENARIO in {"repair_present", "repair_present_bot_off", "repair_present_bot_fail"}:
        candidate_index = next(i for i, rule in enumerate(rules) if rule.get("id") == CANDIDATE_ID)
        rules.insert(candidate_index, repair_rule(baseline_on=True))
    return rules


STATE: dict[str, Any] = {
    "rules": base_rules(),
    "fight_mode": False if SCENARIO in {"baseline_off", "repair_present_bot_off"} else True,
    "enable_js": True,
}


def response(result: Any, success: bool = True, errors: list[dict[str, Any]] | None = None) -> bytes:
    return json.dumps({"success": success, "result": result, "errors": errors or [], "messages": []}).encode()


class Handler(BaseHTTPRequestHandler):
    server_version = "MockCloudflare/1"

    def log_message(self, *_: Any) -> None:
        return

    def read_json(self) -> dict[str, Any]:
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length) if length else b"{}"
        return json.loads(raw)

    def record(self, body: dict[str, Any] | None = None) -> None:
        with LOG_FILE.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps({"method": self.command, "path": self.path, "body": body}) + "\n")

    def send_json(self, status: int, payload: bytes) -> None:
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def ruleset(self) -> dict[str, Any]:
        return {
            "id": RULESET_ID,
            "name": "zone",
            "kind": "zone",
            "phase": "http_request_firewall_custom",
            "rules": STATE["rules"],
            "version": "1",
        }

    def do_GET(self) -> None:
        self.record()
        if self.path in {"/client/v4/user/tokens/verify", "/client/v4/tokens/verify"}:
            self.send_json(200, response({"status": "active"}))
            return
        if self.path == f"/client/v4/zones/{ZONE}/rulesets/phases/http_request_firewall_custom/entrypoint":
            self.send_json(200, response(self.ruleset()))
            return
        if self.path == f"/client/v4/zones/{ZONE}/bot_management":
            self.send_json(
                200,
                response(
                    {
                        "fight_mode": STATE["fight_mode"],
                        "enable_js": STATE["enable_js"],
                        "ai_bots_protection": "disabled",
                    }
                ),
            )
            return
        self.send_json(404, response(None, False, [{"code": 1000, "message": "not found"}]))

    def do_POST(self) -> None:
        body = self.read_json()
        self.record(body)
        if self.path != f"/client/v4/zones/{ZONE}/rulesets/{RULESET_ID}/rules":
            self.send_json(404, response(None, False, [{"code": 1000, "message": "not found"}]))
            return
        if any(rule.get("ref") == body.get("ref") for rule in STATE["rules"]):
            self.send_json(409, response(None, False, [{"code": 1001, "message": "duplicate ref"}]))
            return
        new_rule = {key: value for key, value in body.items() if key != "position"}
        new_rule["id"] = REPAIR_ID
        before = body.get("position", {}).get("before")
        if before == "":
            index = 0
        else:
            index = next((i for i, rule in enumerate(STATE["rules"]) if rule.get("id") == before), len(STATE["rules"]))
        STATE["rules"].insert(index, new_rule)
        if SCENARIO == "malformed_after_create":
            malformed = {key: value for key, value in self.ruleset().items() if key != "rules"}
            malformed["rules"] = []
            self.send_json(200, response(malformed))
            return
        self.send_json(200, response(self.ruleset()))

    def do_PATCH(self) -> None:
        body = self.read_json()
        self.record(body)
        expected = f"/client/v4/zones/{ZONE}/rulesets/{RULESET_ID}/rules/{REPAIR_ID}"
        if self.path != expected:
            self.send_json(404, response(None, False, [{"code": 1000, "message": "not found"}]))
            return
        current_index = next((i for i, rule in enumerate(STATE["rules"]) if rule.get("id") == REPAIR_ID), -1)
        if current_index < 0:
            self.send_json(404, response(None, False, [{"code": 1002, "message": "repair not found"}]))
            return
        rule = STATE["rules"].pop(current_index)
        position = body.get("position", {})
        if "before" in position:
            before = position["before"]
            if before == "":
                index = 0
            else:
                index = next((i for i, item in enumerate(STATE["rules"]) if item.get("id") == before), len(STATE["rules"]))
        elif "after" in position:
            after = position["after"]
            if after == "":
                index = len(STATE["rules"])
            else:
                found = next((i for i, item in enumerate(STATE["rules"]) if item.get("id") == after), len(STATE["rules"]) - 1)
                index = found + 1
        elif "index" in position:
            index = max(0, min(int(position["index"]) - 1, len(STATE["rules"])))
        else:
            STATE["rules"].insert(current_index, rule)
            self.send_json(400, response(None, False, [{"code": 1003, "message": "position missing"}]))
            return
        STATE["rules"].insert(index, rule)
        self.send_json(200, response(self.ruleset()))

    def do_PUT(self) -> None:
        body = self.read_json()
        self.record(body)
        if self.path != f"/client/v4/zones/{ZONE}/bot_management":
            self.send_json(404, response(None, False, [{"code": 1000, "message": "not found"}]))
            return
        if SCENARIO in {"bot_fail", "repair_present_bot_fail"} and body.get("fight_mode") is False:
            self.send_json(403, response(None, False, [{"code": 9109, "message": "permission denied"}]))
            return
        if "fight_mode" in body:
            STATE["fight_mode"] = body["fight_mode"]
        self.send_json(200, response({"fight_mode": STATE["fight_mode"], "enable_js": STATE["enable_js"]}))

    def do_DELETE(self) -> None:
        self.record()
        expected = f"/client/v4/zones/{ZONE}/rulesets/{RULESET_ID}/rules/{REPAIR_ID}"
        if self.path != expected:
            self.send_json(404, response(None, False, [{"code": 1000, "message": "not found"}]))
            return
        STATE["rules"] = [rule for rule in STATE["rules"] if rule.get("id") != REPAIR_ID]
        self.send_json(200, response(self.ruleset()))


server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
PORT_FILE.write_text(str(server.server_address[1]), encoding="utf-8")
server.serve_forever()
