#!/usr/bin/env python3
"""PostgreSQL cutover hooks (deploy/synology/scripts/cutover-hooks/, plan P6 steps 3 to 5).

Runs the real hooks with fake docker, gh, sleep and key-tool executables, a local HTTP server on loopback in
place of the Platform and Gateway ports, and the real attempt evidence tool (attempt_evidence.py) on a temporary
directory; only its host-location check and the evidence group id are faked. Nothing touches Docker, GitHub
or the NAS. The cases prove the P6a step-4 order (no evidence write before the anchor issue is proven usable,
no init before an anchored init-intent, an anchored attempt-intent that require-open-attempt accepts), the D3
anchor retry, --close-init, step4-observe and step4-recheck (D5), fail-closed refusals, the derived runtime
environment, the smoke checks, the workflow wiring, and that no secret reaches the output.

Run: python3 tests/ci/test_synology_cutover_hooks.py
"""

from __future__ import annotations

import base64
import hashlib
import html
import http.server
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
import urllib.parse
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HOOKS = ROOT / "deploy" / "synology" / "scripts" / "cutover-hooks"
TOOL = ROOT / "deploy" / "synology" / "scripts" / "attempt_evidence.py"
DOC = ROOT / "docs" / "operations" / "PLATFORM_POSTGRES_CUTOVER_EVIDENCE.md"
STABLE_DIR = "/work/Oteryn-Platform/Oteryn-Platform/deploy/synology/scripts/cutover-hooks"
HOOK_NAMES = ("provenance-init", "web-smoke", "native-smoke")

TASK = "OTERYN-20261013-pg-cutover"
CLOSE_RUN = "7000000001"
ISSUE = "77"
S1 = "c" * 64
KID = "kid-1"
SCOPE = {"issuer": "iss", "profile": "prof", "key_purpose": "fresh"}
BOT = {"id": 41898282, "login": "github-actions[bot]"}
PLATFORM_IMAGE = "ghcr.io/oteryn/oteryn-platform@sha256:" + "1" * 64
GATEWAY_IMAGE = "ghcr.io/oteryn/oteryn-gateway@sha256:" + "2" * 64
SECRETS = {
    "APP_KEY": "base64:SECRETAPPKEYSENTINEL0123456789abcdef=",
    "PLATFORM_POSTGRES_APP_PASSWORD": "pgappsentinel0123456789abcdefgh",
    "OTERYN_PLATFORM_SERVICE_TOKEN": "servicetokensentinel0123456789abc",
}
GH_TOKEN = "ghtokensentinel0123456789abcdef"
PUBLIC_KEY = "PUBLICKEYSENTINEL0123456789"
COOKIE = "cookiesentinel0123456789"
CSRF = "csrfsentinel0123456789"
# The owner-provisioned native smoke actor and the values its login carries; none may reach the output.
NATIVE_EMAIL = "native-actor@example.invalid"
NATIVE_PASSWORD = "nativepasswordsentinel0123456789"
NATIVE_CHARACTER = "0192abcd-0000-7def-8000-00000000beef"
CLIENT_ID = "0199abcd-1111-7222-8333-444455556666"
AUTH_TOKEN = "authtokensentinel&0123"
BEARER = "bearersentinel0123456789"
TICKET = "ticketsentinel0123456789"
GRANT = "grantsentinel0123456789"


def observation(store=S1, binding="match", floor=None, profile=None, lsn=0x1000, configured=None, is_trusted=False,
                result="observed"):
    return {
        "result": result, "fence_required": True,
        "store": {"marker": store, "row": store, "binding": binding},
        "scope": SCOPE,
        "floor": {"value": floor, "temp_entries": 0},
        "profile": profile or {"present": False},
        "key": {"requested": None, "configured_key_id": configured, "in_history": False, "trusted": is_trusted},
        "database": {
            "identity": {"system_identifier": "7001", "timeline_id": 1, "postmaster_start_time": "t0",
                         "database_oid": 16384, "relfilenodes": [1, 2, 3]},
            "progress": {"wal_flush_lsn": f"0/{lsn:X}", "xmax": "100"},
            "in_flight": {"sessions": 0, "prepared": 0},
        },
    }


COMMITTED = observation(floor=1, configured=KID, is_trusted=True, lsn=0x2000,
                        profile={"present": True, "id": 1, "profile_version": 1, "issuer_revision": 1, "revoked": False})

DOCKER_STUB = r"""#!/usr/bin/env python3
# Fake docker: logs each call (never a program text) and answers compose exec / ps, inspect and run.
import json, os, sys
stub = os.environ["STUB_DIR"]
path = os.path.join(stub, "state.json")
state = json.load(open(path))
args = sys.argv[1:]
def log(entry):
    with open(os.path.join(stub, "docker.log"), "a") as handle:
        handle.write(json.dumps(entry) + "\n")
def save():
    json.dump(state, open(path, "w"))
if args[:1] == ["compose"]:
    env_file = args[args.index("--env-file") + 1]
    image = None
    with open(env_file) as handle:
        for line in handle:
            if line.startswith("PLATFORM_IMAGE="):
                image = line.strip().split("=", 1)[1]
    rest = args[args.index("-f") + 2:]
    log({"compose": rest, "env_file": env_file, "image": image})
    if rest[:2] == ["ps", "-q"]:
        print("cid-" + rest[-1])
        sys.exit(0)
    if rest[:3] == ["exec", "-T", "platform"] and "artisan" not in rest:
        # The web smoke's in-container administrator and wallet probe (PHP on stdin; never logged).
        program = sys.stdin.read()
        assert rest[-1] == "php" and program.startswith("<?php") and "SET TRANSACTION READ ONLY" in program, rest
        mode = state["probe"]
        admin = ["web_smoke_admin_access=granted", "web_smoke_admin_mfa=confirmed", "web_smoke_admin_page=read"]
        wallet = ["web_smoke_wallet_accounts=3", "web_smoke_wallet_ledger_entries=7", "web_smoke_wallet=read-reconciled",
                  "web_smoke_marketplace_enabled=" + ("true" if state["marketplace"] else "false")]
        if mode == "no-admin":
            print("No active identity holds admin.access; bootstrap the first administrator.", file=sys.stderr)
            sys.exit(41)
        if mode == "no-mfa":
            print("No administrator has confirmed MFA.", file=sys.stderr)
            sys.exit(43)
        if mode == "unreconciled":
            print("\n".join(admin))
            print("A wallet balance does not equal the sum of its ledger deltas.", file=sys.stderr)
            sys.exit(53)
        if mode == "silent":
            sys.exit(0)
        if mode == "no-wallet":
            print("\n".join(admin))
            sys.exit(0)
        if mode == "no-switch":
            wallet = wallet[:-1]
        print("\n".join(admin + wallet))
        sys.exit(0)
    if rest[:3] == ["exec", "-T", "platform"]:
        command = rest[rest.index("artisan") + 1]
        if command == "game-auth:oauth-client:ensure":
            assert "--rust" in rest, rest
            if state["client_id"]:
                print("Oteryn native OAuth client id: " + state["client_id"])
            sys.exit(0)
        if command == "game-auth:native-evidence:observe-trust":
            key = next((a.split("=", 1)[1] for a in rest if a.startswith("--key-id=")), None)
            phase = state["phase"] if state["phase"] == "native" else ("post" if state["init_done"] else "pre")
            state["observe_calls"] += 1
            if state.get("observe_fail_at") == state["observe_calls"]:
                save()
                print("Killed")
                sys.exit(137)
            obs = state["observations"][phase]
            if phase == "native" and state["native_queue"]:
                obs = state["native_queue"].pop(0)
            save()
            if "key" in obs:
                key = key or obs["key"]["configured_key_id"]
                if key is None:
                    print("Native signing trust observation failed: Supply a valid --key-id.", file=sys.stderr)
                    sys.exit(1)
                obs["key"]["requested"] = key
            print(json.dumps(obs))
            sys.exit(0 if obs["result"] == "observed" else 1)
        if command == "game-auth:native-admission:self-check":
            state["self_check_calls"] += 1
            save()
            if state["self_check_ok"]:
                print(json.dumps({"result": "ok", "key_id": state["kid"]}))
                sys.exit(0)
            print("ERROR  Native admission signing self-check failed: signature mismatch.")
            sys.exit(1)
        if command == "game-auth:native-evidence:init-provenance":
            state["init_calls"] += 1
            result = state["init_result"]
            if result is None:
                save()
                print("Killed")
                sys.exit(137)
            ok = result["result"] in ("created", "completed", "unchanged")
            state["init_done"] = state["init_done"] or ok
            save()
            print(json.dumps(result))
            if not ok:
                print("ERROR  Native evidence provenance init refused.")
            sys.exit(0 if ok else 1)
    sys.exit(0)
if args[:1] == ["inspect"]:
    log({"inspect": args[-1]})
    # A "<NAME>=@env-file" entry takes its value from the environment file, so state.json holds no secret.
    env_lines = dict(line.split("=", 1) for line in open(os.path.join(stub, "..", ".env")).read().splitlines() if "=" in line)
    print("\n".join(f"{line[:-10]}={env_lines[line[:-10]]}" if line.endswith("=@env-file") else line
                    for line in state["container_env"][args[-1]]))
    sys.exit(0)
if args[:1] == ["run"]:
    log({"run": args[:args.index("python3")]})
    i = args.index("python3")
    os.execv(sys.executable, [sys.executable, *args[i + 1:]])
log({"other": args})
sys.exit(0)
"""

GH_STUB = r"""#!/usr/bin/env python3
# Fake gh api: the anchor issue (locked state, comments in two pages) and comment posts by the bot.
import json, os, re, sys
stub = os.environ["STUB_DIR"]
path = os.path.join(stub, "state.json")
state = json.load(open(path))
args = sys.argv[1:]
assert args[0] == "api", args
method, body, paginate, target, i = "GET", None, False, None, 1
while i < len(args):
    if args[i] == "-X":
        method, i = args[i + 1], i + 2
    elif args[i] == "-f":
        body, i = args[i + 1].split("=", 1)[1], i + 2
    elif args[i] == "--paginate":
        paginate, i = True, i + 1
    else:
        target, i = args[i], i + 1
with open(os.path.join(stub, "gh.log"), "a") as handle:
    handle.write(json.dumps({"method": method, "target": target, "paginate": paginate}) + "\n")
m = re.fullmatch(r"repos/([^/]+/[^/]+)/issues/([0-9]+)(/comments)?(\?.*)?", target or "")
if not m or m.group(1) != "Oteryn/Oteryn-Platform" or m.group(2) != state["issue"]:
    sys.exit(1)
if not m.group(3):
    print(json.dumps({"number": int(m.group(2)), "locked": state["locked"]}))
elif method == "POST":
    limit = state["post_limit"]
    if state["post_fail"] or (limit is not None and len([c for c in state["comments"] if c["user"] == state["poster"]]) >= limit):
        print("HTTP 403: Resource not accessible by integration", file=sys.stderr)
        sys.exit(1)
    comment = {"id": state["next_id"], "user": state["poster"], "body": body}
    state["next_id"] += 1
    state["comments"].append(comment)
    json.dump(state, open(path, "w"))
    print(json.dumps(comment))
else:
    comments = state["comments"]
    half = len(comments) // 2
    print(json.dumps(comments[:half]))
    print(json.dumps(comments[half:]))
"""

SLEEP_STUB = r"""#!/usr/bin/env bash
printf '%s\n' "$*" >> "$STUB_DIR/sleep.log"
"""

KEY_STUB = r"""#!/usr/bin/env bash
exec python3 "$STUB_DIR/key.py" "$@"
"""

