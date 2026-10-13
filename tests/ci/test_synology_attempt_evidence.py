#!/usr/bin/env python3
"""Attempt evidence tool contract (docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md D3 to D6, D8).

Behaviour tests of deploy/synology/scripts/attempt_evidence.py against temporary directories and fixture
`docker inspect` / `mountinfo` data. No Docker, no network, no database.
"""

from __future__ import annotations

import builtins
import contextlib
import importlib.util
import io
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
TOOL = ROOT / "deploy/synology/scripts/attempt_evidence.py"

spec = importlib.util.spec_from_file_location("attempt_evidence", TOOL)
ae = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ae)

S1 = "a" * 64
S2 = "b" * 64
K = "kid-1"
SCOPE = {"issuer": "iss", "profile": "prof", "key_purpose": "fresh"}
TARGET = "/var/lib/oteryn-attempt-evidence"
SOURCE = "/volume1/oteryn-attempt-evidence"
BOT = {"id": 41898282, "login": "github-actions[bot]"}


def obs(store=S1, binding="match", floor=None, profile=None, lsn=0x1000, xmax=100, pmst="t0", sysid="7001", tl=1,
        oid=16384, rel=(1, 2, 3), in_flight=0, fence=True, in_history=False, configured=K, trusted=False,
        temp=0, db=True):
    o = {
        "result": "ok", "fence_required": fence,
        "store": {"marker": store, "row": store, "binding": binding},
        "scope": SCOPE,
        "floor": {"value": floor, "temp_entries": temp},
        "profile": profile if profile else {"present": False},
        "key": {"requested": K, "configured_key_id": configured, "in_history": in_history, "trusted": trusted},
        "database": None,
    }
    if db:
        o["database"] = {
            "identity": {"system_identifier": sysid, "timeline_id": tl, "postmaster_start_time": pmst,
                         "database_oid": oid, "relfilenodes": list(rel)},
            "progress": {"wal_flush_lsn": f"0/{lsn:X}", "xmax": str(xmax)},
            "in_flight": {"sessions": in_flight, "prepared": 0},
        }
    return o


def none_obs(**kw):
    return obs(store=None, binding="none", **kw) if "db" not in kw else obs(store=None, binding="none", **kw)


def profile(rev=1, revoked=False):
    return {"present": True, "id": 1, "profile_version": 1, "issuer_revision": rev, "revoked": revoked}


def committed_obs(**kw):
    return obs(floor=1, profile=profile(1), trusted=True, **kw)


class Driver:
    """Runs the CLI in-process and plays the part of P6 (anchor comments)."""

    def __init__(self, directory):
        self.dir = directory
        self.comments = []
        self.locked = True
        self.gid = os.getgid()
        self.next_id = 1000

    def cli(self, *argv):
        buf = io.StringIO()
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            code = ae.main([str(x) for x in argv], out=buf)
        text = buf.getvalue().strip()
        self.last_stderr = err.getvalue()
        if not text and self.last_stderr:
            return code, {"ok": False, "error": self.last_stderr.split(": ")[2], "stderr": self.last_stderr}
        return code, (json.loads(text) if text else None)

    def anchors_file(self, extra=None):
        path = os.path.join(os.path.dirname(self.dir), "anchors.json")
        doc = {"locked": self.locked, "comments": self.comments + (extra or [])}
        with open(path, "w") as handle:
            json.dump(doc, handle)
        return path

    def post(self, anchor, author=BOT):
        if anchor is None:
            return None
        self.next_id += 1
        self.comments.append({"id": self.next_id, "author": author, "anchor": anchor})
        return self.next_id

    def post_lines(self, out):
        for line in out.get("lines") or [out]:
            if line.get("anchor"):
                self.post(line["anchor"])

    def open(self, pre, anchors=True):
        args = ["open", "--dir", self.dir, "--pre-init", json.dumps(pre), "--location", json.dumps({"ok": True}),
                "--group-gid", self.gid]
        if anchors:
            args += ["--anchors", self.anchors_file()]
        code, out = self.cli(*args)
        if out and out.get("lines"):
            self.post_lines(out)
        return code, out

    def append(self, typ, run, data=None, anchor=True, extra=()):
        args = ["append", typ, "--dir", self.dir, "--run", run, "--data", json.dumps(data or {}), *extra]
        code, out = self.cli(*args)
        if anchor and out and out.get("anchor"):
            self.post(out["anchor"])
        return code, out

    def anchored(self, run, post=True):
        cid = self.next_id + 1
        if post:
            # the comment of the previous line was already posted by append(); find it
            cid = self.comments[-1]["id"]
        return self.append("anchored", run, {"anchor_comment_id": cid})

    def start(self, pre=None):
        pre = pre or none_obs()
        code, out = self.open(pre)
        assert code == 0, out
        return out["run_id"]

    def do_init(self, run, store=S1, observation=None, result="created"):
        assert self.append("init-intent", run)[0] == 0
        assert self.anchored(run)[0] == 0
        return self.append("init", run, {"result": result, "store_id": store, "observation": observation or obs(store=store)})

    def do_intent(self, run, observation=None, n=None, **kw):
        data = {"store_id": S1, "scope": SCOPE, "key_id": K, "kid_record_count": 1,
                "observation": observation or obs()}
        data.update(kw)
        return self.append("attempt-intent", run, data)

    def full_to_intent(self, anchor_intent=True):
        run = self.start()
        assert self.do_init(run)[0] == 0
        code, out = self.do_intent(run)
        assert code == 0, out
        if anchor_intent:
            assert self.anchored(run)[0] == 0
        return run

    def observe(self, run, o1, o2=None, **kw):
        data = {"observation_1": o1, "observation_2": o2 or o1}
        data.update(kw)
        return self.append("attempt-observation", run, data)

    def classify(self):
        return self.cli("classify", "--dir", self.dir, "--anchors", self.anchors_file())

    def verdict(self):
        return self.classify()[1]["directory"]

    def lines(self, run):
        path = os.path.join(self.dir, run + ".jsonl")
        with open(path, "rb") as handle:
            return handle.read().split(b"\n")[:-1]


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="evidence-test-")
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.dir = os.path.join(self.tmp, "ev")
        os.mkdir(self.dir)
        os.chmod(self.dir, 0o2750)
        self.d = Driver(self.dir)

    def run_files(self):
        return sorted(n for n in os.listdir(self.dir) if n.endswith(".jsonl"))


