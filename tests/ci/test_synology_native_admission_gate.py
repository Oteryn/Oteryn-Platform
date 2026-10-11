from __future__ import annotations

import http.server
import json
import pathlib
import re
import socketserver
import subprocess
import threading


ROOT = pathlib.Path(__file__).resolve().parents[2]
COMPOSE = ROOT / "deploy" / "synology" / "compose.yml"
LIB = ROOT / "deploy" / "synology" / "scripts" / "lib.sh"
HEALTH_CHECK = ROOT / "deploy" / "synology" / "scripts" / "health-check.sh"
README = ROOT / "deploy" / "synology" / "README.md"

PROBE_MARKER = '"$platform_native_admission" <<\'PY\'\n'
UUID7 = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
NO_STORE = {
    "Cache-Control": "no-store, no-cache, must-revalidate, private",
    "Pragma": "no-cache",
    "Expires": "0",
}


def test_gateway_native_login_is_the_platform_admission_flag() -> None:
    text = COMPOSE.read_text(encoding="utf-8")
    assert 'x-native-admission-enabled: &native_admission_enabled "false"' in text
    assert "GAME_AUTH_NATIVE_ADMISSION_ENABLED: *native_admission_enabled" in text
    assert "GATEWAY_NATIVE_LOGIN_ENABLED: *native_admission_enabled" in text
    # Synology runs APP_ENV=staging, which the Platform issuer refuses to sign in, so no
    # literal flag value may bypass the shared anchor.
    assert re.search(r"(GAME_AUTH_NATIVE_ADMISSION_ENABLED|GATEWAY_NATIVE_LOGIN_ENABLED):\s*[\"']?(true|false)", text) is None
    platform = text.index("\n  platform:\n")
    gateway = text.index("\n  gateway:\n")
    assert platform < text.index("GAME_AUTH_NATIVE_ADMISSION_ENABLED: *") < text.index("\n  tls-init:\n")
    assert gateway < text.index("GATEWAY_NATIVE_LOGIN_ENABLED: *")


def run_pairing(platform: str, gateway: str, app_env: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["bash", "-c", 'source "$1"; oteryn_assert_native_login_pairing "$2" "$3" "$4"', "bash", str(LIB), platform, gateway, app_env],
        text=True,
        capture_output=True,
        check=False,
    )


def test_pairing_accepts_both_off_on_staging() -> None:
    assert run_pairing("false", "false", "staging").returncode == 0


def test_pairing_accepts_both_on_in_preproduction() -> None:
    assert run_pairing("true", "true", "preproduction").returncode == 0


def test_pairing_refuses_gateway_on_without_platform() -> None:
    result = run_pairing("false", "true", "staging")
    assert result.returncode != 0
    assert "does not match Platform native admission" in result.stderr


def test_pairing_refuses_missing_platform_flag() -> None:
    result = run_pairing("", "true", "staging")
    assert result.returncode != 0
    assert "missing or invalid" in result.stderr


def test_pairing_refuses_native_login_on_staging() -> None:
    result = run_pairing("true", "true", "staging")
    assert result.returncode != 0
    assert "refuses APP_ENV=staging" in result.stderr


def test_health_check_asserts_pairing_before_probes() -> None:
    script = HEALTH_CHECK.read_text(encoding="utf-8")
    pairing = script.index('oteryn_assert_native_login_pairing "$platform_native_admission"')
    probes = script.index(PROBE_MARKER)
    assert pairing < probes
    assert '"$platform_native_admission" <<\'PY\'' in script
    assert "body=b'{}'" in script


def probe_source() -> str:
    script = HEALTH_CHECK.read_text(encoding="utf-8")
    start = script.index(PROBE_MARKER) + len(PROBE_MARKER)
    end = script.index("\nPY\n", start)
    return script[start:end]


class _Server(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True


def serve(handler: type[http.server.BaseHTTPRequestHandler]) -> tuple[_Server, int]:
    server = _Server(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, server.server_address[1]


def gateway_handler(native_answer, seen: list[dict]):
    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *args) -> None:
            return

        def reply(self, status: int, payload: object, extra: dict[str, str] | None = None) -> None:
            data = json.dumps(payload).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            for key, value in (extra or {}).items():
                self.send_header(key, value)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self) -> None:
            if self.path == "/version":
                self.reply(200, {"service": "oteryn-game-gateway", "version": "synology-staging"})
            else:
                self.reply(404, {"error": "not_found"})

        def do_POST(self) -> None:
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            if body.get("protocol_version") == 2:
                seen.append(body)
                status, payload = native_answer(body)
                self.reply(status, payload, NO_STORE)
            else:
                self.reply(400, {"error": "invalid_request"}, NO_STORE)

    return Handler


class PlatformHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args) -> None:
        return

    def do_GET(self) -> None:
        data = b'{"service":"oteryn-platform"}'
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


def run_probe(enabled: str, native_answer) -> tuple[subprocess.CompletedProcess[str], list[dict]]:
    seen: list[dict] = []
    gateway, gateway_port = serve(gateway_handler(native_answer, seen))
    platform, platform_port = serve(PlatformHandler)
    try:
        result = subprocess.run(
            ["python3", "-I", "-", str(platform_port), str(gateway_port), "synology-staging", enabled],
            input=probe_source(),
            text=True,
            capture_output=True,
            check=False,
            timeout=60,
        )
    finally:
        gateway.shutdown()
        platform.shutdown()
    return result, seen


def refusal(code: str):
    return lambda body: (
        {"NATIVE_LOGIN_UNAVAILABLE": 503, "NATIVE_LOGIN_AUTHENTICATION_REQUIRED": 401}[code],
        {"protocol_version": 2, "error": {"code": code, "public_class": "X"}, "attempt_ref": body["attempt_ref"]},
    )


def test_probe_sends_well_formed_native_login_and_accepts_closed_gateway() -> None:
    result, seen = run_probe("false", lambda body: (400, {"error": "invalid_request"}))
    assert result.returncode == 0, result.stderr
    assert len(seen) == 1
    request = seen[0]
    assert set(request) == {"protocol_version", "game_login_ticket", "attempt_ref", "character_id", "channel_id", "offer"}
    assert UUID7.match(request["attempt_ref"]) and UUID7.match(request["character_id"])
    assert request["channel_id"] is None
    assert request["offer"]["client_platform"] == "windows"
    assert request["offer"]["transports"] == [{"protocol_major": 1, "transport_profile": 1, "alpn": "oteryn-game/1"}]


def test_probe_fails_when_closed_gateway_answers_native_login() -> None:
    result, _ = run_probe("false", refusal("NATIVE_LOGIN_UNAVAILABLE"))
    assert result.returncode != 0
    assert "while native admission is off" in result.stderr


def test_probe_fails_loudly_when_native_admission_is_unavailable() -> None:
    result, _ = run_probe("true", refusal("NATIVE_LOGIN_UNAVAILABLE"))
    assert result.returncode != 0
    assert "Native admission is unavailable" in result.stderr


def test_probe_accepts_issuer_ticket_rejection_when_enabled() -> None:
    result, _ = run_probe("true", refusal("NATIVE_LOGIN_AUTHENTICATION_REQUIRED"))
    assert result.returncode == 0, result.stderr


def test_readme_documents_native_admission_keys_by_name() -> None:
    readme = README.read_text(encoding="utf-8")
    for key in (
        "GAME_AUTH_NATIVE_ADMISSION_ENABLED",
        "GATEWAY_NATIVE_LOGIN_ENABLED",
        "GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_FILE",
        "GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_ID",
    ):
        assert key in readme
