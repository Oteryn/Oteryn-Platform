#!/usr/bin/env python3
"""PostgreSQL cutover hooks (deploy/synology/scripts/cutover-hooks/, plan P6 steps 3 to 5).

Runs the real hooks with fake docker, gh and key-tool executables, a local HTTP server on loopback in place of
the Platform and Gateway ports, and the real attempt evidence tool (attempt_evidence.py) on a temporary
directory; only its host-location check and the evidence group id are faked. Nothing touches Docker, GitHub
or the NAS. The cases prove the P6a step-4 order (no evidence write before the anchor issue is proven usable,
no init before an anchored init-intent, an anchored attempt-intent that require-open-attempt accepts), fail-closed
refusals, the derived runtime environment, the smoke checks, and that no secret reaches the output.

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


def observation(store=S1, binding="match", floor=None, profile=None, lsn=0x1000, configured=None, trusted=False,
                result="observed"):
    return {
        "result": result, "fence_required": True,
        "store": {"marker": store, "row": store, "binding": binding},
        "scope": SCOPE,
        "floor": {"value": floor, "temp_entries": 0},
        "profile": profile or {"present": False},
        "key": {"requested": None, "configured_key_id": configured, "in_history": False, "trusted": trusted},
        "database": {
            "identity": {"system_identifier": "7001", "timeline_id": 1, "postmaster_start_time": "t0",
                         "database_oid": 16384, "relfilenodes": [1, 2, 3]},
            "progress": {"wal_flush_lsn": f"0/{lsn:X}", "xmax": "100"},
            "in_flight": {"sessions": 0, "prepared": 0},
        },
    }


COMMITTED = observation(floor=1, configured=KID, trusted=True, lsn=0x2000,
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
            obs = state["observations"][phase]
            key = key or obs["key"]["configured_key_id"]
            if key is None:
                print("Native signing trust observation failed: Supply a valid --key-id.", file=sys.stderr)
                sys.exit(1)
            obs["key"]["requested"] = key
            print(json.dumps(obs))
            sys.exit(0 if obs["result"] == "observed" else 1)
        if command == "game-auth:native-evidence:init-provenance":
            state["init_calls"] += 1
            result = state["init_result"]
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
    print("\n".join(state["container_env"][args[-1]]))
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
    if state["post_fail"]:
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
            "phase": "cutover", "init_done": False, "init_calls": 0,
            "init_result": {"result": "created", "store_id": S1, "entries": 0},
            "observations": {
                "pre": observation(store=None, binding="none", lsn=0x1000),
                "post": observation(lsn=0x1100),
                "native": COMMITTED,
            },
            "container_env": {
                "cid-platform": ["APP_ENV=staging", "GAME_AUTH_NATIVE_ADMISSION_ENABLED=true",
                                 f"APP_KEY={SECRETS['APP_KEY']}"],
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

    def run_records(self):
        files = sorted(self.evidence.glob("*.jsonl"))
        self.assertEqual(len(files), 1, files)
        return [json.loads(line) for line in files[0].read_text().splitlines()]

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
        self.write_env(native=True)
        self.state["phase"] = "native"
        self.save_state()

    def test_passes_with_native_admission_on_and_trust_published(self):
        proc = self.run_hook("native-smoke")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(proc.stdout.splitlines()[-1], "native_smoke_result=passed")
        self.assert_compose_used_derived_env()
        self.assert_no_secret(proc)

    def test_fails_when_the_gateway_refuses_native_login(self):
        self.http.gateway_on = False
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("native login smoke failed", proc.stderr)

    def test_fails_when_admission_is_off_or_unpaired(self):
        self.state["container_env"]["cid-gateway"] = ["GATEWAY_NATIVE_LOGIN_ENABLED=false"]
        self.save_state()
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)
        self.assert_no_secret(proc)
        self.write_env(native=False)
        proc = self.run_hook("native-smoke")
        self.assertNotEqual(proc.returncode, 0)

    def test_fails_without_a_published_profile(self):
        unpublished = observation(configured=KID, trusted=True, lsn=0x2000)
        for obs in (unpublished, dict(COMMITTED, key=dict(COMMITTED["key"], trusted=False)),
                    dict(COMMITTED, store=dict(COMMITTED["store"], binding="marker_only"))):
            with self.subTest(obs=obs["store"]["binding"]):
                self.state["observations"]["native"] = obs
                self.save_state()
                proc = self.run_hook("native-smoke")
                self.assertNotEqual(proc.returncode, 0)
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

    def test_hooks_never_trace_or_print_the_environment_file(self):
        for path in HOOKS.glob("*.sh"):
            text = path.read_text()
            self.assertNotRegex(text, r"set -[a-z]*x|set -o xtrace", path.name)
            self.assertNotRegex(text, r"cat\s+[^|]*ENV_FILE", path.name)
            if path.name != "common.sh":
                self.assertTrue(text.startswith("#!/usr/bin/env bash\n"), path.name)
                self.assertIn("set -euo pipefail", text)

    def test_evidence_doc_names_each_stable_path(self):
        doc = DOC.read_text()
        for name in HOOK_NAMES:
            self.assertIn(f"{STABLE_DIR}/{name}.sh", doc)
        for var in ("OTERYN_STAGING_CUTOVER_PROVENANCE_INIT_COMMAND", "OTERYN_STAGING_CUTOVER_WEB_SMOKE_COMMAND",
                    "OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_COMMAND", "OTERYN_CUTOVER_ANCHOR_ISSUE"):
            self.assertIn(var, doc)


if __name__ == "__main__":
    unittest.main()