KEY_PY = r"""import json, os, sys
stub = os.environ["STUB_DIR"]
state = json.load(open(os.path.join(stub, "state.json")))
with open(os.path.join(stub, "key.log"), "a") as handle:
    handle.write(json.dumps({"args": sys.argv[1:], "env_file": os.environ.get("OTERYN_ENV_FILE"),
                             "platform_image_set": "PLATFORM_IMAGE" in os.environ}) + "\n")
if sys.argv[1] == "show":
    print("current_key_id=" + state["kid"])
    print("current_public_key=__PUBLIC__")
elif sys.argv[1] == "check-kid-history":
    n = state["kid_count"]
    print(f"kid_history current {state['kid']} count={n}")
    print(f"kid_history kid:{sys.argv[3]} {sys.argv[3]} count={n}")
    sys.exit(0 if n == 1 else 1)
else:
    sys.exit(2)
""".replace("__PUBLIC__", PUBLIC_KEY)

EVIDENCE_WRAPPER = r"""import json, os, subprocess, sys
# The real tool, except the host-location check (it inspects the runner container) and the evidence group.
args = sys.argv[1:]
if args[0] == "check-location":
    with open(os.path.join(os.environ["STUB_DIR"], "location.log"), "a") as handle:
        handle.write(json.dumps(args) + "\n")
    print(json.dumps({"ok": True, "errors": [], "path": "/var/lib/oteryn-attempt-evidence",
                      "canonical_source": "/volume1/oteryn-attempt-evidence", "share": "/volume1/oteryn-attempt-evidence",
                      "dir_append_only": None}))
    sys.exit(0)
if args[0] == "open":
    args += ["--group-gid", str(os.getgid())]
sys.exit(subprocess.run([sys.executable, "-I", __TOOL__, *args]).returncode)
""".replace("__TOOL__", repr(str(TOOL)))


class FakeHttp:
    """Platform and Gateway on loopback ports, answering the smoke requests."""

    def __init__(self):
        self.platform_mode = "ok"
        self.gateway_on = True
        self.bazaar = False
        self.identities = {}
        self.signed_in = None
        self.smoke_posts = []
        self.native_mode = "ok"
        self.native = {}
        owner = self

        class Platform(http.server.BaseHTTPRequestHandler):
            def log_message(self, *a):
                pass

            def do_GET(self):
                if self.path == "/health":
                    return self.reply(200, "application/json", b'{"status":"ok"}')
                if self.path == "/login?locale=en":
                    action = "https://oteryn.molehill.cloud/login?locale=en"
                    if owner.platform_mode == "bad-action" or self.headers.get("X-Forwarded-Proto") != "https":
                        action = "http://127.0.0.1/login?locale=en"
                    body = (f'<form method="POST" action="{action}"><input type="hidden" name="_token" '
                            f'value="{CSRF}"></form>').encode()
                    return self.reply(200, "text/html; charset=UTF-8", body, cookie=True)
                if self.path == "/news?locale=en":
                    status = 500 if owner.platform_mode == "news-error" else 200
                    return self.reply(status, "text/html; charset=UTF-8", b"<html>news</html>")
                if self.path == "/register?locale=en":
                    return self.reply(200, "text/html", f'<input type="hidden" name="_token" value="{CSRF}">'.encode(),
                                      cookie=True)
                if self.path == "/account":
                    if not self.session():
                        return self.redirect("/login")
                    status = 500 if owner.platform_mode == "account-error" else 200
                    return self.reply(status, "text/html", (f'<dd>{owner.signed_in}</dd><form method="POST"><input '
                                                            f'type="hidden" name="_token" value="{CSRF}"></form>').encode())
                if self.path.startswith("/oauth/authorize?"):
                    if not self.session():
                        return self.redirect("/login")
                    query = dict(urllib.parse.parse_qsl(urllib.parse.urlsplit(self.path).query))
                    expected = {"client_id": CLIENT_ID, "redirect_uri": "http://127.0.0.1/callback",
                                "response_type": "code", "scope": "game:ticket", "code_challenge_method": "S256"}
                    if any(query.get(k) != v for k, v in expected.items()) or not query.get("code_challenge"):
                        return self.reply(400, "text/html", b"invalid request")
                    owner.native["authorize"] = query
                    if owner.native_mode == "oauth-denied":
                        return self.location(f"http://127.0.0.1/callback?error=access_denied&state={query['state']}")
                    body = (f'<form method="POST"><input type="hidden" name="_token" value="{CSRF}"><input '
                            f'type="hidden" name="auth_token" value="{html.escape(AUTH_TOKEN)}"></form>').encode()
                    return self.reply(200, "text/html", body)
                if self.path == "/admin":
                    if not self.session():
                        return self.redirect("/login")
                    return self.reply(200 if owner.platform_mode == "admin-open" else 403, "text/html", b"admin")
                if self.path == "/bazaar" and owner.bazaar:
                    return self.reply(200, "text/html", b"bazaar")
                if self.path == "/account/bazaar" and owner.bazaar and self.session():
                    return self.reply(500 if owner.platform_mode == "wallet-error" else 200, "text/html", b"wallet")
                return self.reply(404, "text/html", b"not found")

            def session(self):
                return owner.signed_in is not None and f"oteryn_session={COOKIE}" in self.headers.get("Cookie", "")

            def do_POST(self):
                length = int(self.headers.get("Content-Length", "0"))
                raw = self.rfile.read(length)
                if self.path == "/oauth/token":
                    return self.token_exchange(dict(urllib.parse.parse_qsl(raw.decode())))
                if self.path == "/api/v1/game-auth/tickets":  # routes/api.php: under the api prefix
                    return self.ticket_issue(raw)
                form = dict(urllib.parse.parse_qsl(raw.decode()))
                owner.smoke_posts.append((self.path, sorted(form)))
                if form.get("_token") != CSRF or f"oteryn_session={COOKIE}" not in self.headers.get("Cookie", ""):
                    return self.reply(419, "text/html", b"page expired")
                if self.path == "/register":
                    if owner.platform_mode == "register-error":
                        return self.reply(500, "text/html", b"error")
                    if form.get("password") == form.get("password_confirmation") and form.get("email"):
                        owner.identities[form["email"]] = form["password"]
                    return self.redirect("/register")
                if self.path == "/login?locale=en":
                    if owner.platform_mode != "login-fails" and owner.identities.get(form.get("email")) == form.get("password"):
                        if owner.native_mode == "actor-mfa":
                            return self.redirect("/mfa/challenge")
                        owner.signed_in = form["email"]
                        # Laravel redirects to url('/'), the bare origin.
                        return self.redirect("")
                    return self.redirect("/login")
                if self.path == "/oauth/authorize":
                    pending = owner.native.get("authorize") or {}
                    if (not self.session() or form.get("auth_token") != AUTH_TOKEN or form.get("state") != pending.get("state")
                            or form.get("client_id") != CLIENT_ID):
                        return self.reply(400, "text/html", b"invalid request")
                    owner.native["code"] = "code-" + pending["state"][:8]
                    return self.location("http://127.0.0.1/callback?" + urllib.parse.urlencode(
                        {"code": owner.native["code"], "state": pending["state"]}))
                if self.path == "/logout":
                    if owner.platform_mode != "logout-keeps":
                        owner.signed_in = None
                    return self.redirect("/")
                return self.reply(404, "text/html", b"not found")

            def token_exchange(self, form):
                pending = owner.native.get("authorize") or {}
                challenge = base64.urlsafe_b64encode(
                    hashlib.sha256(form.get("code_verifier", "").encode()).digest()).decode().rstrip("=")
                if (owner.native_mode == "token-error" or form.get("grant_type") != "authorization_code"
                        or form.get("code") != owner.native.get("code") or form.get("client_id") != CLIENT_ID
                        or form.get("redirect_uri") != "http://127.0.0.1/callback"
                        or challenge != pending.get("code_challenge")):
                    return self.json(400, {"error": "invalid_grant"})
                owner.native["code"] = None
                owner.native["bearer"] = "live"
                return self.json(200, {"token_type": "Bearer", "expires_in": 600, "access_token": BEARER})

            def ticket_issue(self, raw):
                owner.native["ticket_posts"] = owner.native.get("ticket_posts", 0) + 1
                if (self.headers.get("Authorization") != f"Bearer {BEARER}" or owner.native.get("bearer") != "live"
                        or json.loads(raw or b"{}") != {"protocol_version": 1}):
                    return self.json(401, {"message": "Unauthenticated."})
                if owner.native_mode != "bearer-kept":
                    owner.native["bearer"] = "revoked"
                expires = 0 if owner.native_mode == "ticket-expired" else 60
                return self.json(200, {"protocol_version": 1, "ticket": TICKET, "expires_in": expires})

            def json(self, status, payload):
                self.reply(status, "application/json", json.dumps(payload).encode())

            def redirect(self, path):
                self.location(f"https://oteryn.molehill.cloud{path}")

            def location(self, url):
                self.send_response(302)
                self.send_header("Location", url)
                self.send_header("Content-Length", "0")
                self.end_headers()

            def reply(self, status, ctype, body, cookie=False):
                self.send_response(status)
                self.send_header("Content-Type", ctype)
                if cookie:
                    self.send_header("Set-Cookie", f"oteryn_session={COOKIE}; path=/; httponly")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

        class Gateway(http.server.BaseHTTPRequestHandler):
            def log_message(self, *a):
                pass

            def do_POST(self):
                length = int(self.headers.get("Content-Length", "0"))
                payload = json.loads(self.rfile.read(length) or b"{}")
                if self.path != "/v1/login" or payload.get("protocol_version") != 2 or not owner.gateway_on:
                    status, body = 400, {"error": "invalid_request"}
                elif payload.get("game_login_ticket") == TICKET and payload.get("character_id") == NATIVE_CHARACTER:
                    status, body = self.admission(payload["attempt_ref"])
                else:
                    status, body = 401, {"error": {"code": "NATIVE_LOGIN_AUTHENTICATION_REQUIRED"}}
                raw = json.dumps(body).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(raw)))
                self.end_headers()
                self.wfile.write(raw)

            def admission(self, attempt_ref):
                logins = owner.native.setdefault("logins", [])
                logins.append(attempt_ref)
                mode = owner.native_mode
                if mode == "busy-once" and len(logins) == 1:
                    return 503, {"error": {"code": "ROUTE_UNAVAILABLE", "retryable": True}}
                if mode == "conflict":
                    return 409, {"error": {"code": "CHARACTER_CONFLICT"}}
                if mode == "route-unpublished":
                    return 404, {"error": {"code": "ROUTE_NOT_FOUND"}}
                channel = "channel-2" if mode == "replay-differs" and len(logins) > 1 else "channel-1"
                grant = {"profile": "oteryn-pre-admission-v0" if mode == "bad-grant" else "oteryn-pre-admission-v1",
                         "token": GRANT, "valid_for_seconds": 30}
                return 200, {"protocol_version": 2, "attempt_ref": attempt_ref, "world_id": "world-1",
                             "channel_id": channel, "endpoint": {"host": "game.molehill.cloud", "port": 7000},
                             "grant": grant}

        self.servers = [http.server.ThreadingHTTPServer(("127.0.0.1", 0), Platform),
                        http.server.ThreadingHTTPServer(("127.0.0.1", 0), Gateway)]
        for server in self.servers:
            threading.Thread(target=server.serve_forever, daemon=True).start()
        self.platform_port = self.servers[0].server_address[1]
        self.gateway_port = self.servers[1].server_address[1]

    def close(self):
        for server in self.servers:
            server.shutdown()
            server.server_close()