class LocationTest(Base):
    def fixtures(self):
        self.mountinfo = (
            f"100 50 0:50 /oteryn-attempt-evidence {TARGET} rw - btrfs /dev/vg1 rw\n"
            "101 50 0:50 /docker/oteryn/state /var/lib/oteryn/state rw - btrfs /dev/vg1 rw\n"
        )
        self.runner = {"Id": "runner1", "Mounts": [
            {"Type": "bind", "Source": SOURCE, "Destination": TARGET, "RW": True},
            {"Type": "bind", "Source": "/volume1/docker/oteryn/state", "Destination": "/var/lib/oteryn/state", "RW": True},
            {"Type": "bind", "Source": "/var/run/docker.sock", "Destination": "/var/run/docker.sock", "RW": True},
        ]}
        self.project = [
            {"Id": "pg", "Name": "/postgres", "Config": {"Labels": {"com.docker.compose.service": "postgres"}},
             "Mounts": [{"Type": "volume", "Source": "/volume1/@docker/volumes/postgres_data/_data", "Destination": "/var/lib/postgresql/data", "RW": True}]},
            {"Id": "pl", "Name": "/platform", "Config": {"Labels": {"com.docker.compose.service": "platform"}},
             "Mounts": [{"Type": "bind", "Source": SOURCE, "Destination": TARGET, "RW": False}]},
        ]
        self.info = {"DockerRootDir": "/volume1/@docker"}

    def check(self, **overrides):
        return self.d.cli(*self.prepare(**overrides))

    def prepare(self, **overrides):
        self.fixtures()
        for key, value in overrides.items():
            setattr(self, key, value)
        paths = {}
        for name, doc in (("runner", self.runner), ("project", self.project), ("info", self.info)):
            paths[name] = os.path.join(self.tmp, name + ".json")
            with open(paths[name], "w") as handle:
                json.dump(doc, handle)
        paths["mountinfo"] = os.path.join(self.tmp, "mountinfo")
        with open(paths["mountinfo"], "w") as handle:
            handle.write(self.mountinfo)
        st = os.stat(self.dir)
        args = ["check-location", "--dir", self.dir, "--target", TARGET, "--runner-inspect", paths["runner"],
                "--project-inspect", paths["project"], "--docker-info", paths["info"], "--mountinfo", paths["mountinfo"],
                "--owner-uid", getattr(self, "owner_uid", st.st_uid), "--group-gid", st.st_gid, "--dir-mode", "2750"]
        return args

    def assertRefused(self, prefix, **kw):
        code, out = self.check(**kw)
        self.assertEqual(code, 1, out)
        self.assertTrue(any(e.startswith(prefix) for e in out["errors"]), out)

    def test_passes_with_readonly_platform_mount(self):
        code, out = self.check()
        self.assertEqual(code, 0, out)
        self.assertEqual(out["canonical_source"], SOURCE)
        self.assertEqual(out["share"], SOURCE)

    def test_named_volume_and_docker_root(self):
        self.fixtures()
        runner = json.loads(json.dumps(self.runner))
        runner["Mounts"][0]["Type"] = "volume"
        self.assertRefused("mount-not-bind", runner=runner)
        self.assertRefused("docker-root", info={"DockerRootDir": "/volume1"})

    def test_overlap_inside_and_containing(self):
        self.fixtures()
        for src in (SOURCE, SOURCE + "/sub", "/volume1"):
            project = json.loads(json.dumps(self.project))
            project[0]["Mounts"][0]["Source"] = src
            self.assertRefused("overlap:", project=project)

    def test_same_share_as_another_mount(self):
        self.fixtures()
        project = json.loads(json.dumps(self.project))
        project[0]["Mounts"][0]["Source"] = "/volume1/oteryn-attempt-evidence-x/data"
        # different share name passes; the same first two components refuse via overlap, not share
        code, _ = self.check(project=project)
        self.assertEqual(code, 0)

    def test_platform_mount_writable_other_target_or_second_container(self):
        self.fixtures()
        for change in ("rw", "target", "second"):
            project = json.loads(json.dumps(self.project))
            if change == "rw":
                project[1]["Mounts"][0]["RW"] = True
            elif change == "target":
                project[1]["Mounts"][0]["Destination"] = "/elsewhere"
            else:
                project.append({"Id": "x", "Name": "/worker", "Config": {"Labels": {"com.docker.compose.service": "worker"}},
                                "Mounts": [{"Type": "bind", "Source": SOURCE, "Destination": TARGET, "RW": False}]})
            self.assertRefused("overlap:", project=project)

    def test_not_canonical_symlink_bind_and_non_top_volume(self):
        self.fixtures()
        runner = json.loads(json.dumps(self.runner))
        runner["Mounts"][0]["Source"] = "/volume1/link-to-share"
        self.assertRefused("source-not-canonical", runner=runner)
        # the kernel's root differs from Source: a host bind mount at a component
        self.assertRefused("source-not-canonical", mountinfo=f"100 50 0:50 /other/place {TARGET} rw - btrfs /dev/vg1 rw\n")
        self.assertRefused("source-not-canonical", mountinfo=f"100 50 0:50 /deep/oteryn-attempt-evidence {TARGET} rw - btrfs x rw\n")

    def test_state_mount_alias_into_evidence_share_refuses_on_mountinfo_path(self):
        self.fixtures()
        runner = json.loads(json.dumps(self.runner))
        runner["Mounts"][1]["Source"] = "/volume1/docker/alias"
        mountinfo = (f"100 50 0:50 /oteryn-attempt-evidence {TARGET} rw - btrfs x rw\n"
                     "101 50 0:50 /oteryn-attempt-evidence/state /var/lib/oteryn/state rw - btrfs x rw\n")
        self.assertRefused("overlap:", runner=runner, mountinfo=mountinfo)

    def test_directory_ownership_mode_writability_readonly_and_foreign_entries(self):
        self.fixtures()
        st = os.stat(self.dir)
        self.assertRefused("directory-owner", owner_uid=st.st_uid + 1)
        os.chmod(self.dir, 0o2755)
        self.assertRefused("directory-mode")
        os.chmod(self.dir, 0o2750)
        with mock.patch.object(os, "access", return_value=False):
            self.assertRefused("not-writable")
        with mock.patch.object(os, "statvfs", return_value=mock.Mock(f_flag=os.ST_RDONLY)):
            self.assertRefused("read-only-filesystem")
        Path(self.dir, "stray.txt").write_text("x")
        self.assertRefused("foreign-entry:stray.txt")

    def test_existing_run_file_mode_and_group(self):
        self.fixtures()
        run = self.d.start()
        path = os.path.join(self.dir, run + ".jsonl")
        os.chmod(path, 0o600)
        self.assertRefused("run-file-mode")
        os.chmod(path, 0o640)
        code, _ = self.check()
        self.assertEqual(code, 0)

    def test_check_location_is_pure_read(self):
        self.fixtures()
        self.d.start()
        boom = AssertionError("check-location must not write")
        real_open, real_os_open = builtins.open, os.open

        def guarded_open(file, mode="r", *a, **k):
            if any(c in mode for c in "wax+"):
                raise boom
            return real_open(file, mode, *a, **k)

        def guarded_os_open(path, flags, *a, **k):
            if flags & (os.O_CREAT | os.O_WRONLY | os.O_RDWR | os.O_TRUNC):
                raise boom
            return real_os_open(path, flags, *a, **k)

        before = sorted(os.listdir(self.dir))
        patches = [mock.patch.object(os, n, side_effect=boom) for n in
                   ("unlink", "remove", "rename", "replace", "mkdir", "rmdir", "symlink", "link", "truncate", "chmod", "fchmod")]
        args = self.prepare()
        with contextlib.ExitStack() as stack:
            for p in patches:
                stack.enter_context(p)
            code, out = self.d.cli(*args)
        self.assertEqual(code, 0, out)
        self.assertEqual(sorted(os.listdir(self.dir)), before)
        # builtin open / os.open are exercised through the guards as well
        with mock.patch.object(builtins, "open", guarded_open), mock.patch.object(os, "open", guarded_os_open):
            code, _ = self.d.cli(*args)
        self.assertEqual(code, 0)

    def test_docker_helper_allows_read_only_commands_only(self):
        with self.assertRaises(ae.Refusal):
            ae._docker(["rm", "x"])
        with self.assertRaises(ae.Refusal):
            ae._docker(["run", "x"])


