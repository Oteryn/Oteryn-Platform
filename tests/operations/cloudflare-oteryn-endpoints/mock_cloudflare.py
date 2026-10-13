#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

ACCOUNT_ID = "a" * 32
ZONE_ID = "b" * 32
TUNNEL_ID = "123e4567-e89b-42d3-a456-426614174000"
TOKEN = "test-token"
WWW_HOST = "oteryn.molehill.cloud"
GATEWAY_HOST = "gateway.molehill.cloud"
LEGACY_GATEWAY_HOST = "login.oteryn.molehill.cloud"
TARGET = f"{TUNNEL_ID}.cfargotunnel.com"

import copy

STATE = {
    "mutations": [],
    "version": 1,
    "get_count": 0,
    "inject": {},
    "config_src": "cloudflare",
    "probe": {},
    "config": {
        "originRequest": {"connectTimeout": "30s"},
        "ingress": [
            {"hostname": "other.molehill.cloud", "service": "http://127.0.0.1:9000"},
            {
                "hostname": WWW_HOST,
                "service": "http://old-www:8000",
                "originRequest": {"httpHostHeader": "old-www"},
            },
            {
                "hostname": LEGACY_GATEWAY_HOST,
                "service": "http://old-gateway:8080",
                "originRequest": {"connectTimeout": "15s"},
            },
            {"service": "http_status:404"},
        ],
    },
    "dns": {
        LEGACY_GATEWAY_HOST: {
            "id": "record-legacy-gateway",
            "type": "CNAME",
            "name": LEGACY_GATEWAY_HOST,
            "content": TARGET,
            "proxied": True,
        }
    },
}


INITIAL = copy.deepcopy(STATE)


def envelope(result: object, success: bool = True) -> dict[str, object]:
    return {"success": success, "errors": [], "messages": [], "result": result}