class HookHarness(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.http = FakeHttp()

    @classmethod
    def tearDownClass(cls):
        cls.http.close()

    def setUp(self):
        self.http.platform_mode = "ok"
        self.http.gateway_on = True
        self.http.bazaar = False
        self.http.identities = {NATIVE_EMAIL: NATIVE_PASSWORD}  # the owner-provisioned native smoke actor
        self.http.signed_in = None
        self.http.smoke_posts = []
        self.http.native_mode = "ok"
        self.http.native = {}
        self.tmp = Path(tempfile.mkdtemp(prefix="cutover-hooks-test-"))
        self.stub = self.tmp / "stub"
        self.bin = self.tmp / "bin"
        self.evidence = self.tmp / "evidence"
        self.scratch = self.tmp / "scratch"
        self.state_dir = self.tmp / "state"
        for d in (self.stub, self.bin, self.evidence, self.scratch, self.state_dir):
            d.mkdir()
        self.write_exec(self.bin / "docker", DOCKER_STUB)
        self.write_exec(self.bin / "gh", GH_STUB)
        self.write_exec(self.bin / "sleep", SLEEP_STUB)
        self.write_exec(self.stub / "key-tool.sh", KEY_STUB)
        (self.stub / "key.py").write_text(KEY_PY)
        (self.stub / "evidence.py").write_text(EVIDENCE_WRAPPER)
        (self.tmp / "compose.yml").write_text("services: {}\n")
        self.env_file = self.tmp / ".env"
        self.write_env(native=False)
        (self.state_dir / "current-release.env").write_text(
            f"RELEASE_SHA={'d' * 40}\nPLATFORM_SOURCE_SHA={'e' * 40}\nGATEWAY_SOURCE_SHA={'f' * 40}\n"
            f"PLATFORM_IMAGE={PLATFORM_IMAGE}\nGATEWAY_IMAGE={GATEWAY_IMAGE}\n")
        self.write_cutover_state(TASK, CLOSE_RUN)
        self.state = {
            "phase": "cutover", "init_done": False, "init_calls": 0, "observe_calls": 0,
            "self_check_ok": True, "self_check_calls": 0, "native_queue": [], "post_limit": None,
            "init_result": {"result": "created", "store_id": S1, "entries": 0},
            "observations": {
                "pre": observation(store=None, binding="none", lsn=0x1000),
                "post": observation(lsn=0x1100),
                "native": COMMITTED,
            },
            "container_env": {
                "cid-platform": ["APP_ENV=staging", "GAME_AUTH_NATIVE_ADMISSION_ENABLED=true",
                                 "APP_KEY=@env-file"],
                "cid-gateway": ["GATEWAY_NATIVE_LOGIN_ENABLED=true"],
            },
            "issue": ISSUE, "locked": True, "post_fail": False, "next_id": 9001, "poster": BOT,
            "comments": [{"id": 9000, "user": {"id": 5, "login": "someone"}, "body": "not an anchor"}],
            "kid": KID, "kid_count": 1,
            "probe": "ok", "marketplace": False, "client_id": CLIENT_ID,
        }
        self.save_state()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    # --- helpers -----------------------------------------------------------------------------------

    @staticmethod
    def write_exec(path, text):
        path.write_text(text)
        path.chmod(0o755)

    def write_env(self, native):
        lines = [
            "COMPOSE_PROJECT_NAME=oteryn-staging",
            "OTERYN_RELEASE_SHA=rollback-placeholder",
            "PLATFORM_SOURCE_SHA=rollback-placeholder",
            "GATEWAY_SOURCE_SHA=rollback-placeholder",
            "PLATFORM_IMAGE=rollback-placeholder",
            "GATEWAY_IMAGE=rollback-placeholder",
            "GATEWAY_VERSION=rollback-placeholder",
            f"PLATFORM_PORT={self.http.platform_port}",
            f"GATEWAY_PORT={self.http.gateway_port}",
            f"OTERYN_NATIVE_ADMISSION_ENABLED={'true' if native else 'false'}",
        ] + [f"{k}={v}" for k, v in SECRETS.items()]
        self.env_file.write_text("\n".join(lines) + "\n")

    def write_cutover_state(self, task, close_run, reenable=True):
        # cutover-postgres.sh state: preflight records the task, freeze the close run, native-reenable its marker.
        lines = [f"TASK_ID={task}", "STEP_FREEZE=2026-10-13T10:00:00Z", f"CLOSE_RUN_ID={close_run}"]
        if reenable:
            lines += ["STEP_REENABLE_STARTED=2026-10-13T11:00:00Z", f"REENABLE_CLOSE_RUN_ID={close_run}"]
        path = self.state_dir / "postgres-cutover" / "cutover-state.env"
        path.parent.mkdir(exist_ok=True)
        path.write_text("\n".join(lines) + "\n")

    def save_state(self):
        (self.stub / "state.json").write_text(json.dumps(self.state))

    def load_state(self):
        return json.loads((self.stub / "state.json").read_text())

    def env(self, **overrides):
        env = {
            "PATH": f"{self.bin}:{os.environ['PATH']}",
            "HOME": str(self.tmp),
            "TMPDIR": str(self.scratch),
            "STUB_DIR": str(self.stub),
            "OTERYN_ENV_FILE": str(self.env_file),
            "OTERYN_STATE_DIR": str(self.state_dir),
            "OTERYN_CUTOVER_COMPOSE_FILE": str(self.tmp / "compose.yml"),
            "OTERYN_ATTEMPT_EVIDENCE_DIR": str(self.evidence),
            "OTERYN_ATTEMPT_EVIDENCE_TOOL": str(self.stub / "evidence.py"),
            "OTERYN_CUTOVER_KEY_SCRIPT": str(self.stub / "key-tool.sh"),
            "OTERYN_CUTOVER_ANCHOR_ISSUE": ISSUE,
            "OTERYN_CUTOVER_TASK_ID": TASK,
            "OTERYN_CUTOVER_CLOSE_RUN_ID": CLOSE_RUN,
            "GITHUB_REPOSITORY": "Oteryn/Oteryn-Platform",
            "GH_TOKEN": GH_TOKEN,
            "OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL": NATIVE_EMAIL,
            "OTERYN_CUTOVER_NATIVE_SMOKE_PASSWORD": NATIVE_PASSWORD,
            "OTERYN_CUTOVER_NATIVE_SMOKE_CHARACTER_ID": NATIVE_CHARACTER,
        }
        for key, value in overrides.items():
            if value is None:
                env.pop(key, None)
            else:
                env[key] = value
        return env

    def run_hook(self, name, *args, **env):
        return subprocess.run(["bash", str(HOOKS / f"{name}.sh"), *args], env=self.env(**env),
                              capture_output=True, text=True, timeout=120)

    def assert_no_secret(self, proc):
        text = proc.stdout + proc.stderr
        for value in [*SECRETS.values(), GH_TOKEN, PUBLIC_KEY, COOKIE, CSRF, NATIVE_EMAIL, NATIVE_PASSWORD, AUTH_TOKEN,
                      BEARER, TICKET, GRANT]:
            self.assertNotIn(value, text)

    def log(self, name):
        path = self.stub / name
        return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []

    def run_records(self, count=1):
        files = sorted(self.evidence.glob("*.jsonl"), key=lambda f: json.loads(f.read_text().splitlines()[0])["at"])
        self.assertEqual(len(files), count, files)
        return [json.loads(line) for line in files[-1].read_text().splitlines()]

    def all_records(self):
        return {f.name[:-6]: [json.loads(line) for line in f.read_text().splitlines()] for f in self.evidence.glob("*.jsonl")}

    def update_state(self, **values):
        self.state = self.load_state()
        self.state.update(values)
        self.save_state()

    def assert_verified(self):
        code, payload = self.tool("verify", "--dir", str(self.evidence), "--anchors", str(self.anchors_file()))
        self.assertEqual(code, 0, payload)

    def classify(self):
        return self.tool("classify", "--dir", str(self.evidence), "--anchors", str(self.anchors_file()))[1]

    def outputs(self, proc):
        return dict(line.split("=", 1) for line in proc.stdout.splitlines() if "=" in line)

    def anchors_file(self):
        state = self.load_state()
        doc = {"locked": state["locked"], "comments": [
            {"id": c["id"], "author": {"id": c["user"]["id"], "login": c["user"]["login"]}, "body": c["body"]}
            for c in state["comments"]]}
        path = self.tmp / "anchors.json"
        path.write_text(json.dumps(doc))
        return path

    def tool(self, *args):
        proc = subprocess.run([sys.executable, "-I", str(TOOL), *args], capture_output=True, text=True)
        return proc.returncode, json.loads(proc.stdout) if proc.stdout.strip() else None

    def posts(self):
        return [entry for entry in self.log("gh.log") if entry["method"] == "POST"]

    def assert_compose_used_derived_env(self):
        compose = [entry for entry in self.log("docker.log") if "compose" in entry]
        self.assertTrue(compose)
        for entry in compose:
            self.assertEqual(entry["image"], PLATFORM_IMAGE)
            self.assertNotEqual(entry["env_file"], str(self.env_file))
            self.assertFalse(Path(entry["env_file"]).exists(), "the derived environment file must be removed")
        self.assertEqual(list(self.scratch.iterdir()), [])


class ProvenanceInitTest(HookHarness):
    def test_happy_path_writes_anchored_intent_accepted_by_the_fence_check(self):
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        out = dict(line.split("=", 1) for line in proc.stdout.splitlines())
        self.assertEqual(out["provenance_init_result"], "created")
        self.assertEqual(out["witness_store_id"], S1)
        records = self.run_records()
        self.assertEqual([r["type"] for r in records],
                         ["run-open", "init-intent", "anchored", "init", "attempt-intent", "anchored"])
        self.assertEqual(records[0]["cutover_ref"], f"{TASK}/close-{CLOSE_RUN}")
        self.assertEqual(records[0]["pre_init"]["key"]["requested"], KID)
        intent = records[4]
        self.assertEqual((intent["store_id"], intent["key_id"], intent["scope"]), (S1, KID, SCOPE))
        self.assertEqual(intent["kid_record_count"], 1)
        self.assertIs(intent["anchor_issue_locked"], True)
        state = self.load_state()
        self.assertEqual(state["init_calls"], 1)
        # The write probe, then the anchors of run-open, init-intent, init and attempt-intent.
        bodies = [c["body"] for c in state["comments"] if c["user"] == BOT]
        self.assertEqual(json.loads(bodies[0]), {"oteryn_cutover_preflight": f"{TASK}/close-{CLOSE_RUN}"})
        self.assertEqual([json.loads(b)["type"] for b in bodies[1:]], ["run-open", "init-intent", "init", "attempt-intent"])
        anchors = self.anchors_file()
        code, payload = self.tool("verify", "--dir", str(self.evidence), "--anchors", str(anchors))
        self.assertEqual(code, 0, payload)
        identity = json.dumps(self.state["observations"]["post"]["database"]["identity"])
        code, payload = self.tool("require-open-attempt", "--dir", str(self.evidence), "--store", S1, "--key-id", KID,
                                  "--scope", json.dumps(SCOPE), "--identity", identity, "--anchors", str(anchors))
        self.assertEqual(code, 0, payload)
        self.assertEqual(payload["intent_sha256"], out["attempt_intent_line_sha256"])
        self.assertEqual(payload["run_id"], out["attempt_run_id"])
        # Derived runtime environment, location check on the deployed project, key tool on the derived file.
        self.assert_compose_used_derived_env()
        self.assertIn(["--project", "oteryn-staging"], [l[l.index("--project"):l.index("--project") + 2]
                                                         for l in self.log("location.log")])
        for entry in self.log("key.log"):
            self.assertFalse(entry["platform_image_set"])
            self.assertNotEqual(entry["env_file"], str(self.env_file))
        self.assert_no_secret(proc)

    def assert_nothing_written(self, proc):
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(list(self.evidence.iterdir()), [])
        self.assertEqual(self.load_state()["init_calls"], 0)
        self.assert_no_secret(proc)

    def test_refuses_without_anchor_issue(self):
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN, OTERYN_CUTOVER_ANCHOR_ISSUE=None)
        self.assert_nothing_written(proc)
        self.assertIn("OTERYN_CUTOVER_ANCHOR_ISSUE", proc.stderr)
        self.assertEqual(self.log("docker.log"), [])

    def test_refuses_another_repository(self):
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN, GITHUB_REPOSITORY="Oteryn/Oteryn-Game")
        self.assert_nothing_written(proc)

    def test_refuses_unlocked_anchor_issue(self):
        self.state["locked"] = False
        self.save_state()
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assert_nothing_written(proc)
        self.assertEqual(self.posts(), [])

    def test_refuses_when_the_bot_cannot_post(self):
        self.state["post_fail"] = True
        self.save_state()
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assert_nothing_written(proc)
        self.assertIn("issues: write", proc.stderr)

    def test_refuses_a_post_by_another_author(self):
        self.state["poster"] = {"id": 5, "login": "someone"}
        self.save_state()
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assert_nothing_written(proc)

    def test_refuses_kid_history_other_than_one(self):
        for count in (0, 2):
            with self.subTest(count=count):
                self.state["kid_count"] = count
                self.save_state()
                proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
                self.assert_nothing_written(proc)
                self.assertEqual(self.posts(), [])

    def test_refused_init_closes_the_intent_with_an_anchored_begin_refused(self):
        self.state["init_result"] = {"result": "refused", "reason": "marker and row differ"}
        self.save_state()
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("init-refused:refused", proc.stderr)
        self.assertEqual([r["type"] for r in self.run_records()], ["run-open", "init-intent", "anchored", "begin-refused"])
        code, payload = self.tool("verify", "--dir", str(self.evidence), "--anchors", str(self.anchors_file()))
        self.assertEqual(code, 0, payload)
        self.assert_no_secret(proc)

    def test_refused_open_is_anchored_and_runs_no_init(self):
        self.state["observations"]["pre"] = observation(store=None, binding="none", floor=1, lsn=0x1000)
        self.save_state()
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("precondition:floor", proc.stderr)
        self.assertEqual([r["type"] for r in self.run_records()], ["run-open", "begin-refused"])
        self.assertEqual(self.load_state()["init_calls"], 0)
        code, payload = self.tool("verify", "--dir", str(self.evidence), "--anchors", str(self.anchors_file()))
        self.assertEqual(code, 0, payload)

    def test_rejects_bad_arguments(self):
        for args in ([], ["init"], ["init", TASK], ["run", TASK, CLOSE_RUN], ["init", "bad task", CLOSE_RUN],
                     ["init", TASK, "0"], ["init", TASK, "12x"], ["init", TASK, CLOSE_RUN, "extra"],
                     ["close-init", TASK, CLOSE_RUN], ["retry", TASK, CLOSE_RUN], ["rebegin", TASK, CLOSE_RUN, "x"]):
            with self.subTest(args=args):
                proc = self.run_hook("provenance-init", *args)
                self.assertEqual(proc.returncode, 1)
        self.assertEqual(self.log("docker.log"), [])
        self.assertEqual(list(self.evidence.iterdir()), [])


    def test_preflight_checks_anchor_access_without_writing_evidence(self):
        proc = self.run_hook("provenance-init", "preflight", TASK, CLOSE_RUN)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.outputs(proc), {"provenance_preflight": "passed"})
        self.assertEqual(list(self.evidence.iterdir()), [])
        state = self.load_state()
        self.assertEqual(state["init_calls"], 0)
        bodies = [json.loads(c["body"]) for c in state["comments"] if c["user"] == BOT]
        self.assertEqual(bodies, [{"oteryn_cutover_preflight": f"{TASK}/close-{CLOSE_RUN}"}])
        self.assert_no_secret(proc)
        for env, state_values, text in (({"OTERYN_CUTOVER_ANCHOR_ISSUE": None}, {}, "OTERYN_CUTOVER_ANCHOR_ISSUE"),
                                        ({}, {"locked": False}, "locked"),
                                        ({}, {"post_fail": True}, "issues: write")):
            with self.subTest(text=text):
                self.update_state(**{"locked": True, "post_fail": False, **state_values})
                proc = self.run_hook("provenance-init", "preflight", TASK, CLOSE_RUN, **env)
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn(text, proc.stderr)
                self.assertEqual(list(self.evidence.iterdir()), [])

    def test_anchor_retry_posts_an_unanchored_last_line_before_open(self):
        # The probe, run-open, init-intent and init are posted; the attempt-intent post fails.
        self.update_state(post_limit=4)
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("stays unanchored", proc.stderr)
        first = self.run_records()
        self.assertEqual(first[-1]["type"], "attempt-intent")
        code, payload = self.tool("verify", "--dir", str(self.evidence), "--anchors", str(self.anchors_file()))
        self.assertEqual(code, 1, payload)
        # The next begin retries that post (D3) before open; open then refuses the unclosed attempt.
        self.update_state(post_limit=None)
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn(f"anchor_retried={first[0]['run_id']}/{first[-1]['seq']}", proc.stdout)
        self.assertIn("attempt-unclosed", proc.stderr)
        self.assertEqual([r["type"] for r in self.run_records(count=2)], ["run-open", "begin-refused"])
        self.assertEqual(self.load_state()["init_calls"], 1)
        self.assert_verified()
        self.assert_no_secret(proc)


