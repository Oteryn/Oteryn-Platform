#!/usr/bin/env python3
from __future__ import annotations

import json
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

ZONE = "b" * 32
SCENARIO = os.getenv("MOCK_SCENARIO", "normal")
PORT_FILE = Path(os.environ["MOCK_PORT_FILE"])
LOG_FILE = Path(os.environ["MOCK_LOG_FILE"])

BASELINE = {
    "enabled": True,
    "include_subdomains": True,
    "max_age": 0,
    "nosniff": True,
    "preload": True,
}
TARGET = {
    "enabled": True,
    "include_subdomains": False,
    "max_age": 2_592_000,
    "nosniff": True,
    "preload": False,
}
STATE: dict[str, Any] = {"hsts": dict(TARGET if SCENARIO == "staged" else BASELINE)}


def payload(result: Any, success: bool = True, errors: list[dict[str, Any]] | None = None) -> bytes:
    return json.dumps({"success": success, "result": result, "errors": errors or [], "messages": []}).encode()


class Handler(BaseHTTPRequestHandler):
    server_version = "MockCloudflareHsts/1"

    def log_message(self, *_: Any) -> None:
        return

    def record(self, body: dict[str, Any] | None = None) -> None:
        with LOG_FILE.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps({"method": self.command, "path": self.path, "body": body}) + "\n")

    def read_json(self) -> dict[str, Any]:
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length) if length else b"{}"
        return json.loads(raw)

    def send_json(self, status: int, data: bytes) -> None:
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def setting(self) -> dict[str, Any]:
        return {
            "id": "security_header",
            "value": {"strict_transport_security": STATE["hsts"]},
            "editable": True,
        }

    def do_GET(self) -> None:
        self.record()
        if self.path in {"/client/v4/user/tokens/verify", "/client/v4/tokens/verify"}:
            self.send_json(200, payload({"status": "active"}))
            return
        if self.path == f"/client/v4/zones/{ZONE}/settings/security_header":
            self.send_json(200, payload(self.setting()))
            return
        self.send_json(404, payload(None, False, [{"code": 1000, "message": "not found"}]))

    def do_PATCH(self) -> None:
        body = self.read_json()
        self.record(body)
        if self.path != f"/client/v4/zones/{ZONE}/settings/security_header":
            self.send_json(404, payload(None, False, [{"code": 1000, "message": "not found"}]))
            return
        if SCENARIO == "deny":
            self.send_json(403, payload(None, False, [{"code": 10000, "message": "permission denied"}]))
            return
        value = body.get("value", {}).get("strict_transport_security")
        if not isinstance(value, dict):
            self.send_json(400, payload(None, False, [{"code": 1001, "message": "invalid HSTS"}]))
            return
        STATE["hsts"] = dict(value)
        if SCENARIO == "malformed_after_patch" and value == TARGET:
            self.send_json(200, payload({"id": "security_header", "value": {}}))
            return
        self.send_json(200, payload(self.setting()))


server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
PORT_FILE.write_text(str(server.server_address[1]), encoding="utf-8")
server.serve_forever()