class WriterTest(Base):
    def test_exclusive_create_and_mode_under_umask(self):
        old = os.umask(0o077)
        self.addCleanup(os.umask, old)
        run = self.d.start()
        path = os.path.join(self.dir, run + ".jsonl")
        self.assertEqual(stat.S_IMODE(os.stat(path).st_mode), 0o640)
        self.assertEqual(os.stat(path).st_gid, os.getgid())
        self.d.append("init-intent", run)
        self.assertEqual(stat.S_IMODE(os.stat(path).st_mode), 0o640)
        with self.assertRaises(FileExistsError):
            os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY)

    @unittest.skipUnless(os.geteuid() == 0 and hasattr(os, "setgroups"), "needs root to drop to the evidence group only")
    def test_group_only_process_reads_intent_and_anchored(self):
        run = self.d.full_to_intent()
        os.chmod(self.tmp, 0o755)
        os.chown(os.path.join(self.dir, run + ".jsonl"), 0, 4242)
        os.chmod(self.dir, 0o2750)
        os.chown(self.dir, 0, 4242)
        code = ("import sys; d=open(sys.argv[1],'rb').read().split(b'\\n'); print(len(d))")
        proc = subprocess.run([sys.executable, "-I", "-c", code, os.path.join(self.dir, run + ".jsonl")],
                              capture_output=True, text=True, user=65000, group=4242, extra_groups=[])
        self.assertEqual(proc.returncode, 0, proc.stderr)

    def test_open_refuses_before_first_line_when_fchmod_fails(self):
        with mock.patch.object(os, "fchmod", side_effect=OSError("nope")):
            code, out = self.d.open(none_obs())
        self.assertEqual(code, 1)
        self.assertEqual(out["error"], "run-file-mode")
        (name,) = self.run_files()
        self.assertEqual(os.path.getsize(os.path.join(self.dir, name)), 0)
        # the empty file then fails the anchor check of the next open
        code, out = self.d.open(none_obs())
        self.assertEqual(out["reason"], "evidence-lost")

    def test_open_refuses_wrong_group(self):
        code, out = self.d.cli("open", "--dir", self.dir, "--pre-init", json.dumps(none_obs()), "--location",
                               json.dumps({"ok": True}), "--group-gid", os.getgid() + 12345,
                               "--anchors", self.d.anchors_file())
        self.assertEqual(out["error"], "run-file-mode")

    def test_run_ids_are_fresh_v4_and_chain_is_linked(self):
        a, b = self.d.start(), None
        self.assertRegex(a, r"^[0-9a-f-]{36}$")
        self.assertEqual(ae.uuid.UUID(a).version, 4)
        self.d.append("init-intent", a)
        lines = self.d.lines(a)
        second = json.loads(lines[1])
        self.assertEqual(second["prev"], ae.sha256(lines[0]))
        self.assertEqual(second["seq"], 2)

    def test_append_run_open_and_unknown_type_refused(self):
        run = self.d.start()
        self.assertEqual(self.d.append("run-open", run)[1]["error"], "use-open")

    def test_append_anchored_rules(self):
        run = self.d.start()
        code, out = self.d.append("anchored", run, {"anchor_comment_id": 5})
        self.assertEqual(out["error"], "bad-sequence")
        self.d.append("init-intent", run)
        code, out = self.d.append("anchored", run, {})
        self.assertEqual(out["error"], "anchor-comment-required")
        code, out = self.d.append("anchored", run, {"anchor_comment_id": self.d.comments[-1]["id"]})
        self.assertEqual(code, 0)
        self.assertIsNone(out["anchor"])

    def test_init_requires_anchored_intent_and_one_store(self):
        run = self.d.start()
        self.d.append("init-intent", run)
        code, out = self.d.append("init", run, {"result": "created", "store_id": S1, "observation": obs()})
        self.assertEqual(out["error"], "bad-sequence")  # D1 may not run before the anchored line
        self.d.anchored(run)
        code, out = self.d.append("init", run, {"result": "created", "store_id": S1, "observation": obs()})
        self.assertEqual(code, 0)
        self.assertEqual(out["record"]["observation"]["store"]["binding"], "match")
        self.d.append("run-close", run, {"done": False})
        # a later init with another store id refuses
        code, out = self.d.open(obs(store=S1, binding="marker_only"))
        self.assertEqual(code, 0, out)
        run2 = out["run_id"]
        self.d.append("init-intent", run2)
        self.d.anchored(run2)
        code, out = self.d.append("init", run2, {"result": "unchanged", "store_id": S2, "observation": obs(store=S2)})
        self.assertEqual(out["error"], "store-id-mismatch")

    def test_init_result_not_allowed_writes_begin_refused(self):
        run = self.d.start()
        self.d.append("init-intent", run)
        self.d.anchored(run)
        code, out = self.d.append("init", run, {"result": "refused", "store_id": S1, "observation": obs()})
        self.assertEqual(code, 1)
        self.assertEqual(out["record"]["type"], "begin-refused")

    def test_init_observation_must_continue_pre_init(self):
        run = self.d.start(none_obs(lsn=0x5000))
        self.d.append("init-intent", run)
        self.d.anchored(run)
        code, out = self.d.append("init", run, {"result": "created", "store_id": S1, "observation": obs(lsn=0x10)})
        self.assertEqual(out["reason"], "store-changed")

    def test_directory_with_two_store_ids_fails_verify(self):
        run = self.d.start()
        self.d.do_init(run)
        self.d.append("run-close", run, {"done": False})
        # hand craft a second run with another store (bypassing append init)
        other = str(ae.uuid.uuid4())
        rec, raw = ae.make_line(other, 1, None, "run-open", {"store_id": None, "location": {}, "pre_init": none_obs()})
        h1 = ae.sha256(raw)
        rec2, raw2 = ae.make_line(other, 2, h1, "init", {"store_id": S2, "result": "created", "observation": obs(store=S2)})
        Path(self.dir, other + ".jsonl").write_bytes(raw + b"\n" + raw2 + b"\n")
        code, out = self.d.cli("verify", "--dir", self.dir)
        self.assertEqual(code, 1)
        self.assertIn("init records for more than one store id", out["errors"])

    def test_intent_preconditions_write_begin_refused(self):
        cases = [
            ("floor", obs(floor=1), "precondition:floor"),
            ("temp", obs(temp=1), "precondition:floor"),
            ("profile", obs(profile=profile()), "precondition:profile"),
            ("history", obs(in_history=True), "precondition:key-history"),
            ("configured", obs(configured="other"), "precondition:configured-key"),
            ("inflight", obs(in_flight=1), "precondition:in-flight"),
            ("nulldb", obs(db=False), "store-changed"),
            ("binding", obs(binding="marker_only"), "store-changed"),
            ("lower-lsn", obs(lsn=1), "store-changed"),
            ("sysid", obs(sysid="9"), "store-changed"),
            ("timeline", obs(tl=2), "store-changed"),
            ("oid", obs(oid=1), "store-changed"),
            ("relfilenode", obs(rel=(9, 2, 3)), "store-changed"),
        ]
        for name, observation, reason in cases:
            with self.subTest(name):
                d = Driver(self._fresh())
                run = d.start()
                d.do_init(run)
                code, out = d.do_intent(run, observation)
                self.assertEqual(code, 1, out)
                self.assertEqual(out["reason"], reason)
                self.assertEqual(out["record"]["type"], "begin-refused")

    def _fresh(self):
        path = tempfile.mkdtemp(prefix="ev-", dir=self.tmp)
        os.chmod(path, 0o2750)
        sub = os.path.join(path, "ev")
        os.mkdir(sub)
        os.chmod(sub, 0o2750)
        return sub

    def test_intent_passes_with_reboot_and_lower_xmax(self):
        run = self.d.start()
        self.d.do_init(run, observation=obs(lsn=0x2000, xmax=500))
        code, out = self.d.do_intent(run, obs(lsn=0x2000, xmax=10, pmst="reboot"))
        self.assertEqual(code, 0, out)

    def test_crash_where_write_position_fell_but_flush_did_not(self):
        o = obs(lsn=0x2000)
        o["database"]["progress"]["wal_write_lsn"] = "0/9000"
        run = self.d.start()
        self.d.do_init(run, observation=o)
        n = obs(lsn=0x2000)
        n["database"]["progress"]["wal_write_lsn"] = "0/2500"
        code, out = self.d.do_intent(run, n)
        self.assertEqual(code, 0, out)

    def test_intent_ordering_and_unobserved_attempt(self):
        run = self.d.full_to_intent()
        code, out = self.d.do_intent(run)
        self.assertEqual(out["error"], "attempt-unobserved")
        code, out = self.d.open(obs(binding="match"))
        self.assertEqual(out["reason"], "attempt-unclosed")

    def test_intent_store_scope_binding_enforced(self):
        run = self.d.start()
        self.d.do_init(run)
        code, out = self.d.do_intent(run, store_id=S2)
        self.assertEqual(out["error"], "store-id-mismatch")