class Handler(BaseHTTPRequestHandler):
    server_version = "MockCloudflare/2.0"

    def log_message(self, *_args: object) -> None:
        return

    def _json(self, status: int, payload: object) -> None:
        body = json.dumps(payload, separators=(",", ":")).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _authorized(self) -> bool:
        if self.path.startswith("/__") or not self.path.startswith("/client/"):
            return True
        if self.headers.get("Authorization") == f"Bearer {TOKEN}":
            return True
        self._json(
            403,
            {
                "success": False,
                "errors": [{"code": 9109, "message": "Unauthorized"}],
                "messages": [],
                "result": None,
            },
        )
        return False

    def _body(self) -> dict[str, object]:
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length) if length else b"{}"
        return json.loads(raw)

    def do_GET(self) -> None:  # noqa: N802
        if not self._authorized():
            return
        parsed = urlparse(self.path)
        path = parsed.path

        if not path.startswith("/client/") and not path.startswith("/__"):
            self._probe()
            return
        if path == "/__state":
            self._json(200, STATE)
            return
        if path == "/client/v4/user/tokens/verify":
            self._json(200, envelope({"id": "token-id", "status": "active"}))
            return
        if path == f"/client/v4/accounts/{ACCOUNT_ID}/cfd_tunnel/{TUNNEL_ID}":
            self._json(
                200,
                envelope(
                    {
                        "id": TUNNEL_ID,
                        "account_tag": ACCOUNT_ID,
                        "config_src": STATE["config_src"],
                        "status": "healthy",
                        "deleted_at": None,
                    }
                ),
            )
            return
        if path == f"/client/v4/accounts/{ACCOUNT_ID}/cfd_tunnel/{TUNNEL_ID}/configurations":
            inject = STATE["inject"]
            STATE["get_count"] += 1
            if inject.get("readback_fail") and "tunnel-put" in STATE["mutations"]:
                self._json(500, {"success": False, "errors": [{"code": 2, "message": "boom"}], "messages": [], "result": None})
                return
            reported = STATE["version"] + int(inject.get("version_report_offset", 0))
            result = {"account_id": ACCOUNT_ID, "config": copy.deepcopy(STATE["config"]), "version": reported}
            if inject.get("omit_version"):
                del result["version"]
            payload = envelope(result)
            edit_at = inject.get("edit_after_get")
            if edit_at is not None and STATE["get_count"] == int(edit_at):
                # Interleaved external write: visible from the next read on.
                STATE["config"] = inject["edit_config"]
                if inject.get("edit_bumps_version", True):
                    STATE["version"] += 1
                STATE["mutations"].append("external-edit")
            self._json(200, payload)
            return
        if path == f"/client/v4/zones/{ZONE_ID}/dns_records":
            host = parse_qs(parsed.query).get("name.exact", [""])[0]
            record = STATE["dns"].get(host)
            self._json(200, envelope([] if record is None else [record]))
            return

        self._json(
            404,
            {
                "success": False,
                "errors": [{"code": 1000, "message": f"Unknown GET {path}"}],
                "messages": [],
                "result": None,
            },
        )

    def do_PUT(self) -> None:  # noqa: N802
        if not self._authorized():
            return
        path = urlparse(self.path).path
        if path == f"/client/v4/accounts/{ACCOUNT_ID}/cfd_tunnel/{TUNNEL_ID}/configurations":
            payload = self._body()
            mode = STATE["inject"].get("put_mode", "")
            if mode == "fail_500":
                self._json(500, {"success": False, "errors": [{"code": 1, "message": "boom"}], "messages": [], "result": None})
                return
            STATE["config"] = payload["config"]
            STATE["version"] += int(STATE["inject"].get("put_version_bump", 1))
            STATE["mutations"].append("tunnel-put")
            if mode == "applied_then_500":
                self._json(500, {"success": False, "errors": [{"code": 1, "message": "boom"}], "messages": [], "result": None})
                return
            self._json(
                200,
                envelope({"account_id": ACCOUNT_ID, "config": STATE["config"], "version": STATE["version"]}),
            )
            return
        self._json(404, envelope(None, success=False))

    def _probe(self) -> None:
        host = (self.headers.get("Host") or "").split(":")[0].lower()
        table = STATE["probe"]
        entry = table.get(host) or table.get("*") or {"status": 404, "headers": {}, "cookies": []}
        self.send_response_only(int(entry["status"]))
        for name, value in entry.get("headers", {}).items():
            self.send_header(name, value)
        for cookie in entry.get("cookies", []):
            self.send_header("Set-Cookie", f"{cookie}=1; Path=/")
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_POST(self) -> None:  # noqa: N802
        if not self._authorized():
            return
        path = urlparse(self.path).path
        if path == "/__reset":
            payload = self._body()
            STATE.clear()
            STATE.update(copy.deepcopy(INITIAL))
            for key in ("config", "version", "dns", "config_src", "probe", "inject"):
                if key in payload:
                    STATE[key] = payload[key]
            self._json(200, envelope(True))
            return
        if path == "/__inject":
            STATE["inject"] = self._body()
            STATE["get_count"] = 0
            self._json(200, envelope(True))
            return
        if path == "/__external_edit":
            payload = self._body()
            STATE["config"] = payload["config"]
            if payload.get("bump_version", True):
                STATE["version"] += 1
            STATE["mutations"].append("external-edit")
            self._json(200, envelope(True))
            return
        if path == f"/client/v4/zones/{ZONE_ID}/dns_records":
            payload = self._body()
            host = str(payload["name"])
            record = {"id": f"record-{len(STATE['dns']) + 1}", **payload}
            STATE["dns"][host] = record
            STATE["mutations"].append(f"dns-post:{host}")
            self._json(200, envelope(record))
            return
        self._json(404, envelope(None, success=False))

    def do_PATCH(self) -> None:  # noqa: N802
        if not self._authorized():
            return
        path = urlparse(self.path).path
        prefix = f"/client/v4/zones/{ZONE_ID}/dns_records/"
        if path.startswith(prefix):
            record_id = path.removeprefix(prefix)
            payload = self._body()
            for host, record in list(STATE["dns"].items()):
                if record["id"] == record_id:
                    updated = {**record, **payload}
                    new_host = str(updated["name"])
                    if new_host != host:
                        del STATE["dns"][host]
                    STATE["dns"][new_host] = updated
                    STATE["mutations"].append(f"dns-patch:{new_host}")
                    self._json(200, envelope(updated))
                    return
            self._json(404, envelope(None, success=False))
            return
        self._json(404, envelope(None, success=False))

    def do_DELETE(self) -> None:  # noqa: N802
        if not self._authorized():
            return
        path = urlparse(self.path).path
        prefix = f"/client/v4/zones/{ZONE_ID}/dns_records/"
        if path.startswith(prefix):
            record_id = path.removeprefix(prefix)
            for host, record in list(STATE["dns"].items()):
                if record["id"] == record_id:
                    del STATE["dns"][host]
                    STATE["mutations"].append(f"dns-delete:{host}")
                    self._json(200, envelope({"id": record_id}))
                    return
            self._json(404, envelope(None, success=False))
            return
        self._json(404, envelope(None, success=False))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port-file", required=True)
    args = parser.parse_args()

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    Path(args.port_file).write_text(str(server.server_address[1]), encoding="utf-8")
    server.serve_forever()


if __name__ == "__main__":
    main()
