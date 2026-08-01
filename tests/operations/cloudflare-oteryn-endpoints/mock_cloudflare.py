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
LOGIN_HOST = "login.oteryn.molehill.cloud"

STATE = {
    "mutations": [],
    "config": {
        "originRequest": {"connectTimeout": "30s"},
        "ingress": [
            {"hostname": "other.molehill.cloud", "service": "http://127.0.0.1:9000"},
            {
                "hostname": WWW_HOST,
                "service": "http://old-www:8000",
                "originRequest": {"httpHostHeader": "old-www"},
            },
            {"hostname": LOGIN_HOST, "service": "http://old-login:8080"},
            {"service": "http_status:404"},
        ],
    },
    "dns": {
        LOGIN_HOST: {
            "id": "record-login",
            "type": "CNAME",
            "name": LOGIN_HOST,
            "content": "wrong.example.invalid",
            "proxied": False,
        }
    },
}


def envelope(result: object, success: bool = True) -> dict[str, object]:
    return {"success": success, "errors": [], "messages": [], "result": result}


class Handler(BaseHTTPRequestHandler):
    server_version = "MockCloudflare/1.0"

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
        if self.path.startswith("/__state"):
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
                        "config_src": "cloudflare",
                        "status": "healthy",
                        "deleted_at": None,
                    }
                ),
            )
            return
        if path == f"/client/v4/accounts/{ACCOUNT_ID}/cfd_tunnel/{TUNNEL_ID}/configurations":
            self._json(
                200,
                envelope({"account_id": ACCOUNT_ID, "config": STATE["config"], "version": 1}),
            )
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
            STATE["config"] = payload["config"]
            STATE["mutations"].append("tunnel-put")
            self._json(
                200,
                envelope({"account_id": ACCOUNT_ID, "config": STATE["config"], "version": 2}),
            )
            return
        self._json(
            404,
            {
                "success": False,
                "errors": [{"code": 1000, "message": f"Unknown PUT {path}"}],
                "messages": [],
                "result": None,
            },
        )

    def do_POST(self) -> None:  # noqa: N802
        if not self._authorized():
            return
        path = urlparse(self.path).path
        if path == f"/client/v4/zones/{ZONE_ID}/dns_records":
            payload = self._body()
            host = str(payload["name"])
            record = {"id": f"record-{len(STATE['dns']) + 1}", **payload}
            STATE["dns"][host] = record
            STATE["mutations"].append(f"dns-post:{host}")
            self._json(200, envelope(record))
            return
        self._json(
            404,
            {
                "success": False,
                "errors": [{"code": 1000, "message": f"Unknown POST {path}"}],
                "messages": [],
                "result": None,
            },
        )

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
            self._json(
                404,
                {
                    "success": False,
                    "errors": [{"code": 81044, "message": "Record not found"}],
                    "messages": [],
                    "result": None,
                },
            )
            return
        self._json(
            404,
            {
                "success": False,
                "errors": [{"code": 1000, "message": f"Unknown PATCH {path}"}],
                "messages": [],
                "result": None,
            },
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port-file", required=True)
    args = parser.parse_args()

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    Path(args.port_file).write_text(str(server.server_address[1]), encoding="utf-8")
    server.serve_forever()


if __name__ == "__main__":
    main()