class ClassifyTest(Base):
    def run_attempt(self, o1, o2=None, intent_obs=None, anchor_intent=True):
        run = self.d.start()
        self.d.do_init(run)
        self.d.do_intent(run, intent_obs)
        if anchor_intent:
            self.d.anchored(run)
        code, out = self.d.observe(run, o1, o2)
        self.assertEqual(code, 0, out)
        return run, out["record"]["classification"]

    def test_committed(self):
        run, cls = self.run_attempt(committed_obs(lsn=0x1100))
        self.assertEqual(cls, "committed")
        self.assertEqual(self.d.verdict(), "self-check-pending")
        self.d.append("self-check", run, {"passed": True, "observation": committed_obs(lsn=0x1200)})
        self.assertEqual(self.d.append("run-close", run, {"done": True})[0], 0)
        self.assertEqual(self.d.verdict(), "committed")

    def test_observation_for_another_key_is_not_committed(self):
        other = committed_obs(lsn=0x1100)
        other["key"]["requested"] = "kid-other"
        _, cls = self.run_attempt(other)
        self.assertNotEqual(cls, "committed")

    def test_failed_before_witness_and_retry(self):
        run, cls = self.run_attempt(obs(lsn=0x1100))
        self.assertEqual(cls, "failed-before-witness")
        self.assertEqual(self.d.verdict(), "failed-before-witness")
        code, out = self.d.do_intent(run, obs(lsn=0x1200))
        self.assertEqual(code, 0, out)
        self.assertEqual(out["record"]["attempt"], 2)

    def test_failed_after_witness(self):
        _, cls = self.run_attempt(obs(floor=1, lsn=0x1100))
        self.assertEqual(cls, "failed-after-witness")
        self.assertEqual(self.d.verdict(), "failed-after-witness")

    def test_witness_ahead(self):
        _, cls = self.run_attempt(obs(floor=2, profile=profile(1), trusted=True, lsn=0x1100))
        self.assertEqual(cls, "witness-ahead")

    def test_unknown_cases(self):
        cases = {
            "identity changed": (obs(floor=1, sysid="x", lsn=0x1100), None),
            "restart": (obs(floor=1, pmst="t9", lsn=0x1100), None),
            "null database": (obs(db=False), None),
            "o1 differs from o2": (obs(lsn=0x1100), obs(lsn=0x1200, floor=1)),
            "progress decreased": (obs(lsn=0x10), None),
            "in flight": (obs(in_flight=1, lsn=0x1100), None),
            "fence off": (obs(fence=False, lsn=0x1100), None),
            "untrusted key": (obs(floor=1, profile=profile(1), trusted=False, lsn=0x1100), None),
        }
        for name, (o1, o2) in cases.items():
            with self.subTest(name):
                self.d = Driver(self._fresh_dir())
                _, cls = self.run_attempt(o1, o2)
                self.assertEqual(cls, "unknown")
                self.assertEqual(self.d.verdict(), "unknown")

    def _fresh_dir(self):
        sub = os.path.join(tempfile.mkdtemp(prefix="ev-", dir=self.tmp), "ev")
        os.mkdir(sub)
        os.chmod(sub, 0o2750)
        return sub

    def test_floor_appearing_after_failed_before_witness_is_unknown(self):
        run, cls = self.run_attempt(obs(lsn=0x1100))
        self.assertEqual(cls, "failed-before-witness")
        self.d.append("run-close", run, {"done": False})
        # next observation for S shows the floor, before any later intent
        code, out = self.d.open(obs(floor=1, lsn=0x1300))
        self.assertEqual(code, 1)
        self.assertEqual(self.d.verdict(), "unknown")

    def test_restore_or_loss_after_committed(self):
        run, _ = self.run_attempt(committed_obs(lsn=0x1100))
        self.d.append("self-check", run, {"passed": True, "observation": committed_obs(lsn=0x1200)})
        self.d.append("run-close", run, {"done": True})
        # hand-written later attempt for the same store with no profile and a lower floor
        a = self.d
        later = str(ae.uuid.uuid4())
        runs, _ = ae.load_runs(self.dir)
        self.assertEqual(len(runs), 1)
        rec, raw = ae.make_line(later, 1, None, "run-open", {"store_id": None, "location": {}, "pre_init": obs(lsn=0x1400)})
        h = ae.sha256(raw)
        init = dict(S1=S1)
        recs = [(rec, raw)]
        prev = h
        seq = 2
        for typ, fields in (
            ("init", {"store_id": S1, "result": "unchanged", "observation": obs(lsn=0x1400)}),
            ("attempt-intent", {"attempt": 1, "store_id": S1, "scope": SCOPE, "key_id": K, "kid_record_count": 1,
                                "observation": obs(lsn=0x1400)}),
            ("attempt-observation", {"attempt": 1, "store_id": S1, "scope": SCOPE, "key_id": K,
                                     "observation_1": obs(lsn=0x1500), "observation_2": obs(lsn=0x1600)}),
        ):
            r, rw = ae.make_line(later, seq, prev, typ, fields)
            prev = ae.sha256(rw)
            recs.append((r, rw))
            seq += 1
        Path(self.dir, later + ".jsonl").write_bytes(b"".join(rw + b"\n" for _, rw in recs))
        os.chmod(os.path.join(self.dir, later + ".jsonl"), 0o640)
        for i, (r, rw) in enumerate(recs):
            if r["type"] in ("run-open", "attempt-observation"):
                a.post({"run_id": later, "seq": r["seq"], "type": r["type"], "line_sha256": ae.sha256(rw)})
        _, out = a.classify()
        self.assertIn("restore-or-loss", [x["classification"] for x in out["attempts"]])
        self.assertEqual(out["directory"], "restore-or-loss")

    def test_unanchored_intent_closed_by_anchored_observation(self):
        run = self.d.start()
        self.d.do_init(run)
        self.d.do_intent(run)  # posted? remove its anchor to simulate a failed post
        self.d.comments.pop()
        code, out = self.d.observe(run, obs(lsn=0x1100))
        self.assertEqual(out["record"]["classification"], "failed-before-witness")
        code, out = self.d.cli("verify", "--dir", self.dir, "--anchors", self.d.anchors_file())
        self.assertEqual(code, 0, out)
        self.assertEqual(self.d.verdict(), "failed-before-witness")
        # no anchored line: the fence guard refuses it
        code, _ = self.d.cli("require-open-attempt", "--dir", self.dir, "--store", S1, "--key-id", K,
                             "--scope", json.dumps(SCOPE), "--identity", json.dumps(obs()["database"]["identity"]))
        self.assertEqual(code, 1)


