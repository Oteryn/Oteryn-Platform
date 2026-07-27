#!/usr/bin/env python3
"""Minimal CI-only Cloudflare Access assertion emulator.

The verifier intentionally proves only edge admission behavior. Oteryn Platform does
not consume the asserted identity and remains authoritative for auth, MFA and RBAC.
"""

from __future__ import annotations

import base64
import hashlib
import hmac
import json
import os
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any


HOST = "127.0.0.1"
PORT = 9080
EXPECTED_AUDIENCE = "oteryn-admin"


def decode_segment(value: str) -> bytes:
    padding = "=" * (-len(value) % 4)
    return base64.urlsafe_b64decode(value + padding)


def valid_assertion(token: str, secret: bytes) -> bool:
    try:
        encoded_header, encoded_payload, encoded_signature = token.split(".")
        header = json.loads(decode_segment(encoded_header))
        payload: dict[str, Any] = json.loads(decode_segment(encoded_payload))
        signature = decode_segment(encoded_signature)
    except (ValueError, TypeError, json.JSONDecodeError, base64.binascii.Error):
        return False

    if header.get("alg") != "HS256" or header.get("typ") != "JWT":
        return False

    signed = f"{encoded_header}.{encoded_payload}".encode("ascii")
    expected = hmac.new(secret, signed, hashlib.sha256).digest()
    if not hmac.compare_digest(signature, expected):
        return False

    now = int(time.time())
    if not isinstance(payload.get("exp"), int) or payload["exp"] < now:
        return False
    if payload.get("aud") != EXPECTED_AUDIENCE:
        return False

    email = payload.get("email")
    return isinstance(email, str) and email.endswith("@example.test")


class Handler(BaseHTTPRequestHandler):
    server_version = "OterynAccessEmulator/1"

    def do_GET(self) -> None:  # noqa: N802 - stdlib callback name
        secret = os.environ.get("ACCESS_JWT_SECRET", "").encode("utf-8")
        token = self.headers.get("X-Access-JWT", "")
        allowed = bool(secret) and valid_assertion(token, secret)

        self.send_response(204 if allowed else 401)
        self.send_header("Cache-Control", "no-store")
        self.end_headers()

    def log_message(self, format: str, *args: object) -> None:
        # Never emit the assertion. Keep CI output bounded to status only.
        print(f"access-verifier status={args[1] if len(args) > 1 else 'UNKNOWN'}", flush=True)


if __name__ == "__main__":
    if not os.environ.get("ACCESS_JWT_SECRET"):
        raise SystemExit("ACCESS_JWT_SECRET is required")
    ThreadingHTTPServer((HOST, PORT), Handler).serve_forever()