class CloseInitTest(HookHarness):
    def leave_unfinished_init(self, post_limit=None):
        self.update_state(init_result=None, post_limit=post_limit)
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        records = self.run_records()
        self.assertEqual(records[1]["type"], "init-intent")
        self.update_state(post_limit=None)
        return records[0]["run_id"], proc

    def test_marker_or_match_reruns_the_init_and_writes_a_late_init(self):
        run, proc = self.leave_unfinished_init()
        self.assertIn("provenance-init.sh close-init", proc.stderr)
        self.assertEqual([r["type"] for r in self.run_records()], ["run-open", "init-intent", "anchored"])
        # D1 had created the store before it died; the rerun completes it.
        self.update_state(init_done=True, init_result={"result": "completed", "store_id": S1, "entries": 0})
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        out = self.outputs(proc)
        self.assertEqual((out["close_init_result"], out["witness_store_id"]), ("init-late", S1))
        records = self.run_records()
        self.assertEqual([r["type"] for r in records],
                         ["run-open", "init-intent", "anchored", "init", "run-close"])
        self.assertIs(records[3]["late"], True)
        self.assertIs(records[4]["done"], False)
        self.assertEqual(self.load_state()["init_calls"], 2)
        self.assert_verified()
        # A new run may begin on that store.
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual([r["type"] for r in self.run_records(count=2)],
                         ["run-open", "init-intent", "anchored", "init", "attempt-intent", "anchored"])
        self.assert_verified()
        self.assert_no_secret(proc)

    def test_binding_none_after_an_anchored_intent_is_init_lost(self):
        run, _ = self.leave_unfinished_init()
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("init-lost", proc.stderr)
        records = self.run_records()
        self.assertEqual([r["type"] for r in records], ["run-open", "init-intent", "anchored", "begin-refused"])
        self.assertEqual(records[-1]["reason"], "init-lost")
        self.assertEqual(self.load_state()["init_calls"], 1)
        self.assert_verified()
        # Permanent: the next begin is refused.
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertIn("blocked:init-lost", proc.stderr)

    def test_binding_none_without_an_anchored_line_is_init_not_run(self):
        # The probe and run-open are posted; the init-intent anchor fails, so D1 never runs.
        self.update_state(post_limit=2)
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(self.load_state()["init_calls"], 0)
        run = self.run_records()[0]["run_id"]
        self.update_state(post_limit=None)
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn(f"anchor_retried={run}/2", proc.stdout)
        self.assertEqual(self.outputs(proc)["close_init_result"], "init-not-run")
        records = self.run_records()
        self.assertEqual([r["type"] for r in records], ["run-open", "init-intent", "begin-refused"])
        self.assertEqual(records[-1]["reason"], "init-not-run")
        self.assert_verified()
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assert_verified()

    def test_refuses_a_run_without_an_unfinished_intent_and_bad_ids(self):
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        run = self.outputs(proc)["attempt_run_id"]
        before = self.all_records()
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("no unfinished init-intent", proc.stderr)
        for args in (["close-init", TASK, CLOSE_RUN], ["close-init", TASK, CLOSE_RUN, "not-a-uuid"],
                     ["observe", TASK, CLOSE_RUN, run.upper()], ["close-init", TASK, CLOSE_RUN, run, "x"]):
            with self.subTest(args=args):
                self.assertEqual(self.run_hook("provenance-init", *args).returncode, 1)
        self.assertEqual(self.all_records(), before)


    def bot_posts(self):
        return len([c for c in self.load_state()["comments"] if c["user"] == BOT])

    def test_run_open_only_resumes_the_whole_begin(self):
        # The probe is posted, the run-open anchor post fails: the runner stopped after run-open.
        self.update_state(post_limit=1)
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        records = self.run_records()
        self.assertEqual([r["type"] for r in records], ["run-open"])
        self.update_state(post_limit=None)
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, records[0]["run_id"])
        self.assertEqual(proc.returncode, 0, proc.stderr)
        out = self.outputs(proc)
        self.assertEqual((out["close_init_result"], out["provenance_init_result"], out["witness_store_id"]),
                         ("resumed-begin", "created", S1))
        self.assertEqual([r["type"] for r in self.run_records()],
                         ["run-open", "init-intent", "anchored", "init", "attempt-intent", "anchored"])
        self.assertEqual(self.load_state()["init_calls"], 1)
        self.assert_verified()
        self.assert_no_secret(proc)
        self.assert_open_attempt()

    def test_run_open_only_retakes_a_refused_open_decision(self):
        # A refused open whose begin-refused line was lost with the runner: run-open is the only durable line.
        self.state["observations"]["pre"] = observation(store=None, binding="none", floor=1, lsn=0x1000)
        self.state["post_limit"] = 2
        self.save_state()
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        path = next(self.evidence.glob("*.jsonl"))
        lines = path.read_text().splitlines()
        self.assertEqual([json.loads(l)["type"] for l in lines], ["run-open", "begin-refused"])
        path.write_text(lines[0] + "\n")
        self.update_state(post_limit=None)
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, json.loads(lines[0])["run_id"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("precondition:floor", proc.stderr)
        records = self.run_records()
        self.assertEqual([r["type"] for r in records], ["run-open", "begin-refused"])
        self.assertEqual(records[1]["observation"], records[0]["pre_init"])
        self.assertEqual(self.load_state()["init_calls"], 0)
        self.assert_verified()

    def test_init_without_attempt_intent_is_resumed_and_native_smoke_waits_for_it(self):
        # pre, post-D1, then the intent observation dies: init is durable, the attempt intent is not.
        self.update_state(observe_fail_at=3)
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        records = self.run_records()
        run = records[0]["run_id"]
        self.assertEqual([r["type"] for r in records], ["run-open", "init-intent", "anchored", "init"])
        self.update_state(observe_fail_at=None)
        # The deploy cannot have an open attempt; native-smoke points to evidence-close-init and writes nothing.
        self.write_env(native=True)
        self.update_state(phase="native")
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn(f"run the evidence-close-init step for run {run}", proc.stderr)
        self.assertEqual(self.run_records(), records)
        # Native admission on: the begin is not resumed.
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("native admission is on", proc.stderr)
        self.assertEqual(self.run_records(), records)
        self.write_env(native=False)
        self.update_state(phase="cutover")
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        out = self.outputs(proc)
        self.assertEqual((out["close_init_result"], out["witness_store_id"], out["attempt_run_id"]),
                         ("resumed-attempt", S1, run))
        self.assertEqual([r["type"] for r in self.run_records()],
                         ["run-open", "init-intent", "anchored", "init", "attempt-intent", "anchored"])
        self.assertEqual(self.load_state()["init_calls"], 1)
        self.assert_verified()
        self.assert_open_attempt()
        # Then the deploy and native-smoke close the attempt as usual.
        self.write_env(native=True)
        self.update_state(phase="native")
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual([r["type"] for r in self.run_records()][-3:], ["attempt-observation", "self-check", "run-close"])

    def test_unanchored_attempt_intent_gets_its_anchored_line(self):
        self.update_state(post_limit=4)
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        records = self.run_records()
        run = records[0]["run_id"]
        self.assertEqual(records[-1]["type"], "attempt-intent")
        self.update_state(post_limit=None)
        self.write_env(native=True)
        self.update_state(phase="native")
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("has no anchored line; run the evidence-close-init step", proc.stderr)
        self.assertEqual(self.run_records(), records)
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.outputs(proc)["close_init_result"], "resumed-anchor")
        self.assertEqual([r["type"] for r in self.run_records()][-2:], ["attempt-intent", "anchored"])
        self.assert_verified()
        self.assert_open_attempt()

    def test_late_init_without_run_close_is_closed(self):
        run, _ = self.leave_unfinished_init()
        self.update_state(init_done=True, init_result={"result": "completed", "store_id": S1, "entries": 0},
                          post_limit=self.bot_posts())
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual([r["type"] for r in self.run_records()][-1], "init")
        self.update_state(post_limit=None)
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.outputs(proc)["close_init_result"], "init-late")
        records = self.run_records()
        self.assertEqual([r["type"] for r in records][-2:], ["init", "run-close"])
        self.assertIs(records[-1]["done"], False)
        self.assertEqual(self.load_state()["init_calls"], 2)
        self.assert_verified()

    def test_recovery_is_bound_to_the_task_and_close_run_of_the_cutover(self):
        run, _ = self.leave_unfinished_init()
        other_task, other_close = "OTERYN-20261013-other-cutover", "7000000002"
        before, posts = self.all_records(), self.bot_posts()
        for command in ("close-init", "observe"):
            # The arguments name another task or close run than the cutover state records.
            for task, close, text in ((other_task, CLOSE_RUN, "records another task"),
                                      (TASK, other_close, "for another close run")):
                with self.subTest(command=command, task=task, close=close):
                    proc = self.run_hook("provenance-init", command, task, close, run)
                    self.assertNotEqual(proc.returncode, 0)
                    self.assertIn(text, proc.stderr)
            # State and arguments agree, but the run was opened for another task or close run.
            for task, close in ((other_task, CLOSE_RUN), (TASK, other_close)):
                with self.subTest(command=command, task=task, close=close, state="agrees"):
                    self.write_cutover_state(task, close)
                    proc = self.run_hook("provenance-init", command, task, close, run)
                    self.assertNotEqual(proc.returncode, 0)
                    self.assertIn(f"records cutover {TASK}/close-{CLOSE_RUN}, not {task}/close-{close}", proc.stderr)
            # No native re-enable in the cutover state, or no state at all.
            self.write_cutover_state(TASK, CLOSE_RUN, reenable=False)
            proc = self.run_hook("provenance-init", command, TASK, CLOSE_RUN, run)
            self.assertNotEqual(proc.returncode, 0)
            self.assertIn("records no native re-enable", proc.stderr)
            (self.state_dir / "postgres-cutover" / "cutover-state.env").unlink()
            proc = self.run_hook("provenance-init", command, TASK, CLOSE_RUN, run)
            self.assertNotEqual(proc.returncode, 0)
            self.assertIn("cutover state file is missing", proc.stderr)
            self.write_cutover_state(TASK, CLOSE_RUN)
        self.assertEqual(self.all_records(), before)
        self.assertEqual(self.bot_posts(), posts)
        self.assertEqual(self.load_state()["init_calls"], 1)
        # With the matching task and close run the same run is closed.
        self.update_state(init_done=True, init_result={"result": "completed", "store_id": S1, "entries": 0})
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.outputs(proc)["close_init_result"], "init-late")

    def test_rebegin_takes_a_refused_begin_again_in_a_new_run(self):
        self.update_state(observations=dict(self.load_state()["observations"],
                                            pre=observation(store=None, binding="none", floor=1, lsn=0x1000)))
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        first = self.run_records()[0]["run_id"]
        # The cause is still there: open refuses again, in a second run.
        proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN, first)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("precondition:floor", proc.stderr)
        second = self.run_records(count=2)
        self.assertEqual([r["type"] for r in second], ["run-open", "begin-refused"])
        before = self.all_records()
        proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN, first)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("not the newest run of this cutover", proc.stderr)
        self.write_env(native=True)
        proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN, second[0]["run_id"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("native admission is on", proc.stderr)
        self.assertEqual(self.all_records(), before)
        # Cause fixed, native admission off: the whole begin runs in a third run.
        self.write_env(native=False)
        self.update_state(observations=dict(self.load_state()["observations"],
                                            pre=observation(store=None, binding="none", lsn=0x1000)))
        proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN, second[0]["run_id"])
        self.assertEqual(proc.returncode, 0, proc.stderr)
        out = self.outputs(proc)
        self.assertEqual((out["rebegin_of"], out["provenance_init_result"], out["witness_store_id"]),
                         (second[0]["run_id"], "created", S1))
        third = self.run_records(count=3)
        self.assertEqual(out["attempt_run_id"], third[0]["run_id"])
        self.assertEqual([r["type"] for r in third],
                         ["run-open", "init-intent", "anchored", "init", "attempt-intent", "anchored"])
        self.assert_verified()
        self.assert_open_attempt()
        proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN, third[0]["run_id"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("did not end in a begin that may be taken again", proc.stderr)

    def test_rebegin_after_init_late_but_not_after_init_lost(self):
        run, _ = self.leave_unfinished_init()
        self.update_state(init_done=True, init_result={"result": "completed", "store_id": S1, "entries": 0})
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertEqual(self.outputs(proc)["close_init_result"], "init-late")
        self.update_state(init_result={"result": "unchanged", "store_id": S1, "entries": 0})
        proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN, run)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual([r["type"] for r in self.run_records(count=2)],
                         ["run-open", "init-intent", "anchored", "init", "attempt-intent", "anchored"])
        self.assert_verified()
        self.assert_open_attempt()

    def test_rebegin_refuses_a_permanent_refusal(self):
        run, _ = self.leave_unfinished_init()
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertIn("init-lost", proc.stderr)
        before = self.all_records()
        proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN, run)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("permanent begin-refused (init-lost)", proc.stderr)
        self.assertEqual(self.all_records(), before)

    def test_init_failing_before_open_is_begun_again_without_a_run_id(self):
        # native-reenable records its forward-only marker, then each check before open fails: nothing is written
        # and the hook names the step that goes on without a run id.
        for state_values, text in (({"observe_fail_at": 1}, "trust observation"), ({"post_fail": True}, "issues: write"),
                                   ({"kid_count": 2}, "used.kids")):
            with self.subTest(text=text):
                self.update_state(**state_values)
                proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn(text, proc.stderr)
                self.assertIn("next: no new evidence run was opened", proc.stderr)
                self.assertIn("evidence-rebegin step with evidence_run_id left empty", proc.stderr)
                self.assertEqual(list(self.evidence.iterdir()), [])
                self.update_state(observe_fail_at=None, observe_calls=0, post_fail=False, kid_count=1)
        # Only while the cutover state records native re-enable for this task and close run, native admission off.
        for reenable, native, text in ((False, False, "records no native re-enable"),
                                       (True, True, "native admission is on")):
            with self.subTest(text=text):
                self.write_cutover_state(TASK, CLOSE_RUN, reenable=reenable)
                self.write_env(native=native)
                proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN)
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn(text, proc.stderr)
                self.assertEqual(list(self.evidence.iterdir()), [])
        self.write_cutover_state(TASK, CLOSE_RUN)
        self.write_env(native=False)
        proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        out = self.outputs(proc)
        records = self.run_records()
        self.assertEqual((out["rebegin_of"], out["attempt_run_id"], out["provenance_init_result"]),
                         ("none", records[0]["run_id"], "created"))
        self.assertEqual(records[0]["cutover_ref"], f"{TASK}/close-{CLOSE_RUN}")
        self.assertEqual([r["type"] for r in records],
                         ["run-open", "init-intent", "anchored", "init", "attempt-intent", "anchored"])
        self.assert_verified()
        self.assert_open_attempt()
        self.assert_no_secret(proc)
        # A run of this cutover exists now: a rebegin without a run id names it and writes nothing.
        before, posts = self.all_records(), self.bot_posts()
        proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn(f"holds run {records[0]['run_id']} (last line anchored) of this cutover", proc.stderr)
        self.assertEqual((self.all_records(), self.bot_posts()), (before, posts))

    def test_a_run_written_by_open_is_named_and_resumed(self):
        # The probe is posted, the run-open anchor post fails: the run id is printed with the failure, and a
        # rebegin without a run id (the run id lost with the runner log) names the run instead of opening another.
        self.update_state(post_limit=1)
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        run = self.run_records()[0]["run_id"]
        self.assertEqual(self.outputs(proc)["attempt_run_id"], run)
        self.assertIn(f"next: evidence run {run} of this cutover holds the begin", proc.stderr)
        self.update_state(post_limit=None)
        proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn(f"holds run {run} (last line run-open) of this cutover", proc.stderr)
        self.assertEqual(len(self.all_records()), 1)
        proc = self.run_hook("provenance-init", "close-init", TASK, CLOSE_RUN, run)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.outputs(proc)["close_init_result"], "resumed-begin")
        self.assert_open_attempt()

    def test_a_failure_after_open_names_the_run_id(self):
        # D1 prints no result: init-intent is unfinished; the hint carries the run id for evidence-close-init.
        run, proc = self.leave_unfinished_init()
        self.assertEqual(self.outputs(proc)["attempt_run_id"], run)
        self.assertIn(f"with evidence_run_id {run}.", proc.stderr)

    def assert_open_attempt(self):
        identity = json.dumps(self.load_state()["observations"]["post"]["database"]["identity"])
        code, payload = self.tool("require-open-attempt", "--dir", str(self.evidence), "--store", S1, "--key-id", KID,
                                  "--scope", json.dumps(SCOPE), "--identity", identity,
                                  "--anchors", str(self.anchors_file()))
        self.assertEqual(code, 0, payload)