class AnchorTest(Base):
    def make(self):
        run = self.d.full_to_intent()
        self.d.observe(run, obs(lsn=0x1100))
        return run

    def verify(self):
        return self.d.cli("verify", "--dir", self.dir, "--anchors", self.d.anchors_file())

    def test_clean_run_verifies(self):
        self.make()
        code, out = self.verify()
        self.assertEqual(code, 0, out)

    def test_rewritten_anchored_line_fails_and_classify_unknown(self):
        run = self.make()
        path = os.path.join(self.dir, run + ".jsonl")
        lines = Path(path).read_bytes().split(b"\n")
        rec = json.loads(lines[0])
        rec["cutover_ref"] = "tampered"
        lines[0] = ae.jdump(rec).encode()
        Path(path).write_bytes(b"\n".join(lines))
        code, out = self.verify()
        self.assertEqual(code, 1)
        self.assertEqual(self.d.classify()[1]["directory"], "unknown")

    def test_regenerated_consistent_chain_fails(self):
        run = self.make()
        path = os.path.join(self.dir, run + ".jsonl")
        out, prev = [], None
        for raw in Path(path).read_bytes().split(b"\n")[:-1]:
            rec = json.loads(raw)
            rec["at"] = "2030-01-01T00:00:00.000000Z"
            rec["prev"] = prev
            line = ae.jdump(rec).encode()
            prev = ae.sha256(line)
            out.append(line)
        Path(path).write_bytes(b"\n".join(out) + b"\n")
        code, res = self.verify()
        self.assertEqual(code, 1)
        self.assertTrue(any("differs" in e or "anchored line" in e for e in res["errors"]), res)

    def test_truncated_file_and_unanchored_tail(self):
        run = self.make()
        path = os.path.join(self.dir, run + ".jsonl")
        data = Path(path).read_bytes().split(b"\n")
        Path(path).write_bytes(b"\n".join(data[:3]) + b"\n")
        self.assertEqual(self.verify()[0], 1)
        # unanchored tail: the last line has no anchor
        Path(path).write_bytes(b"\n".join(data))
        self.d.comments.pop()
        code, out = self.verify()
        self.assertEqual(code, 1)
        self.assertTrue(any("unanchored tail" in e for e in out["errors"]), out)

    def test_anchored_line_naming_foreign_author_comment_fails(self):
        run = self.d.start()
        self.d.append("init-intent", run)
        intent_anchor = self.d.comments.pop()
        self.d.comments.append({"id": 77, "author": {"id": 1, "login": "someone"}, "anchor": intent_anchor["anchor"]})
        self.d.append("anchored", run, {"anchor_comment_id": 77})
        self.d.post(intent_anchor["anchor"])  # the real anchor exists, but the line names comment 77
        code, out = self.verify()
        self.assertEqual(code, 1)
        self.assertTrue(any("anchored line" in e for e in out["errors"]), out)

    def test_foreign_author_comments_are_ignored(self):
        self.make()
        fake = {"id": 9, "author": {"id": 7, "login": "mallory"},
                "anchor": {"run_id": "00000000-0000-4000-8000-000000000000", "seq": 4, "type": "init", "line_sha256": "0" * 64}}
        fake_bot_name_wrong_id = {"id": 10, "author": {"id": 7, "login": "github-actions[bot]"}, "anchor": fake["anchor"]}
        self.d.comments += [fake, fake_bot_name_wrong_id]
        self.assertEqual(self.verify()[0], 0)
        code, out = self.d.open(none_obs())
        # not evidence-lost, only the real cause (an attempt exists with a failed-before-witness class)
        self.assertNotEqual(out.get("reason"), "evidence-lost")

    def test_anchor_retry_does_not_reuse_foreign_comment(self):
        trusted = ae.trusted_comment
        self.assertFalse(trusted({"author": {"id": 1, "login": "github-actions[bot]"}}))
        self.assertFalse(trusted({"author": {"id": 41898282, "login": "x"}}))
        self.assertTrue(trusted({"author": BOT}))

    def test_locked_false_refuses_open(self):
        self.d.locked = False
        code, out = self.d.open(none_obs())
        self.assertEqual(code, 1)
        self.assertEqual(out["reason"], "anchor-issue-unlocked")
        self.assertEqual(self.d.lines(out["run_id"]).__len__(), 2)

    def test_deleted_run_file_gives_evidence_lost_before_continuity(self):
        run = self.d.start()
        self.d.do_init(run)
        self.d.do_intent(run)
        self.d.anchored(run)
        self.d.observe(run, committed_obs(lsn=0x1100))
        self.d.append("run-close", run, {"done": False})
        os.unlink(os.path.join(self.dir, run + ".jsonl"))
        code, out = self.d.open(none_obs())
        self.assertEqual(out["reason"], "evidence-lost")
        self.assertEqual(self.d.verdict(), "unknown")

    def test_anchor_naming_missing_line_or_file_without_anchor_or_no_anchors_given(self):
        run = self.make()
        self.d.comments.append({"id": 99, "author": BOT, "anchor": {"run_id": run, "seq": 50, "type": "init", "line_sha256": "0" * 64}})
        code, out = self.d.open(none_obs())
        self.assertEqual(out["reason"], "evidence-lost")
        self.d.comments.pop()
        code, out = self.d.open(none_obs(), anchors=False)
        self.assertEqual(out["reason"], "evidence-lost")
        # a file with no anchored line at all
        self.d.comments.clear()
        code, out = self.d.open(none_obs())
        self.assertEqual(out["reason"], "evidence-lost")


