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

import http.server
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
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
    if rest[:3] == ["exec", "-T", "platform"]:
        command = rest[rest.index("artisan") + 1]
        if command == "game-auth:native-evidence:observe-trust":
            key = next((a.split("=", 1)[1] for a in rest if a.startswith("--key-id=")), None)
            phase = state["phase"] if state["phase"] == "native" else ("post" if state["init_done"] else "pre")
            state["observe_calls"] += 1
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
                return self.reply(404, "text/html", b"not found")

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
                if self.path == "/v1/login" and payload.get("protocol_version") == 2 and owner.gateway_on:
                    status, body = 401, {"error": {"code": "NATIVE_LOGIN_AUTHENTICATION_REQUIRED"}}
                else:
                    status, body = 400, {"error": "invalid_request"}
                raw = json.dumps(body).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(raw)))
                self.end_headers()
                self.wfile.write(raw)

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
            "GITHUB_REPOSITORY": "Oteryn/Oteryn-Platform",
            "GH_TOKEN": GH_TOKEN,
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
        for value in [*SECRETS.values(), GH_TOKEN, PUBLIC_KEY, COOKIE, CSRF]:
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
                     ["init", TASK, "0"], ["init", TASK, "12x"], ["init", TASK, CLOSE_RUN, "extra"]):
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


class WebSmokeTest(HookHarness):
    def test_passes_on_a_healthy_platform(self):
        proc = self.run_hook("web-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(proc.stdout.splitlines()[-1], "web_smoke_result=passed")
        runs = [entry["run"] for entry in self.log("docker.log") if "run" in entry]
        self.assertEqual(runs, [["run", "--rm", "--network", "host", "-i", "python:3.12-alpine"]])
        self.assertEqual(list(self.scratch.iterdir()), [])
        self.assert_no_secret(proc)

    def test_fails_closed(self):
        for mode in ("bad-action", "news-error"):
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
        # A rerun leaves the closed attempt as it is.
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.run_records(), records)

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

    def test_failed_before_witness_leaves_the_run_open_for_the_owner(self):
        unpublished = observation(configured=KID, is_trusted=True, lsn=0x2000)
        self.update_state(observations=dict(self.load_state()["observations"], native=unpublished))
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("failed before the witness", proc.stderr)
        self.assertEqual(self.types()[-1], "attempt-observation")
        self.assertEqual(self.run_records()[-1]["classification"], "failed-before-witness")
        self.assert_verified()

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

    def test_evidence_doc_names_each_stable_path(self):
        doc = DOC.read_text()
        for name in HOOK_NAMES:
            self.assertIn(f"{STABLE_DIR}/{name}.sh", doc)
        for var in ("OTERYN_STAGING_CUTOVER_PROVENANCE_INIT_COMMAND", "OTERYN_STAGING_CUTOVER_WEB_SMOKE_COMMAND",
                    "OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_COMMAND", "OTERYN_CUTOVER_ANCHOR_ISSUE"):
            self.assertIn(var, doc)
        for text in ("OTERYN_STAGING_CUTOVER_ANCHOR_ISSUE", "evidence-close-init", "evidence-observe", "evidence_run_id",
                     "provenance-init.sh preflight <task_id> <close_run_id>"):
            self.assertIn(text, doc)
        # native-reenable refuses once REENABLE_STARTED is recorded, so no recovery may send the operator back to it.
        self.assertNotIn("run `native-reenable` again", doc)
        self.assertIn("it has no resume path", doc)

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
        self.assertIn('"$hook" "$command" "$OTERYN_CUTOVER_TASK_ID"', evidence)
        for choice in ("evidence-close-init", "evidence-observe"):
            self.assertIn(f"          - {choice}\n", text)
        self.assertIn("if: inputs.action == 'cutover' && !startsWith(inputs.cutover_step, 'evidence-')", text)
        # native-reenable's hook prerequisites are checked before cutover-postgres.sh records its marker.
        cutover = next(step for step in steps if step.startswith("Run cutover step\n")).split("        run: |\n", 1)[1]
        preflight = cutover.index('"$hook" preflight "$OTERYN_CUTOVER_TASK_ID" "$OTERYN_CUTOVER_CLOSE_RUN_ID"')
        self.assertLess(preflight, cutover.index('bash deploy/synology/scripts/cutover-postgres.sh "$CUTOVER_STEP"'))
        self.assertIn('if [[ "$CUTOVER_STEP" == native-reenable && "$OTERYN_CUTOVER_DRY_RUN" != 1 ]]; then', cutover)

    def test_contract_workflow_runs_the_cutover_suites(self):
        text = (ROOT / ".github" / "workflows" / "synology-rollback-contract.yml").read_text()
        for name in ("postgres_contract", "attempt_evidence", "cutover_postgres", "cutover_hooks"):
            self.assertIn(f"python3 tests/ci/test_synology_{name}.py", text)
            self.assertEqual(text.count(f"      - tests/ci/test_synology_{name}.py\n"), 2)
        self.assertEqual(text.count("      - deploy/synology/**\n"), 2)


if __name__ == "__main__":
    unittest.main()