class WebSmokeTest(HookHarness):
    def test_passes_on_a_healthy_platform(self):
        proc = self.run_hook("web-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(proc.stdout.splitlines()[-1], "web_smoke_result=passed")
        out = self.outputs(proc)
        self.assertEqual({k: v for k, v in out.items() if k != "web_smoke_result"}, {
            "web_smoke_admin_access": "granted", "web_smoke_admin_mfa": "confirmed", "web_smoke_admin_page": "read",
            "web_smoke_wallet_accounts": "3", "web_smoke_wallet_ledger_entries": "7",
            "web_smoke_wallet": "read-reconciled", "web_smoke_account": "registered-signed-in-signed-out",
            "web_smoke_admin_boundary": "refused-without-mfa", "web_smoke_marketplace": "off"})
        self.assertNotIn("not-exercised", proc.stdout)
        # A fresh identity is registered and signs in with the same random credentials, then signs out.
        self.assertEqual([path for path, _ in self.http.smoke_posts], ["/register", "/login?locale=en", "/logout"])
        self.assertEqual(self.http.smoke_posts[0][1], ["_token", "email", "password", "password_confirmation"])
        (email, password), = [item for item in self.http.identities.items() if item[0] != NATIVE_EMAIL]
        self.assertRegex(email, r"^cutover-smoke-[0-9a-f]{16}@example\.invalid$")
        self.assertIsNone(self.http.signed_in)
        for value in (email, password, "example.invalid"):
            self.assertNotIn(value, proc.stdout + proc.stderr)
        # The administrator and wallet probe runs in the platform service before any web write.
        calls = [entry for entry in self.log("docker.log")]
        probe = [i for i, entry in enumerate(calls) if entry.get("compose", [])[-1:] == ["php"]]
        self.assertEqual(len(probe), 1)
        self.assertEqual(calls[probe[0]]["compose"][:3], ["exec", "-T", "platform"])
        runs = [i for i, entry in enumerate(calls) if "run" in entry]
        self.assertEqual([calls[i]["run"] for i in runs], [["run", "--rm", "--network", "host", "-i", "python:3.12-alpine"]])
        self.assertLess(probe[0], runs[0])
        self.assert_compose_used_derived_env()
        self.assert_no_secret(proc)

    def test_the_marketplace_switch_must_match_what_is_served(self):
        self.update_state(marketplace=True)
        self.http.bazaar = True
        proc = self.run_hook("web-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.outputs(proc)["web_smoke_marketplace"], "read")
        self.assertEqual(self.outputs(proc)["web_smoke_wallet"], "read-reconciled")
        self.http.platform_mode = "wallet-error"
        proc = self.run_hook("web-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("/account/bazaar", proc.stderr)
        # A 404 /bazaar passes only while the switch is off; a served /bazaar while it is off fails too.
        self.http.platform_mode = "ok"
        for enabled, served, text in ((True, False, "while the Marketplace is enabled"),
                                      (False, True, "while the Marketplace is disabled")):
            with self.subTest(enabled=enabled, served=served):
                self.update_state(marketplace=enabled)
                self.http.bazaar = served
                proc = self.run_hook("web-smoke")
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn(text, proc.stderr)
                self.assertNotIn("web_smoke_result=passed", proc.stdout)

    def test_administrator_and_wallet_probe_fails_closed_before_any_web_write(self):
        cases = {
            "no-admin": "probe failed", "no-mfa": "probe failed", "unreconciled": "probe failed",
            "silent": "did not report web_smoke_admin_access=granted", "no-wallet": "did not report web_smoke_wallet",
            "no-switch": "Marketplace switch",
        }
        for mode, text in cases.items():
            with self.subTest(mode=mode):
                self.update_state(probe=mode)
                self.http.smoke_posts = []
                proc = self.run_hook("web-smoke")
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn(text, proc.stderr)
                self.assertNotIn("web_smoke_result=passed", proc.stdout)
                self.assertEqual(self.http.smoke_posts, [])
                self.assertFalse([entry for entry in self.log("docker.log") if "run" in entry])
                self.assert_no_secret(proc)

    def test_fails_closed(self):
        for mode in ("bad-action", "news-error", "register-error", "login-fails", "account-error", "admin-open",
                     "logout-keeps"):
            with self.subTest(mode=mode):
                self.http.platform_mode = mode
                proc = self.run_hook("web-smoke")
                self.assertNotEqual(proc.returncode, 0)
                self.assertNotIn("web_smoke_result=passed", proc.stdout)
                self.assert_no_secret(proc)
        proc = self.run_hook("web-smoke", "extra")
        self.assertEqual(proc.returncode, 1)


class NativeSmokeTest(HookHarness):
    def setUp(self):
        super().setUp()
        proc = self.run_hook("provenance-init", "init", TASK, CLOSE_RUN)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.run_id = self.outputs(proc)["attempt_run_id"]
        self.write_env(native=True)
        self.update_state(phase="native")

    def types(self):
        return [r["type"] for r in self.run_records()]

    def test_observes_the_attempt_closes_it_and_passes(self):
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(proc.stdout.splitlines()[-1], "native_smoke_result=passed")
        out = self.outputs(proc)
        self.assertEqual((out["attempt_run_id"], out["attempt_classification"], out["self_check"]),
                         (self.run_id, "committed", "passed"))
        records = self.run_records()
        self.assertEqual([r["type"] for r in records][-3:], ["attempt-observation", "self-check", "run-close"])
        observed, check, close = records[-3:]
        self.assertEqual((observed["classification"], observed["late"], observed["deploy_exit"]), ("committed", False, None))
        self.assertEqual(observed["observation_1"]["key"]["requested"], KID)
        self.assertEqual((check["passed"], check["late"], check["result_line"]), (True, False, {"result": "ok", "key_id": KID}))
        self.assertIs(close["done"], True)
        self.assertIn("5", (self.stub / "sleep.log").read_text().split())
        self.assert_verified()
        self.assertEqual(self.classify()["directory"], "committed")
        self.assert_compose_used_derived_env()
        self.assert_no_secret(proc)
        self.assertEqual({k: out[k] for k in out if k.startswith("native_smoke_") and k != "native_smoke_result"}, {
            "native_smoke_unknown_ticket": "refused", "native_smoke_oauth": "authorized",
            "native_smoke_ticket": "issued-bearer-revoked", "native_smoke_admission": "granted",
            "native_smoke_replay": "same-admission", "native_smoke_login": "end-to-end"})
        # The actor's ticket was admitted, then replayed with the same attempt; the bearer was refused after issue.
        logins = self.http.native["logins"]
        self.assertEqual(len(logins), 2)
        self.assertEqual(logins[0], logins[1])
        self.assertEqual((self.http.native["ticket_posts"], self.http.native["bearer"]), (2, "revoked"))
        self.assertIsNone(self.http.signed_in)
        self.assertEqual([path for path, _ in self.http.smoke_posts], ["/login?locale=en", "/oauth/authorize", "/logout"])
        # The actor's credentials reach the request container by name only.
        runs = [entry["run"] for entry in self.log("docker.log") if "run" in entry]
        self.assertEqual(runs, [["run", "--rm", "--network", "host", "-e", "OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL",
                                 "-e", "OTERYN_CUTOVER_NATIVE_SMOKE_PASSWORD", "-i", "python:3.12-alpine"]])
        logged = (self.stub / "docker.log").read_text()
        for value in (NATIVE_EMAIL, NATIVE_PASSWORD):
            self.assertNotIn(value, logged)
        # A rerun leaves the closed attempt as it is.
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.run_records(), records)

    def test_rerun_after_close_passes_only_on_a_database_that_continues_the_attempt(self):
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        records = self.run_records()

        def variant(change):
            obs = json.loads(json.dumps(COMMITTED))
            change(obs)
            return obs

        def swap(obs, **identity):
            obs["database"]["identity"].update(identity)

        cases = {
            "other cluster": (variant(lambda o: swap(o, system_identifier="7999")), "does not continue"),
            "new timeline": (variant(lambda o: swap(o, timeline_id=2)), "does not continue"),
            "restored relfilenodes": (variant(lambda o: swap(o, relfilenodes=[4, 5, 6])), "does not continue"),
            "WAL behind": (variant(lambda o: o["database"]["progress"].update(wal_flush_lsn="0/1800")), "does not continue"),
            "in flight": (variant(lambda o: o["database"]["in_flight"].update(sessions=1)), "never settled"),
        }
        for name, (obs, text) in cases.items():
            with self.subTest(name=name):
                self.update_state(observations=dict(self.load_state()["observations"], native=obs))
                proc = self.run_hook("native-smoke")
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn(text, proc.stderr)
                self.assertNotIn("attempt_result=committed", proc.stdout)
                self.assertNotIn("native_smoke_result=passed", proc.stdout)
                self.assertEqual(self.run_records(), records)
                self.assert_no_secret(proc)
        # A restart (new postmaster start time) with WAL moved forward still continues the attempt.
        restarted = variant(lambda o: (swap(o, postmaster_start_time="t9"),
                                       o["database"]["progress"].update(wal_flush_lsn="0/3000", xmax="200")))
        self.update_state(observations=dict(self.load_state()["observations"], native=restarted))
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.outputs(proc)["attempt_result"], "committed")
        self.assertEqual(self.run_records(), records)

    def test_final_observation_must_continue_the_attempt_it_just_closed(self):
        swapped = json.loads(json.dumps(COMMITTED))
        swapped["database"]["identity"]["system_identifier"] = "7999"
        self.update_state(phase="native", native_queue=[COMMITTED, COMMITTED, swapped])
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("does not continue the committed attempt", proc.stderr)
        self.assertNotIn("native_smoke_result=passed", proc.stdout)
        self.assertIs(self.run_records()[-1]["done"], True)

    def test_finished_attempt_is_accepted_only_while_its_anchors_verify(self):
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        records = self.run_records()
        comments = self.load_state()["comments"]
        # An earlier anchor disappears from the issue: the rerun refuses instead of taking the finished branch.
        intent = next(c for c in comments if c["user"] == BOT and json.loads(c["body"]).get("type") == "attempt-intent")
        self.update_state(comments=[c for c in comments if c is not intent and c != intent])
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("verify --anchors fails", proc.stderr)
        self.assertNotIn("native_smoke_result=passed", proc.stdout)
        # A rewritten run file is refused the same way, with every anchor in place.
        self.update_state(comments=comments)
        path = next(p for p in self.evidence.iterdir() if p.name.endswith(".jsonl") and records[0]["run_id"] in p.read_text())
        text = path.read_text()
        path.write_text(text.replace('"passed":true', '"passed":false', 1) if '"passed":true' in text
                        else text.replace('"passed": true', '"passed": false', 1))
        self.assertNotEqual(path.read_text(), text)
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("verify --anchors fails", proc.stderr)
        self.assertEqual(len(self.run_records()), len(records))

    def test_waits_while_the_witness_is_busy_or_a_transaction_is_in_flight(self):
        busy = {"result": "witness_busy"}
        in_flight = json.loads(json.dumps(COMMITTED))
        in_flight["database"]["in_flight"]["sessions"] = 1
        self.update_state(native_queue=[busy, COMMITTED, in_flight, COMMITTED])
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        observed = self.run_records()[-3]
        self.assertEqual(observed["classification"], "committed")
        self.assertEqual(observed["observation_1"]["database"]["in_flight"]["sessions"], 0)

    def test_failed_self_check_keeps_the_run_open_and_the_next_run_rechecks(self):
        self.update_state(self_check_ok=False)
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("stays open", proc.stderr)
        self.assertEqual(self.types()[-2:], ["attempt-observation", "self-check"])
        self.assertIs(self.run_records()[-1]["passed"], False)
        self.assertEqual(self.classify()["directory"], "self-check-pending")
        self.assert_verified()
        self.update_state(self_check_ok=True)
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        records = self.run_records()
        self.assertEqual([r["type"] for r in records][-3:], ["self-check", "self-check", "run-close"])
        self.assertIs(records[-2]["late"], True)
        self.assertIs(records[-1]["done"], True)
        self.assertEqual(self.classify()["directory"], "committed")
        self.assert_verified()

    def test_recheck_refuses_an_observation_that_no_longer_classifies_committed(self):
        self.update_state(self_check_ok=False)
        self.run_hook("native-smoke")
        before = self.run_records()
        untrusted = json.loads(json.dumps(COMMITTED))
        untrusted["key"]["trusted"] = False
        self.update_state(self_check_ok=True, observations=dict(self.load_state()["observations"], native=untrusted))
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("recheck-refused", proc.stderr)
        self.assertEqual(self.run_records(), before)

    def test_unknown_classification_closes_the_run_for_the_owner(self):
        marker_only = json.loads(json.dumps(COMMITTED))
        marker_only["store"]["binding"] = "marker_only"
        self.update_state(observations=dict(self.load_state()["observations"], native=marker_only))
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("classified unknown", proc.stderr)
        records = self.run_records()
        self.assertEqual([r["type"] for r in records][-2:], ["attempt-observation", "run-close"])
        self.assertEqual((records[-1]["done"], records[-1]["classification"]), (False, "unknown"))
        self.assertEqual(self.load_state()["self_check_calls"], 0)
        self.assert_verified()
        self.assertNotIn("native_smoke_result=passed", proc.stdout)

    def test_failed_before_witness_is_retried_with_attempt_two(self):
        unpublished = observation(configured=KID, is_trusted=True, lsn=0x2000)
        self.update_state(observations=dict(self.load_state()["observations"], native=unpublished))
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn(f"run the evidence-retry step for run {self.run_id}", proc.stderr)
        self.assertEqual(self.types()[-1], "attempt-observation")
        self.assertEqual(self.run_records()[-1]["classification"], "failed-before-witness")
        self.assert_verified()
        # A rerun before the retry writes nothing and names the retry step.
        before = self.run_records()
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn(f"run the evidence-retry step for run {self.run_id}", proc.stderr)
        self.assertEqual(self.run_records(), before)
        for command in ("close-init", "observe"):
            self.assertNotEqual(self.run_hook("provenance-init", command, TASK, CLOSE_RUN, self.run_id).returncode, 0)
        self.assertEqual(self.run_records(), before)
        # The retry: a fresh observation and attempt-intent 2, anchored, that the deploy's fence check accepts.
        proc = self.run_hook("provenance-init", "retry", TASK, CLOSE_RUN, self.run_id)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        out = self.outputs(proc)
        self.assertEqual((out["retry_result"], out["attempt"], out["attempt_run_id"]), ("intent", "2", self.run_id))
        records = self.run_records()
        self.assertEqual([r["type"] for r in records][-3:], ["attempt-observation", "attempt-intent", "anchored"])
        self.assertEqual(records[-2]["attempt"], 2)
        self.assertEqual(out["attempt_intent_line_sha256"],
                         self.tool("require-open-attempt", "--dir", str(self.evidence), "--store", S1, "--key-id", KID,
                                   "--scope", json.dumps(SCOPE), "--identity",
                                   json.dumps(unpublished["database"]["identity"]),
                                   "--anchors", str(self.anchors_file()))[1]["intent_sha256"])
        self.assert_verified()
        proc = self.run_hook("provenance-init", "retry", TASK, CLOSE_RUN, self.run_id)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("no failed-before-witness observation", proc.stderr)
        # The deploy publishes; native-smoke closes attempt 2 as committed.
        self.update_state(observations=dict(self.load_state()["observations"], native=COMMITTED))
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        records = self.run_records()
        self.assertEqual([r["type"] for r in records][-3:], ["attempt-observation", "self-check", "run-close"])
        self.assertEqual(records[-3]["classification"], "committed")
        self.assertIs(records[-1]["done"], True)
        self.assert_verified()

    def test_retry_refusal_ends_the_run_and_rebegin_opens_a_new_one(self):
        unpublished = observation(configured=KID, is_trusted=True, lsn=0x2000)
        self.update_state(observations=dict(self.load_state()["observations"], native=unpublished))
        self.run_hook("native-smoke")
        busy = json.loads(json.dumps(unpublished))
        busy["database"]["in_flight"]["sessions"] = 1
        self.update_state(observations=dict(self.load_state()["observations"], native=busy))
        proc = self.run_hook("provenance-init", "retry", TASK, CLOSE_RUN, self.run_id)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("precondition:in-flight", proc.stderr)
        self.assertEqual(self.types()[-1], "begin-refused")
        self.assert_verified()
        # native-smoke has no attempt to close and stops without a write.
        before = self.all_records()
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(self.all_records(), before)
        # With native admission off again, the begin is taken again in a new run.
        self.write_env(native=False)
        self.update_state(phase="cutover", observations=dict(self.load_state()["observations"],
                          pre=observation(lsn=0x2100), post=observation(lsn=0x2200)))
        self.update_state(init_result={"result": "unchanged", "store_id": S1, "entries": 0})
        proc = self.run_hook("provenance-init", "rebegin", TASK, CLOSE_RUN, self.run_id)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual([r["type"] for r in self.run_records(count=2)],
                         ["run-open", "init-intent", "anchored", "init", "attempt-intent", "anchored"])
        self.assert_verified()

    def bot_posts(self):
        return len([c for c in self.load_state()["comments"] if c["user"] == BOT])

    def test_passing_self_check_without_run_close_is_closed_from_the_record(self):
        # The observation anchor is posted, the self-check anchor post fails: the runner stopped after a
        # durable passing self-check and before run-close.
        self.update_state(post_limit=self.bot_posts() + 1)
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(self.types()[-2:], ["attempt-observation", "self-check"])
        self.assertIs(self.run_records()[-1]["passed"], True)
        self.update_state(post_limit=None)
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(proc.stdout.splitlines()[-1], "native_smoke_result=passed")
        out = self.outputs(proc)
        self.assertEqual((out["attempt_run_id"], out["attempt_result"], out["self_check"]), (self.run_id, "committed", "passed"))
        records = self.run_records()
        self.assertEqual([r["type"] for r in records][-3:], ["attempt-observation", "self-check", "run-close"])
        self.assertIs(records[-1]["done"], True)
        self.assertEqual(self.load_state()["self_check_calls"], 1)
        self.assertEqual(self.classify()["directory"], "committed")
        self.assert_verified()

    def test_classified_observation_without_run_close_gets_its_run_close(self):
        marker_only = json.loads(json.dumps(COMMITTED))
        marker_only["store"]["binding"] = "marker_only"
        self.update_state(observations=dict(self.load_state()["observations"], native=marker_only),
                          post_limit=self.bot_posts() + 1)
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(self.types()[-2:], ["attempt-observation", "run-close"])
        # The run-close anchor post failed; drop that line to leave the runner's crash window.
        path = next(f for f in self.evidence.glob("*.jsonl") if f.name.startswith(self.run_id))
        path.write_text("\n".join(path.read_text().splitlines()[:-1]) + "\n")
        self.assertEqual(self.types()[-1], "attempt-observation")
        self.update_state(post_limit=None)
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("missing run-close (done: false) is now written", proc.stderr)
        records = self.run_records()
        self.assertEqual([r["type"] for r in records][-2:], ["attempt-observation", "run-close"])
        self.assertEqual((records[-1]["done"], records[-1]["classification"]), (False, "unknown"))
        self.assert_verified()
        self.assertNotIn("native_smoke_result=passed", proc.stdout)

    def test_acts_only_on_a_run_of_its_task_and_close_run(self):
        before, posts = self.all_records(), self.bot_posts()
        for env, text in (({"OTERYN_CUTOVER_TASK_ID": "OTERYN-20261013-other-cutover"}, "records cutover"),
                          ({"OTERYN_CUTOVER_CLOSE_RUN_ID": "7000000002"}, "records cutover"),
                          ({"OTERYN_CUTOVER_TASK_ID": None}, "task id is malformed"),
                          ({"OTERYN_CUTOVER_CLOSE_RUN_ID": None}, "close run id is malformed")):
            with self.subTest(env=env):
                proc = self.run_hook("native-smoke", **env)
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn(text, proc.stderr)
                self.assertNotIn("native_smoke_result=passed", proc.stdout)
        self.assertEqual(self.all_records(), before)
        self.assertEqual(self.bot_posts(), posts)
        # A committed and closed attempt of this cutover still does not pass a smoke for another one.
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        proc = self.run_hook("native-smoke", OTERYN_CUTOVER_CLOSE_RUN_ID="7000000002")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("records cutover", proc.stderr)

    def test_late_observe_command_closes_an_unclosed_attempt(self):
        proc = self.run_hook("provenance-init", "observe", TASK, CLOSE_RUN, self.run_id)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        records = self.run_records()
        self.assertEqual([r["type"] for r in records][-3:], ["attempt-observation", "self-check", "run-close"])
        self.assertIs(records[-3]["late"], True)
        self.assert_verified()
        proc = self.run_hook("provenance-init", "observe", TASK, CLOSE_RUN, self.run_id)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("no attempt waiting", proc.stderr)

    def test_fails_when_the_gateway_refuses_native_login(self):
        self.http.gateway_on = False
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("native login smoke failed", proc.stderr)

    def test_native_login_must_be_admitted_end_to_end(self):
        cases = {
            "actor-mfa": "has MFA confirmed", "oauth-denied": "returned no code", "token-error": "token exchange",
            "bearer-kept": "issuance must revoke it", "ticket-expired": "already expired",
            "conflict": "status 409, code CHARACTER_CONFLICT", "route-unpublished": "status 404, code ROUTE_NOT_FOUND",
            "replay-differs": "different admission", "bad-grant": "without the native admission contract body",
        }
        for mode, text in cases.items():
            with self.subTest(mode=mode):
                self.http.native_mode = mode
                self.http.native = {}
                self.http.signed_in = None
                proc = self.run_hook("native-smoke")
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn(text, proc.stderr)
                self.assertIn("native login smoke failed", proc.stderr)
                self.assertNotIn("native_smoke_result=passed", proc.stdout)
                self.assert_no_secret(proc)

    def test_owner_waiver_for_this_task_checks_only_the_unknown_ticket(self):
        # The owner waived the successful native login for this task: with no actor set (unset or empty, as the
        # workflow passes them) and the waiver equal to the task id, only the issuer's refusal of an unknown ticket
        # is checked, and the waiver is in the output.
        for value in (None, ""):
            with self.subTest(value=value):
                unset = {name: value for name in ("OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL", "OTERYN_CUTOVER_NATIVE_SMOKE_PASSWORD",
                                                   "OTERYN_CUTOVER_NATIVE_SMOKE_CHARACTER_ID")}
                unset["OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER"] = TASK
                (self.stub / "docker.log").unlink(missing_ok=True)
                proc = self.run_hook("native-smoke", **unset)
                self.assertEqual(proc.returncode, 0, proc.stderr)
                self.assertEqual(proc.stdout.splitlines()[-1], "native_smoke_result=passed")
                out = self.outputs(proc)
                self.assertEqual({k: out[k] for k in out if k.startswith("native_smoke_") and k != "native_smoke_result"},
                                 {"native_smoke_unknown_ticket": "refused", "native_smoke_login": "owner-waived"})
                self.assertEqual(self.http.native, {})
                self.assertEqual(self.http.smoke_posts, [])
                calls = self.log("docker.log")
                self.assertFalse([c for c in calls if "game-auth:oauth-client:ensure" in c.get("compose", [])])
                self.assertEqual([c["run"] for c in calls if "run" in c],
                                 [["run", "--rm", "--network", "host", "-i", "python:3.12-alpine"]])
                self.assertEqual([r["type"] for r in self.run_records()][-3:], ["attempt-observation", "self-check", "run-close"])
                self.assert_verified()
                self.assert_no_secret(proc)
        # The negative check is still real: a Gateway that does not reach the issuer fails the waived smoke.
        self.http.gateway_on = False
        proc = self.run_hook("native-smoke", **unset)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("native login smoke failed", proc.stderr)

    def test_a_busy_route_is_retried_with_the_same_attempt(self):
        self.http.native_mode = "busy-once"
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        logins = self.http.native["logins"]
        self.assertEqual(len(logins), 3)
        self.assertEqual(len(set(logins)), 1)

    def test_refuses_a_partial_native_smoke_actor_before_any_evidence_write(self):
        before, posts = self.all_records(), len(self.posts())
        cases = (
            ({"OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL": None}, "native smoke actor is not configured"),
            ({"OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL": "not-an-address"}, "native smoke actor is not configured"),
            ({"OTERYN_CUTOVER_NATIVE_SMOKE_PASSWORD": ""}, "native smoke actor is not configured"),
            ({"OTERYN_CUTOVER_NATIVE_SMOKE_CHARACTER_ID": None}, "lower-case UUIDv7"),
            ({"OTERYN_CUTOVER_NATIVE_SMOKE_CHARACTER_ID": NATIVE_CHARACTER.upper()}, "lower-case UUIDv7"),
        )
        for overrides, text in cases:
            with self.subTest(overrides=overrides):
                proc = self.run_hook("native-smoke", **overrides)
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn(text, proc.stderr)
                self.assertEqual((self.all_records(), len(self.posts())), (before, posts))
                self.assertFalse(self.http.native)
                self.assert_no_secret(proc)
        self.update_state(client_id=None)
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("printed no client id", proc.stderr)
        self.assertFalse(self.http.native)

    def test_without_an_actor_the_waiver_must_name_this_task_before_any_evidence_write(self):
        # No default waiver: with no actor, only the owner's waiver for this very task lets the smoke run.
        before, posts = self.all_records(), len(self.posts())
        no_actor = {"OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL": "", "OTERYN_CUTOVER_NATIVE_SMOKE_PASSWORD": "",
                    "OTERYN_CUTOVER_NATIVE_SMOKE_CHARACTER_ID": ""}
        cases = (
            (dict(no_actor), "native smoke actor is not configured"),
            (dict(no_actor, OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER=""), "native smoke actor is not configured"),
            (dict(no_actor, OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER="OTERYN-20261012-other-task"), "must equal this cutover's task id"),
            (dict(no_actor, OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER=TASK + " "), "must equal this cutover's task id"),
            (dict(no_actor, OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER="yes"), "must equal this cutover's task id"),
            ({"OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER": TASK}, "not both"),
            ({"OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER": TASK, "OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL": ""}, "not both"),
        )
        for overrides, text in cases:
            with self.subTest(overrides=overrides):
                (self.stub / "docker.log").unlink(missing_ok=True)
                proc = self.run_hook("native-smoke", **overrides)
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn(text, proc.stderr)
                self.assertNotIn("native_smoke_login=", proc.stdout)
                self.assertEqual((self.all_records(), len(self.posts())), (before, posts))
                self.assertFalse(self.http.native)
                self.assertEqual(self.log("docker.log"), [])
                self.assert_no_secret(proc)

    def test_fails_when_admission_is_off_or_unpaired(self):
        self.update_state(container_env=dict(self.load_state()["container_env"], **{"cid-gateway": ["GATEWAY_NATIVE_LOGIN_ENABLED=false"]}))
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assert_no_secret(proc)
        self.write_env(native=False)
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(self.types()[-1], "anchored")

    def test_fails_without_a_recorded_attempt_or_anchor_issue(self):
        proc = self.run_hook("native-smoke", OTERYN_CUTOVER_ANCHOR_ISSUE=None)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("OTERYN_CUTOVER_ANCHOR_ISSUE", proc.stderr)
        for path in self.evidence.iterdir():
            path.unlink()
        # Anchors that name a missing run file fail verify before anything is read.
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("has no file", proc.stderr)
        # An empty directory on a fresh anchor issue: step 4 recorded nothing.
        self.update_state(comments=[c for c in self.load_state()["comments"] if c["user"] != BOT])
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("has not recorded an attempt", proc.stderr)

    def test_final_trust_check_fails_without_a_published_profile(self):
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        unpublished = observation(configured=KID, is_trusted=True, lsn=0x2000)
        untrusted = json.loads(json.dumps(COMMITTED))
        untrusted["key"]["trusted"] = False
        for obs in (unpublished, untrusted):
            with self.subTest(trusted=obs["key"]["trusted"]):
                self.update_state(observations=dict(self.load_state()["observations"], native=obs))
                proc = self.run_hook("native-smoke")
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn("first publication", proc.stderr)
                self.assertNotIn("native_smoke_result=passed", proc.stdout)


class HookContractTest(unittest.TestCase):
    def test_hooks_are_executable_in_git(self):
        listing = subprocess.run(["git", "-C", str(ROOT), "ls-files", "-s", "--", str(HOOKS)],
                                 capture_output=True, text=True, check=True).stdout
        modes = {Path(line.split("\t")[1]).name: line.split()[0] for line in listing.splitlines()}
        for name in HOOK_NAMES:
            self.assertTrue(os.access(HOOKS / f"{name}.sh", os.X_OK), name)
            if modes:
                self.assertEqual(modes.get(f"{name}.sh"), "100755", name)
        if modes:
            self.assertEqual(modes.get("common.sh"), "100644")
            self.assertEqual(modes.get("evidence.sh"), "100644")

    def test_hooks_never_trace_or_print_the_environment_file(self):
        for path in HOOKS.glob("*.sh"):
            text = path.read_text()
            self.assertNotRegex(text, r"set -[a-z]*x|set -o xtrace", path.name)
            self.assertNotRegex(text, r"cat\s+[^|]*ENV_FILE", path.name)
            if path.name not in ("common.sh", "evidence.sh"):
                self.assertTrue(text.startswith("#!/usr/bin/env bash\n"), path.name)
                self.assertIn("set -euo pipefail", text)

    def test_platform_paths_the_smokes_use_are_declared_routes(self):
        # Each Platform path the hooks request, against the route file that declares it: routes/api.php is served
        # under the default /api prefix, routes/web.php and its modules at the root, Passport under /oauth.
        web = (ROOT / "routes" / "web.php").read_text()
        api = (ROOT / "routes" / "api.php").read_text()
        marketplace = (ROOT / "routes" / "modules" / "marketplace.php").read_text()
        app = (ROOT / "bootstrap" / "app.php").read_text()
        self.assertIn("api: __DIR__.'/../routes/api.php'", app)
        self.assertNotIn("apiPrefix", app)
        self.assertFalse((ROOT / "config" / "passport.php").exists(), "a Passport path override would move /oauth")
        self.assertIn("Route::post('/v1/game-auth/tickets'", api)
        for route in ("Route::get('/register'", "Route::post('/register'", "Route::get('/login'", "Route::post('/login'",
                      "Route::post('/logout'", "Route::get('/account'", "Route::view('/admin'", "Route::get('/news'"):
            self.assertIn(route, web)
        self.assertIn("Route::get('/bazaar'", marketplace)
        self.assertIn("prefix('account/bazaar')", marketplace)
        native = (HOOKS / "native-smoke.sh").read_text()
        self.assertEqual(native.count("'/api/v1/game-auth/tickets'"), 2)
        self.assertNotRegex(native, r"'/v1/game-auth/")
        for path in ("'/oauth/authorize?'", "'/oauth/authorize'", "'/oauth/token'", "'/v1/login'", "'/login?locale=en'",
                     "'/account'", "'/logout'"):
            self.assertIn(path, native)

    def test_evidence_doc_names_each_stable_path(self):
        doc = DOC.read_text()
        for name in HOOK_NAMES:
            self.assertIn(f"{STABLE_DIR}/{name}.sh", doc)
        for var in ("OTERYN_STAGING_CUTOVER_PROVENANCE_INIT_COMMAND", "OTERYN_STAGING_CUTOVER_WEB_SMOKE_COMMAND",
                    "OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_COMMAND", "OTERYN_CUTOVER_ANCHOR_ISSUE"):
            self.assertIn(var, doc)
        for text in ("OTERYN_STAGING_CUTOVER_ANCHOR_ISSUE", "evidence-close-init", "evidence-observe", "evidence-retry",
                     "evidence-rebegin", "evidence_run_id",
                     "provenance-init.sh preflight <task_id> <close_run_id>"):
            self.assertIn(text, doc)
        # native-reenable refuses once REENABLE_STARTED is recorded, so no recovery may send the operator back to it.
        self.assertNotIn("run `native-reenable` again", doc)
        self.assertIn("`cutover-postgres.sh` never runs it again", doc)

    def test_workflow_maps_the_anchor_issue_and_scopes_issues_write(self):
        text = (ROOT / ".github" / "workflows" / "deploy-synology-staging.yml").read_text()
        head, jobs = text.split("\njobs:\n", 1)
        self.assertNotIn("issues:", head)
        self.assertEqual(text.count("\n      issues: write\n"), 1)
        self.assertIn("    permissions:\n      contents: read\n      packages: read\n      actions: read\n      issues: write\n", jobs)
        steps = jobs.split("      - name: ")
        with_token = [step.split("\n", 1)[0] for step in steps if "GH_TOKEN: ${{ github.token }}" in step]
        self.assertEqual(with_token, ["Run cutover evidence step", "Run cutover step"])
        for name in with_token:
            step = next(step for step in steps if step.startswith(name + "\n"))
            self.assertIn("OTERYN_CUTOVER_ANCHOR_ISSUE: ${{ vars.OTERYN_STAGING_CUTOVER_ANCHOR_ISSUE }}", step)
            self.assertNotIn("${{", step.split("        run: |\n", 1)[1])
        evidence = next(step for step in steps if step.startswith("Run cutover evidence step\n"))
        self.assertIn("if: inputs.action == 'cutover' && startsWith(inputs.cutover_step, 'evidence-')", evidence)
        self.assertIn('args=("$command" "$OTERYN_CUTOVER_TASK_ID" "$OTERYN_CUTOVER_CLOSE_RUN_ID")', evidence)
        self.assertIn('[[ -z "$OTERYN_CUTOVER_EVIDENCE_RUN_ID" ]] || args+=("$OTERYN_CUTOVER_EVIDENCE_RUN_ID")', evidence)
        self.assertIn('"$hook" "${args[@]}"', evidence)
        for choice in ("evidence-close-init", "evidence-observe", "evidence-retry", "evidence-rebegin"):
            self.assertIn(f"          - {choice}\n", text)
        self.assertIn("if: inputs.action == 'cutover' && !startsWith(inputs.cutover_step, 'evidence-')", text)
        # native-reenable's hook prerequisites are checked before cutover-postgres.sh records its marker.
        cutover = next(step for step in steps if step.startswith("Run cutover step\n")).split("        run: |\n", 1)[1]
        preflight = cutover.index('"$hook" preflight "$OTERYN_CUTOVER_TASK_ID" "$OTERYN_CUTOVER_CLOSE_RUN_ID"')
        self.assertLess(preflight, cutover.index('bash deploy/synology/scripts/cutover-postgres.sh "$CUTOVER_STEP"'))
        self.assertIn('if [[ "$CUTOVER_STEP" == native-reenable && "$OTERYN_CUTOVER_DRY_RUN" != 1 ]]; then', cutover)
        # The native smoke actor: names from variables, the password secret only for the native-smoke step.
        step = next(step for step in steps if step.startswith("Run cutover step\n"))
        for line in ("OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL: ${{ vars.OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_EMAIL }}",
                     "OTERYN_CUTOVER_NATIVE_SMOKE_CHARACTER_ID: ${{ vars.OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_CHARACTER_ID }}",
                     "OTERYN_CUTOVER_NATIVE_SMOKE_PASSWORD: ${{ inputs.cutover_step == 'native-smoke' && "
                     "secrets.OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_PASSWORD || '' }}",
                     "OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER: ${{ inputs.cutover_step == 'native-smoke' && "
                     "inputs.native_smoke_waiver || '' }}"):
            self.assertIn(line, step)
        # The waiver is a per-dispatch owner decision, never a repository setting.
        self.assertIn("      native_smoke_waiver:\n", text)
        self.assertNotIn("vars.OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_WAIVER", text)
        self.assertEqual(text.count("secrets.OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_PASSWORD"), 1)

    def test_contract_workflow_runs_the_cutover_suites(self):
        text = (ROOT / ".github" / "workflows" / "synology-rollback-contract.yml").read_text()
        for name in ("postgres_contract", "attempt_evidence", "cutover_postgres", "cutover_hooks"):
            self.assertIn(f"python3 tests/ci/test_synology_{name}.py", text)
            self.assertEqual(text.count(f"      - tests/ci/test_synology_{name}.py\n"), 2)
        self.assertEqual(text.count("      - deploy/synology/**\n"), 2)


if __name__ == "__main__":
    unittest.main()