class ContinuityTest(Base):
    def committed_run(self, close=True, selfcheck=True):
        run = self.d.start()
        self.d.do_init(run)
        self.d.do_intent(run)
        self.d.anchored(run)
        self.d.observe(run, committed_obs(lsn=0x1100))
        if selfcheck:
            self.d.append("self-check", run, {"passed": True, "observation": committed_obs(lsn=0x1200)})
        if close:
            self.d.append("run-close", run, {"done": selfcheck})
        return run

    def test_binding_none_or_other_store_after_commit_is_restore_or_loss(self):
        for pre in (none_obs(lsn=0x2000), obs(store=S2, lsn=0x2000)):
            with self.subTest(pre["store"]["binding"]):
                self.d = Driver(self._fresh())
                self.committed_run()
                code, out = self.d.open(pre)
                self.assertEqual(out["reason"], "store-changed")
                self.assertEqual(self.d.verdict(), "restore-or-loss")

    def _fresh(self):
        sub = os.path.join(tempfile.mkdtemp(prefix="ev-", dir=self.tmp), "ev")
        os.mkdir(sub)
        os.chmod(sub, 0o2750)
        return sub

    def test_store_changed_without_commit_is_unknown(self):
        run = self.d.start()
        self.d.do_init(run)
        self.d.append("run-close", run, {"done": False})
        code, out = self.d.open(none_obs(lsn=0x2000))
        self.assertEqual(out["reason"], "store-changed")
        self.assertEqual(self.d.verdict(), "unknown")

    def test_database_continuity_cases(self):
        cases = [("lower lsn", obs(lsn=0x10)), ("sysid", obs(sysid="9", lsn=0x2000)), ("timeline", obs(tl=7, lsn=0x2000)),
                 ("oid", obs(oid=3, lsn=0x2000)), ("relfilenode", obs(rel=(0, 0, 0), lsn=0x2000)),
                 ("null db", obs(db=False))]
        for name, pre in cases:
            with self.subTest(name):
                self.d = Driver(self._fresh())
                run = self.d.start()
                self.d.do_init(run)
                self.d.append("run-close", run, {"done": False})
                code, out = self.d.open(pre)
                self.assertEqual(out["reason"], "store-changed")

    def test_changed_postmaster_start_alone_passes(self):
        run = self.d.start()
        self.d.do_init(run)
        self.d.append("run-close", run, {"done": False})
        code, out = self.d.open(obs(pmst="reboot", lsn=0x3000, xmax=1))
        self.assertEqual(code, 0, out)

    def test_open_after_commit_writes_step4_complete_or_self_check_pending(self):
        self.committed_run()
        code, out = self.d.open(obs(lsn=0x2000))
        self.assertEqual(out["reason"], "step4-complete")
        self.d = Driver(self._fresh())
        self.committed_run(close=False, selfcheck=False)
        code, out = self.d.open(obs(lsn=0x2000))
        self.assertEqual(out["reason"], "self-check-pending")

    def test_unfinished_init_intent_blocks_open_before_continuity(self):
        run = self.d.start()
        self.d.append("init-intent", run)
        self.d.anchored(run)
        code, out = self.d.open(none_obs())
        self.assertEqual(out["reason"], "init-unfinished")
        self.assertEqual(self.d.verdict(), "unknown")

    def test_precondition_refusals_at_open(self):
        for pre, reason in ((obs(binding="mismatch"), "precondition:binding"),
                            (none_obs(floor=1), "precondition:floor"),
                            (none_obs(profile=profile()), "precondition:profile")):
            with self.subTest(reason):
                self.d = Driver(self._fresh())
                code, out = self.d.open(pre)
                self.assertEqual(out["reason"], reason)

    def test_failed_after_witness_blocks_new_run_and_intent(self):
        run = self.d.start()
        self.d.do_init(run)
        self.d.do_intent(run)
        self.d.anchored(run)
        self.d.observe(run, obs(floor=1, lsn=0x1100))
        self.d.append("run-close", run, {"done": False})
        code, out = self.d.open(obs(floor=None, lsn=0x2000))
        self.assertEqual(out["reason"], "blocked:failed-after-witness")


class RecheckTest(Base):
    def setUp(self):
        super().setUp()
        self.run = self.d.full_to_intent()
        self.d.observe(self.run, committed_obs(lsn=0x1100))

    def recheck(self, o, anchors=True):
        extra = ["--anchors", self.d.anchors_file()] if anchors else []
        return self.d.append("self-check", self.run, {"passed": True, "late": True, "observation": o}, extra=extra)

    def test_failed_then_late_self_check_closes(self):
        self.d.append("self-check", self.run, {"passed": False, "observation": committed_obs(lsn=0x1200)})
        code, out = self.recheck(committed_obs(lsn=0x1300, pmst="reboot"))
        self.assertEqual(code, 0, out)
        self.assertTrue(out["record"]["late"])
        self.assertEqual(self.d.append("run-close", self.run, {"done": True})[0], 0)
        self.assertEqual(self.d.verdict(), "committed")

    def test_missing_self_check_then_recheck(self):
        self.assertEqual(self.recheck(committed_obs(lsn=0x1300))[0], 0)

    def test_recheck_refusals_write_nothing(self):
        bad = {
            "other identity": committed_obs(sysid="zz", lsn=0x1300),
            "other store": committed_obs(store=S2, lsn=0x1300),
            "no profile": obs(lsn=0x1300),
            "floor above profile": obs(floor=2, profile=profile(1), trusted=True, lsn=0x1300),
            "lower lsn": committed_obs(lsn=0x10),
        }
        before = Path(self.dir, self.run + ".jsonl").read_bytes()
        for name, o in bad.items():
            with self.subTest(name):
                code, out = self.recheck(o)
                self.assertEqual(code, 1, out)
                self.assertEqual(out["error"], "recheck-refused")
                self.assertEqual(Path(self.dir, self.run + ".jsonl").read_bytes(), before)
        code, out = self.recheck(committed_obs(lsn=0x1300), anchors=False)
        self.assertEqual(out["error"], "anchors-required")

    def test_recheck_refuses_failed_anchor_verify_and_closed_run(self):
        self.d.comments.pop()
        code, out = self.recheck(committed_obs(lsn=0x1300))
        self.assertEqual(out["error"], "recheck-refused")
        self.d.post(None)
        # re-post by recomputing: simplest is to close the run and expect bad-sequence afterwards
        self.assertEqual(self.d.append("self-check", self.run, {"passed": True, "observation": committed_obs(lsn=0x1200)})[0], 0)
        self.d.append("run-close", self.run, {"done": True})
        code, out = self.d.append("self-check", self.run, {"passed": True, "late": True, "observation": committed_obs(lsn=0x1300)})
        self.assertEqual(out["error"], "run-closed")

    def test_recheck_never_calls_other_commands(self):
        with mock.patch.object(ae.subprocess, "run", side_effect=AssertionError("no subprocess")):
            self.assertEqual(self.recheck(committed_obs(lsn=0x1300))[0], 0)


class InitIntentTest(Base):
    def test_close_init_paths(self):
        # init-intent without anchored line: the init never ran
        run = self.d.start()
        self.d.append("init-intent", run)
        code, out = self.d.append("begin-refused", run, {"reason": "init-not-run", "observation": none_obs()})
        self.assertEqual(code, 1)
        code, out = self.d.open(none_obs())
        self.assertEqual(code, 0, out)

    def test_close_init_lost_is_permanent(self):
        run = self.d.start()
        self.d.append("init-intent", run)
        self.d.anchored(run)
        self.d.append("begin-refused", run, {"reason": "init-lost", "observation": none_obs()})
        self.assertEqual(self.d.verdict(), "unknown")
        code, out = self.d.open(none_obs())
        self.assertEqual(out["reason"], "blocked:init-lost")

    def test_init_lost_stays_permanent_after_run_close(self):
        run = self.d.start()
        self.d.append("init-intent", run)
        self.d.anchored(run)
        self.d.append("begin-refused", run, {"reason": "init-lost", "observation": none_obs()})
        self.d.append("run-close", run, {"done": False})
        self.assertEqual(self.d.verdict(), "unknown")
        code, out = self.d.open(none_obs())
        self.assertEqual(out["reason"], "blocked:init-lost")

    def test_late_init_for_marker_only_carries_post_d1_observation(self):
        run = self.d.start()
        self.d.append("init-intent", run)
        self.d.anchored(run)
        code, out = self.d.append("init", run, {"result": "completed", "store_id": S1, "late": True,
                                                "observation": obs(lsn=0x1500)})
        self.assertEqual(code, 0, out)
        self.assertTrue(out["record"]["late"])
        self.assertEqual(out["record"]["observation"]["store"]["binding"], "match")

    def test_unfinished_init_intent_blocks_intents_of_other_runs(self):
        a = self.d.start()
        self.d.append("init-intent", a)
        self.d.anchored(a)
        self.assertEqual(self.d.verdict(), "unknown")


class RequireOpenAttemptTest(Base):
    def call(self, store=S1, key=K, scope=SCOPE, identity=None, anchors=False):
        identity = identity if identity is not None else obs()["database"]["identity"]
        args = ["require-open-attempt", "--dir", self.dir, "--store", store, "--key-id", key,
                "--scope", json.dumps(scope), "--identity", json.dumps(identity)]
        if anchors:
            args += ["--anchors", self.d.anchors_file()]
        return self.d.cli(*args)

    def test_prints_fence(self):
        run = self.d.full_to_intent()
        code, out = self.call(anchors=True)
        self.assertEqual(code, 0, out)
        self.assertEqual(set(out), {"run_id", "intent_sha256"})
        self.assertEqual(out["run_id"], run)
        intent_hash = ae.sha256(self.d.lines(run)[-2])
        self.assertEqual(out["intent_sha256"], intent_hash)

    def test_cli_process_stdout_and_stderr(self):
        self.d.full_to_intent()
        base = [sys.executable, "-I", str(TOOL), "require-open-attempt", "--dir", self.dir, "--store", S1,
                "--key-id", K, "--scope", json.dumps(SCOPE, sort_keys=True, separators=(",", ":"))]
        ok = subprocess.run([*base, "--identity", json.dumps(obs()["database"]["identity"], sort_keys=True, separators=(",", ":"))],
                            capture_output=True, text=True)
        self.assertEqual(ok.returncode, 0, ok.stderr)
        self.assertEqual(set(json.loads(ok.stdout)), {"run_id", "intent_sha256"})
        bad = subprocess.run([*base, "--identity", "{}"], capture_output=True, text=True)
        self.assertNotEqual(bad.returncode, 0)
        self.assertEqual(bad.stdout, "")
        self.assertIn("identity-mismatch", bad.stderr)

    def test_refusals(self):
        self.d.full_to_intent()
        self.assertEqual(self.call(store=S2)[0], 1)
        self.assertEqual(self.call(key="other")[0], 1)
        self.assertEqual(self.call(scope={"issuer": "x"})[0], 1)
        self.assertEqual(self.call(identity={"system_identifier": "9"})[0], 1)
        self.assertEqual(self.call(identity=dict(obs()["database"]["identity"], postmaster_start_time="reboot"))[0], 1)

    def test_intent_without_anchored_line_refused(self):
        self.d.full_to_intent(anchor_intent=False)
        code, out = self.call()
        self.assertEqual(out["error"], "intent-not-anchored")

    def test_no_open_attempt_and_observed_attempt(self):
        run = self.d.full_to_intent()
        self.d.observe(run, obs(lsn=0x1100))
        self.assertEqual(self.call()[1]["error"], "no-open-attempt")

    def test_blocking_state_and_extra_line_refuse(self):
        run = self.d.full_to_intent()
        self.d.append("begin-refused", run, {"reason": "anything"})
        self.assertEqual(self.call()[1]["error"], "intent-not-anchored")

    def test_broken_chain_refuses(self):
        run = self.d.full_to_intent()
        path = os.path.join(self.dir, run + ".jsonl")
        lines = Path(path).read_bytes().split(b"\n")
        rec = json.loads(lines[1])
        rec["at"] = "x"
        lines[1] = ae.jdump(rec).encode()
        Path(path).write_bytes(b"\n".join(lines))
        self.assertEqual(self.call()[1]["error"], "verify-failed")

    def test_second_open_intent_in_other_run_refuses(self):
        self.d.full_to_intent()
        # a second run with its own open intent
        other = Driver(self.dir)
        other.comments = self.d.comments
        other.next_id = self.d.next_id
        self.d.comments  # shared
        code, out = self.d.open(obs(), anchors=True)
        self.assertEqual(out["reason"], "attempt-unclosed")


class NoNetworkTest(unittest.TestCase):
    def test_only_stdlib_and_read_only_docker(self):
        source = TOOL.read_text()
        for word in ("urllib", "socket.create_connection", "requests", "http.client"):
            self.assertNotIn(word, source)
        self.assertIn('args[0] not in ("inspect", "info", "ps")', source)
        with mock.patch.object(ae.subprocess, "run", side_effect=AssertionError("subprocess")):
            d = tempfile.mkdtemp(prefix="ev-")
            self.addCleanup(shutil.rmtree, d, True)
            code, out = Driver(d).cli("verify", "--dir", d)
            self.assertEqual(code, 0, out)


if __name__ == "__main__":
    unittest.main()
