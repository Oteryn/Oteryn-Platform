#!/usr/bin/env python3
"""Attempt evidence writer for the PostgreSQL P6a trust safeguards.

Design: docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md (D3 to D6, D8 guard).

Standard library only; runs with `python3 -I`. It makes no network call and starts no subprocess that
writes: `check-location` only runs `docker inspect`, `docker info` and `docker ps`. It never reads or
writes the witness directory, the database or the key volume. Every observation is passed in by the
caller as JSON (the output of the D1 and D2 console commands).

Subcommands (all print one JSON object on stdout; non-zero exit on refusal):

  check-location --dir D [--target T] [--runner-container C] [--project P] [--platform-service S]
                 [--owner-uid U] [--group-gid G] [--dir-mode M]
                 [--runner-inspect F] [--project-inspect F] [--docker-info F] [--mountinfo F]
  open           --dir D --anchors F --pre-init JSON --location JSON [--cutover-ref R] [--group-gid G]
  append TYPE    --dir D --run ID [--data JSON] [--comment-id N] [--anchors F]
  verify         --dir D [--anchors F]
  classify       --dir D [--anchors F]
  require-open-attempt --dir D --store S --key-id K --scope JSON --identity JSON [--anchors F]

JSON arguments accept a literal, `@path` or `-` (stdin). `classify` is information only.
"""

from __future__ import annotations

import argparse
import copy
import datetime
import fcntl
import hashlib
import json
import os
import posixpath
import re
import socket
import stat
import subprocess
import sys
import uuid

TRUSTED_AUTHOR_ID = 41898282
TRUSTED_AUTHOR_LOGIN = "github-actions[bot]"
DEFAULT_TARGET = "/var/lib/oteryn-attempt-evidence"
RUN_FILE = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\.jsonl$")
SYSTEM_DIRS = {"@eaDir", "#recycle"}
TYPES = {
    "run-open", "init-intent", "init", "begin-refused", "attempt-intent", "anchored",
    "attempt-observation", "self-check", "run-close",
}
PERMANENT = {"store-changed", "evidence-lost", "init-lost"}
BLOCKING = {"failed-after-witness", "restore-or-loss", "unknown"}
SEVERITY = ["unknown", "restore-or-loss", "failed-after-witness", "witness-ahead", "failed-before-witness", "committed"]
INIT_RESULTS = {"created", "completed", "unchanged"}
FS_IOC_GETFLAGS = 0x80086601
FS_APPEND_FL = 0x20


class Refusal(Exception):
    def __init__(self, code, message="", **extra):
        super().__init__(code)
        self.code = code
        self.message = message or code
        self.extra = extra


def utcnow():
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def jdump(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"))


def sha256(raw):
    return hashlib.sha256(raw).hexdigest()


def jarg(value, what="argument"):
    if value is None:
        return None
    try:
        if value == "-":
            return json.loads(sys.stdin.read())
        if value.startswith("@"):
            with open(value[1:], "r", encoding="utf-8") as handle:
                return json.load(handle)
        return json.loads(value)
    except (OSError, ValueError) as exc:
        raise Refusal("bad-json", f"{what}: {exc}")


# ---------------------------------------------------------------- observation helpers

def parse_lsn(text):
    hi, lo = str(text).split("/")
    return (int(hi, 16) << 32) | int(lo, 16)


def obs_db(o):
    db = o.get("database") if isinstance(o, dict) else None
    return db if isinstance(db, dict) else None


def stable_identity(identity):
    return {k: v for k, v in identity.items() if k != "postmaster_start_time"}


def flush_lsn(o):
    return parse_lsn(obs_db(o)["progress"]["wal_flush_lsn"])


def xmax_of(o):
    return int(obs_db(o)["progress"]["xmax"])


def store_block(o):
    s = o.get("store") if isinstance(o, dict) else None
    return s if isinstance(s, dict) else {}


def _id(v):
    if isinstance(v, dict):
        v = v.get("id")
    return v if isinstance(v, str) and v else None


def store_id_of(o):
    s = store_block(o)
    return _id(s.get("marker")) or _id(s.get("row"))


def binding_of(o):
    return store_block(o).get("binding")


def floor_value(o):
    f = o.get("floor")
    return f.get("value") if isinstance(f, dict) else f


def floor_temp(o):
    f = o.get("floor")
    return int(f.get("temp_entries", 0) or 0) if isinstance(f, dict) else 0


def profile_of(o):
    p = o.get("profile")
    if isinstance(p, dict) and p.get("present", True):
        return p
    return None


def _zero(v):
    if isinstance(v, bool):
        return False
    if isinstance(v, int):
        return v == 0
    if isinstance(v, dict) and v:
        return all(_zero(x) for x in v.values())
    return False


def in_flight_zero(o):
    db = obs_db(o)
    return db is not None and _zero(db.get("in_flight"))


def strip_progress(o):
    o = copy.deepcopy(o)
    db = obs_db(o)
    if db is not None:
        db.pop("progress", None)
    return o


def continues(o, stable, highwater):
    """D4 database continuity. stable: identity without postmaster_start_time; highwater: int or None."""
    try:
        db = obs_db(o)
        if db is None or stable is None:
            return False
        if stable_identity(db["identity"]) != stable:
            return False
        return highwater is None or flush_lsn(o) >= highwater
    except (KeyError, TypeError, ValueError, AttributeError):
        return False


def observations_of(rec):
    """Every observation a record carries, as (field, observation)."""
    out = []
    for key in ("pre_init", "observation", "observation_1", "observation_2"):
        if isinstance(rec.get(key), dict):
            out.append((key, rec[key]))
    return out


# ---------------------------------------------------------------- loading and verifying

class Run:
    def __init__(self, rid, path):
        self.id = rid
        self.path = path
        self.recs = []
        self.hashes = []
        self.errors = []

    @property
    def first_at(self):
        return self.recs[0].get("at", "") if self.recs else ""


def read_run(path, rid):
    run = Run(rid, path)
    with open(path, "rb") as handle:
        data = handle.read()
    if not data:
        run.errors.append("empty-file")
        return run
    parts = data.split(b"\n")
    if parts[-1] != b"":
        run.errors.append("unterminated-last-line")
        parts = parts[:-1]
    else:
        parts = parts[:-1]
    prev = None
    for i, raw in enumerate(parts):
        try:
            rec = json.loads(raw.decode("utf-8"))
        except ValueError:
            run.errors.append(f"line {i + 1}: not json")
            break
        problem = None
        if not isinstance(rec, dict):
            problem = "not an object"
        elif rec.get("v") != 1 or rec.get("run_id") != rid:
            problem = "bad v or run_id"
        elif rec.get("seq") != i + 1:
            problem = "bad seq"
        elif rec.get("prev") != prev:
            problem = "broken hash chain"
        elif rec.get("type") not in TYPES:
            problem = "unknown type"
        elif not isinstance(rec.get("at"), str):
            problem = "no at"
        if problem:
            run.errors.append(f"line {i + 1}: {problem}")
            break
        prev = sha256(raw)
        run.recs.append(rec)
        run.hashes.append(prev)
    return run


def load_runs(directory):
    try:
        names = sorted(os.listdir(directory))
    except OSError as exc:
        raise Refusal("directory-unreadable", str(exc))
    runs, foreign = [], []
    for name in names:
        path = os.path.join(directory, name)
        if RUN_FILE.match(name) and stat.S_ISREG(os.lstat(path).st_mode):
            runs.append(read_run(path, name[:-6]))
        elif name in SYSTEM_DIRS:
            continue
        else:
            foreign.append(name)
    runs.sort(key=lambda r: (r.first_at, r.id))
    return runs, foreign


def structure_errors(runs, foreign=()):
    errors = [f"foreign-entry:{n}" for n in foreign]
    store_ids = set()
    for run in runs:
        errors += [f"{run.id}: {e}" for e in run.errors]
        recs = run.recs
        if not recs:
            continue
        if recs[0]["type"] != "run-open":
            errors.append(f"{run.id}: first line is not run-open")
        init_store = None
        intent = None
        for i, rec in enumerate(recs):
            t = rec["type"]
            if t == "run-open" and i != 0:
                errors.append(f"{run.id}: run-open not first")
            if init_store is not None and t != "init" and rec.get("store_id") != init_store:
                errors.append(f"{run.id}: seq {rec['seq']} store_id differs from init")
            if t == "init":
                sid = rec.get("store_id")
                if not isinstance(sid, str) or not sid:
                    errors.append(f"{run.id}: init without store_id")
                if init_store not in (None, sid):
                    errors.append(f"{run.id}: second init store")
                init_store = sid
                store_ids.add(sid)
            if t == "attempt-intent":
                intent = rec
            elif t in ("attempt-observation", "self-check"):
                if intent is None or rec.get("scope") != intent.get("scope") or rec.get("key_id") != intent.get("key_id") \
                        or rec.get("attempt") != intent.get("attempt"):
                    errors.append(f"{run.id}: seq {rec['seq']} does not match its intent")
            if t == "anchored":
                prev = recs[i - 1] if i else None
                if prev is None or prev["type"] not in ("attempt-intent", "init-intent") \
                        or rec.get("for_seq") != prev["seq"] or rec.get("for_line_sha256") != run.hashes[i - 1] \
                        or not rec.get("anchor_comment_id"):
                    errors.append(f"{run.id}: seq {rec['seq']} anchored line does not name its intent")
    if len(store_ids) > 1:
        errors.append("init records for more than one store id")
    return errors


def trusted_comment(c):
    a = c.get("author") if isinstance(c, dict) else None
    return isinstance(a, dict) and a.get("id") == TRUSTED_AUTHOR_ID and a.get("login") == TRUSTED_AUTHOR_LOGIN


def load_anchors(path):
    """Returns (locked, {comment_id: anchor}) with only trusted-author comments, or raises Refusal."""
    try:
        with open(path, "r", encoding="utf-8") as handle:
            doc = json.load(handle)
        comments = doc.get("comments", [])
        locked = doc.get("locked") is True
        if not isinstance(comments, list):
            raise ValueError("comments")
    except (OSError, ValueError, AttributeError) as exc:
        raise Refusal("anchors-unreadable", str(exc))
    out = {}
    for c in comments:
        if not trusted_comment(c):
            continue
        anchor = c.get("anchor")
        if anchor is None and isinstance(c.get("body"), str):
            try:
                anchor = json.loads(c["body"])
            except ValueError:
                anchor = None
        if isinstance(anchor, dict) and all(k in anchor for k in ("run_id", "seq", "type", "line_sha256")):
            out[str(c.get("id"))] = anchor
    return locked, out


def anchor_errors(runs, anchors):
    errors = []
    by_run = {r.id: r for r in runs}
    anchored_seqs = {r.id: set() for r in runs}
    for cid, a in anchors.items():
        run = by_run.get(a["run_id"])
        seq = a["seq"]
        if run is None:
            errors.append(f"anchor {cid}: run {a['run_id']} has no file")
        elif not isinstance(seq, int) or not 1 <= seq <= len(run.recs):
            errors.append(f"anchor {cid}: line {seq} of {run.id} is missing")
        elif run.hashes[seq - 1] != a["line_sha256"] or run.recs[seq - 1]["type"] != a["type"]:
            errors.append(f"anchor {cid}: line {seq} of {run.id} differs")
        else:
            anchored_seqs[run.id].add(seq)
    for run in runs:
        for rec in run.recs:
            if rec["type"] != "anchored":
                continue
            a = anchors.get(str(rec.get("anchor_comment_id")))
            if a is None or a["run_id"] != run.id or a["seq"] != rec.get("for_seq") \
                    or a["line_sha256"] != rec.get("for_line_sha256"):
                errors.append(f"{run.id}: anchored line {rec['seq']} names a comment without that anchor")
        seqs = anchored_seqs[run.id]
        if not run.recs:
            continue
        if not seqs:
            errors.append(f"{run.id}: no anchored line")
            continue
        tail = run.recs[max(seqs):]
        if len(tail) > 1 or (tail and tail[0]["type"] != "anchored"):
            errors.append(f"{run.id}: unanchored tail after line {max(seqs)}")
    return errors


# ---------------------------------------------------------------- analysis

class Analysis:
    pass


def analyze(runs):
    A = Analysis()
    A.runs = runs
    A.inits = []
    A.attempts = []
    A.unfinished = []
    A.permanent = []
    A.run_state = {}
    for run in runs:
        pending_init = False
        cur = None
        n = 0
        for i, rec in enumerate(run.recs):
            t = rec["type"]
            if t == "init-intent":
                pending_init = True
            elif t in ("init", "begin-refused"):
                pending_init = False
            if t == "init":
                A.inits.append((run, rec))
            elif t == "attempt-intent":
                n += 1
                cur = {"run": run, "n": rec.get("attempt", n), "intent": rec, "hash": run.hashes[i],
                       "obs": None, "obs_hash": None, "anchored": False, "cls": None}
                A.attempts.append(cur)
            elif t == "anchored" and cur is not None and rec.get("for_seq") == cur["intent"]["seq"]:
                cur["anchored"] = True
            elif t == "attempt-observation" and cur is not None and cur["obs"] is None \
                    and rec.get("attempt") == cur["intent"].get("attempt"):
                cur["obs"] = rec
                cur["obs_hash"] = run.hashes[i]
        if pending_init:
            A.unfinished.append(run.id)
        for rec in run.recs:
            if rec["type"] == "begin-refused" and rec.get("reason") in PERMANENT:
                A.permanent.append((run.id, rec["reason"]))
                break
    A.S0 = A.inits[0][1].get("store_id") if A.inits else None
    A.stable = None
    if A.inits:
        db = obs_db(A.inits[0][1].get("observation") or {})
        if db and isinstance(db.get("identity"), dict):
            A.stable = stable_identity(db["identity"])
    A.store_ids = {r.get("store_id") for _, r in A.inits}
    classify_attempts(A)
    return A


def store_observations(runs, store):
    out = []
    for run in runs:
        for rec in run.recs:
            for field, o in observations_of(rec):
                if store_id_of(o) == store:
                    out.append((rec.get("at", ""), run.id, rec["seq"], rec["type"], o))
    out.sort(key=lambda x: x[:3])
    return out


def highwater(runs, store):
    best = None
    for *_, o in store_observations(runs, store):
        try:
            v = flush_lsn(o) if obs_db(o) else None
        except (KeyError, TypeError, ValueError):
            v = None
        if v is not None and (best is None or v > best):
            best = v
    return best


def common_conditions(I, O1, O2, S):
    try:
        for o in (I, O1, O2):
            if obs_db(o) is None or binding_of(o) != "match" or store_id_of(o) != S or o.get("fence_required") is not True:
                return False
        if not (I["database"]["identity"] == O1["database"]["identity"] == O2["database"]["identity"]):
            return False
        if strip_progress(O1) != strip_progress(O2):
            return False
        if not (flush_lsn(I) <= flush_lsn(O1) <= flush_lsn(O2) and xmax_of(I) <= xmax_of(O1) <= xmax_of(O2)):
            return False
        return in_flight_zero(O1) and in_flight_zero(O2)
    except (KeyError, TypeError, ValueError, AttributeError):
        return False


def raw_classification(att):
    rec = att["obs"]
    I = att["intent"].get("observation")
    O1, O2 = rec.get("observation_1"), rec.get("observation_2")
    S = att["intent"].get("store_id")
    if not common_conditions(I, O1, O2, S):
        return "unknown"
    fl, prof = floor_value(O1), profile_of(O1)
    if prof:
        rev = prof.get("issuer_revision")
        if fl is None or not isinstance(rev, int):
            return "unknown"
        if rev < fl:
            return "witness-ahead"
        key = O1.get("key") if isinstance(O1.get("key"), dict) else {}
        both = all(isinstance(o.get("key"), dict) and o["key"].get("requested") == att["intent"].get("key_id")
                   for o in (O1, O2))
        if rev == fl and not prof.get("revoked") and key.get("trusted") is True and both:
            return "committed"
        return "unknown"
    if fl is None:
        return "failed-before-witness"
    if floor_value(I) is None and profile_of(I) is None and fl == 1:
        return "faw-candidate"
    return "unknown"


def classify_attempts(A):
    observed = sorted((a for a in A.attempts if a["obs"] is not None), key=lambda a: (a["obs"].get("at", ""), a["run"].id, a["obs"]["seq"]))
    base = {id(a): raw_classification(a) for a in observed}
    committed = [a for a in observed if base[id(a)] == "committed"]
    events = {}
    for a in observed:
        S = a["intent"].get("store_id")
        if S not in events:
            events[S] = store_observations(A.runs, S)
    for idx, a in enumerate(observed):
        cls = base[id(a)]
        S = a["intent"].get("store_id")
        earlier = [c for c in committed if c is not a and observed.index(c) < idx and c["intent"].get("store_id") == S]
        O1 = a["obs"].get("observation_1") if isinstance(a["obs"].get("observation_1"), dict) else {}
        if earlier:
            cfloor = floor_value(earlier[-1]["obs"]["observation_1"])
            fl = floor_value(O1)
            if profile_of(O1) is None or fl is None or (cfloor is not None and fl < cfloor):
                cls = "restore-or-loss"
        if cls == "faw-candidate":
            cls = "failed-after-witness" if not [c for c in committed if c is not a and c["intent"].get("store_id") == S] else "unknown"
        if cls == "failed-before-witness":
            key = (a["obs"].get("at", ""), a["run"].id, a["obs"]["seq"])
            for ev in events.get(S, []):
                if ev[:3] <= key:
                    continue
                if ev[3] == "attempt-intent":
                    break
                if floor_value(ev[4]) is not None or profile_of(ev[4]) is not None:
                    cls = "unknown"
                    break
        a["cls"] = cls
    for a in A.attempts:
        if a["obs"] is None:
            a["cls"] = None


def directory_verdict(A, ok):
    if not ok:
        return "unknown"
    if len(A.store_ids) > 1 or A.unfinished:
        return "unknown"
    committed_s0 = any(a["cls"] == "committed" for a in A.attempts)
    reasons = [r for _, r in A.permanent]
    if "evidence-lost" in reasons or "init-lost" in reasons:
        return "unknown"
    if "store-changed" in reasons:
        return "restore-or-loss" if committed_s0 else "unknown"
    classes = [a["cls"] for a in A.attempts if a["cls"]]
    if not classes:
        return "no-attempt"
    for sev in SEVERITY:
        if sev in classes:
            if sev == "committed":
                done = any(r.recs and r.recs[-1]["type"] == "run-close" and r.recs[-1].get("done") is True
                           for r in A.runs if any(a["run"] is r and a["cls"] == "committed" for a in A.attempts))
                return "committed" if done else "self-check-pending"
            return sev
    return "unknown"


def full_errors(runs, foreign, anchors):
    errs = structure_errors(runs, foreign)
    if anchors is None:
        errs.append("anchors not provided")
    else:
        errs += anchor_errors(runs, anchors)
    return errs


# ---------------------------------------------------------------- writing

def fsync_dir(directory):
    fd = os.open(directory, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def make_line(run_id, seq, prev, typ, fields):
    rec = {"v": 1, "run_id": run_id, "seq": seq, "prev": prev, "type": typ, "at": utcnow()}
    for k, v in fields.items():
        if k in rec:
            raise Refusal("reserved-field", k)
        rec[k] = v
    return rec, jdump(rec).encode("utf-8")


def anchor_of(rec, line_hash):
    if rec["type"] == "anchored":
        return None
    return {"run_id": rec["run_id"], "seq": rec["seq"], "type": rec["type"], "line_sha256": line_hash}


def write_all(fd, data):
    if os.write(fd, data) != len(data):
        raise Refusal("short-write")
    os.fsync(fd)


def append_line(path, run_id, typ, fields, expect_last=None):
    fd = os.open(path, os.O_WRONLY | os.O_APPEND)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX)
        run = read_run(path, run_id)
        if run.errors:
            raise Refusal("run-file-broken", "; ".join(run.errors))
        last = run.hashes[-1] if run.hashes else None
        if expect_last is not None and last != expect_last:
            raise Refusal("concurrent-append")
        if stat.S_IMODE(os.fstat(fd).st_mode) != 0o640:
            raise Refusal("run-file-mode", "run file is not mode 0640")
        rec, raw = make_line(run_id, len(run.recs) + 1, last, typ, fields)
        write_all(fd, raw + b"\n")
        h = sha256(raw)
        return rec, h
    finally:
        os.close(fd)


def result_line(rec, h):
    return {"seq": rec["seq"], "type": rec["type"], "line_sha256": h, "anchor": anchor_of(rec, h)}


# ---------------------------------------------------------------- open / append

def find_run(runs, rid):
    for r in runs:
        if r.id == rid:
            return r
    raise Refusal("unknown-run", rid)


def run_init(run):
    for r in run.recs:
        if r["type"] == "init":
            return r
    return None


def with_candidate(runs, run_id, rec, raw_hash="x" * 64):
    out = []
    for r in runs:
        if r.id == run_id:
            c = Run(r.id, r.path)
            c.recs = r.recs + [rec]
            c.hashes = r.hashes + [raw_hash]
            r = c
        out.append(r)
    return out


def blocked_reason(A):
    for _, reason in A.permanent:
        return f"blocked:{reason}"
    if A.unfinished:
        return "blocked:init-unfinished"
    for a in A.attempts:
        if a["cls"] in BLOCKING or a["cls"] in ("committed", "witness-ahead"):
            return f"blocked:{a['cls']}"
    return None


def refusal_record(run, reason, observation, extra=None):
    f = {"reason": reason, "observation": observation}
    init = run_init(run)
    if init is not None:
        f["store_id"] = init["store_id"]
    if extra:
        f.update(extra)
    return f


def cmd_open(a):
    pre = jarg(a.pre_init, "--pre-init")
    location = jarg(a.location, "--location")
    if not isinstance(pre, dict):
        raise Refusal("bad-observation", "--pre-init must be an object")
    if not isinstance(location, dict) or location.get("ok") is not True:
        raise Refusal("location-not-ok", "--location must be a passing check-location result")
    runs, foreign = load_runs(a.dir)
    reason = decide_open(a, runs, foreign, pre)
    rid = str(uuid.uuid4())
    path = os.path.join(a.dir, rid + ".jsonl")
    fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_APPEND, 0o640)
    try:
        try:
            os.fchmod(fd, 0o640)
            st = os.fstat(fd)
            ok = stat.S_IMODE(st.st_mode) == 0o640 and st.st_gid == a.group_gid
        except OSError as exc:
            raise Refusal("run-file-mode", f"fchmod failed: {exc}")
        if not ok:
            raise Refusal("run-file-mode", "run file is not mode 0640 with the evidence group")
        fsync_dir(a.dir)
        rec, raw = make_line(rid, 1, None, "run-open", {
            "store_id": None, "location": location, "pre_init": pre, "cutover_ref": a.cutover_ref})
        write_all(fd, raw + b"\n")
    finally:
        os.close(fd)
    lines = [result_line(rec, sha256(raw))]
    if reason is None:
        return 0, {"ok": True, "run_id": rid, "lines": lines, "anchor": lines[0]["anchor"]}
    rec2, h2 = append_line(path, rid, "begin-refused", {"reason": reason, "observation": pre})
    lines.append(result_line(rec2, h2))
    return 1, {"ok": False, "error": "begin-refused", "reason": reason, "run_id": rid, "lines": lines}


def decide_open(a, runs, foreign, pre):
    try:
        locked, anchors = load_anchors(a.anchors) if a.anchors else (False, None)
    except Refusal:
        locked, anchors = False, None
    if anchors is None:
        return "evidence-lost"
    if not locked:
        return "anchor-issue-unlocked"
    if full_errors(runs, foreign, anchors):
        return "evidence-lost"
    A = analyze(runs)
    if A.unfinished:
        return "init-unfinished"
    if A.S0 is not None:
        sid = store_id_of(pre)
        if binding_of(pre) not in ("marker_only", "match") or sid != A.S0 \
                or not continues(pre, A.stable, highwater(runs, A.S0)):
            return "store-changed"
    if any(at["cls"] == "committed" for at in A.attempts):
        run = next(at["run"] for at in A.attempts if at["cls"] == "committed")
        done = run.recs[-1]["type"] == "run-close" and run.recs[-1].get("done") is True
        return "step4-complete" if done else "self-check-pending"
    if any(at["obs"] is None for at in A.attempts):
        return "attempt-unclosed"
    blocked = blocked_reason(A)
    if blocked:
        return blocked
    if binding_of(pre) not in ("none", "marker_only", "match"):
        return "precondition:binding"
    if floor_value(pre) is not None or floor_temp(pre):
        return "precondition:floor"
    if profile_of(pre) is not None:
        return "precondition:profile"
    return None


def need_dict(data, key):
    v = data.get(key)
    if not isinstance(v, dict):
        raise Refusal("bad-field", f"{key} must be an object")
    return v


def cmd_append(a):
    data = jarg(a.data, "--data") or {}
    if not isinstance(data, dict):
        raise Refusal("bad-json", "--data must be an object")
    if a.comment_id is not None:
        data["anchor_comment_id"] = a.comment_id
    typ = a.type
    runs, foreign = load_runs(a.dir)
    run = find_run(runs, a.run)
    if run.errors or not run.recs:
        raise Refusal("run-file-broken", "; ".join(run.errors) or "empty")
    errs = structure_errors(runs, foreign)
    if errs:
        raise Refusal("verify-failed", errs[0], errors=errs)
    A = analyze(runs)
    last, last_hash = run.recs[-1], run.hashes[-1]
    if last["type"] in ("run-close",):
        raise Refusal("run-closed")
    init = run_init(run)
    fields, outtype = {}, typ
    refused = None

    if typ == "run-open":
        raise Refusal("use-open", "run-open is written by `open`")
    elif typ == "init-intent":
        if last["type"] != "run-open":
            raise Refusal("bad-sequence", "init-intent must follow run-open")
        pre = run.recs[0]["pre_init"]
        fields = {"binding": binding_of(pre), "store_id": store_id_of(pre)}
    elif typ == "anchored":
        if last["type"] not in ("attempt-intent", "init-intent"):
            raise Refusal("bad-sequence", "anchored must follow an intent")
        if not data.get("anchor_comment_id"):
            raise Refusal("anchor-comment-required")
        fields = {"for_seq": last["seq"], "for_line_sha256": last_hash, "anchor_comment_id": data["anchor_comment_id"]}
    elif typ == "init":
        fields, outtype = build_init(A, runs, run, data)
    elif typ == "begin-refused":
        if not isinstance(data.get("reason"), str):
            raise Refusal("bad-field", "reason required")
        fields = {"reason": data["reason"], "observation": data.get("observation")}
    elif typ == "attempt-intent":
        fields, outtype = build_intent(A, runs, run, data)
    elif typ == "attempt-observation":
        fields = build_observation(run, data)
    elif typ == "self-check":
        fields = build_self_check(A, runs, run, data, a)
    elif typ == "run-close":
        fields = build_close(A, run, data)
    else:
        raise Refusal("bad-type", typ)

    if outtype == "begin-refused":
        refused = fields
    if init is not None and outtype not in ("init",):
        fields.setdefault("store_id", init["store_id"])
        if fields["store_id"] != init["store_id"]:
            raise Refusal("store-id-mismatch")
    if outtype in ("attempt-observation", "self-check"):
        intent = last_intent(run)
        fields.update({"scope": intent["scope"], "key_id": intent["key_id"], "attempt": intent["attempt"]})
    if outtype == "attempt-observation":
        probe = dict(make_line(run.id, len(run.recs) + 1, last_hash, outtype, fields)[0])
        cand = analyze(with_candidate(runs, run.id, probe))
        fields["classification"] = next(x["cls"] for x in cand.attempts if x["obs"] is not None and x["obs"] is probe)
    rec, h = append_line(run.path, run.id, outtype, fields, expect_last=last_hash)
    out = {"ok": refused is None, "run_id": run.id, "record": rec, **result_line(rec, h)}
    if refused is not None:
        out["error"] = "begin-refused"
        out["reason"] = refused["reason"]
    return (0 if refused is None else 1), out


def last_intent(run):
    for rec in reversed(run.recs):
        if rec["type"] == "attempt-intent":
            return rec
    raise Refusal("bad-sequence", "no attempt-intent in this run")


def build_init(A, runs, run, data):
    if len(run.recs) < 3 or run.recs[-1]["type"] != "anchored" or run.recs[-2]["type"] != "init-intent":
        raise Refusal("bad-sequence", "init needs an anchored init-intent")
    result, sid, obs = data.get("result"), data.get("store_id"), need_dict(data, "observation")
    if result not in INIT_RESULTS:
        return {"reason": f"init-refused:{result}", "observation": obs}, "begin-refused"
    if not isinstance(sid, str) or sid != store_id_of(obs):
        raise Refusal("store-id-mismatch", "init store_id must equal the observed store id")
    if any(s != sid for s in A.store_ids):
        raise Refusal("store-id-mismatch", "the directory already holds an init for another store")
    pre = run.recs[0]["pre_init"]
    ref = A.stable
    if ref is None and obs_db(pre) and isinstance(obs_db(pre).get("identity"), dict):
        ref = stable_identity(obs_db(pre)["identity"])
    hw = highwater(runs, sid)
    try:
        pre_lsn = flush_lsn(pre) if obs_db(pre) else None
    except (KeyError, TypeError, ValueError):
        pre_lsn = None
    if pre_lsn is not None and (hw is None or pre_lsn > hw):
        hw = pre_lsn
    if binding_of(obs) != "match":
        return {"reason": f"init-refused:binding-{binding_of(obs)}", "observation": obs}, "begin-refused"
    if not continues(obs, ref, hw):
        return {"reason": "store-changed", "observation": obs}, "begin-refused"
    return {"result": result, "store_id": sid, "observation": obs, "late": bool(data.get("late"))}, "init"


def build_intent(A, runs, run, data):
    init = run_init(run)
    last = run.recs[-1]
    if init is None:
        raise Refusal("bad-sequence", "attempt-intent needs an init in this run")
    mine = [x for x in A.attempts if x["run"] is run]
    if last["type"] == "init" and not mine:
        pass
    elif last["type"] == "attempt-observation" and last.get("classification") == "failed-before-witness":
        pass
    elif last["type"] in ("attempt-intent", "anchored") and mine and mine[-1]["obs"] is None:
        raise Refusal("attempt-unobserved", "the previous attempt has no observation")
    else:
        raise Refusal("bad-sequence", f"attempt-intent cannot follow {last['type']}")
    if any(x["obs"] is None for x in A.attempts):
        raise Refusal("attempt-unobserved", "another attempt has no observation")
    blocked = blocked_reason(A)
    if blocked:
        raise Refusal("blocked", blocked)
    obs = need_dict(data, "observation")
    scope = need_dict(data, "scope")
    key_id, sid = data.get("key_id"), data.get("store_id")
    if not isinstance(key_id, str) or not key_id:
        raise Refusal("bad-field", "key_id required")
    if sid != init["store_id"]:
        raise Refusal("store-id-mismatch", "intent store_id must equal the init store id")
    n = len(mine) + 1
    if data.get("attempt", n) != n:
        raise Refusal("bad-field", f"attempt must be {n}")
    base = {"attempt": n, "store_id": sid, "scope": scope, "key_id": key_id,
            "kid_record_count": data.get("kid_record_count"), "observation": obs}
    reason = intent_precondition(A, runs, init, obs, key_id, data)
    if reason:
        return {"reason": reason, "observation": obs}, "begin-refused"
    if "anchor_issue_locked" in data:
        base["anchor_issue_locked"] = data["anchor_issue_locked"]
    return base, "attempt-intent"


def intent_precondition(A, runs, init, obs, key_id, data):
    sid = init["store_id"]
    if data.get("anchor_issue_locked") is False:
        return "anchor-issue-unlocked"
    if binding_of(obs) != "match" or store_id_of(obs) != sid or not continues(obs, A.stable, highwater(runs, sid)):
        return "store-changed"
    if floor_value(obs) is not None or floor_temp(obs):
        return "precondition:floor"
    if profile_of(obs) is not None:
        return "precondition:profile"
    key = obs.get("key") if isinstance(obs.get("key"), dict) else {}
    # in_history and trusted describe key.requested, so the observation must be about the intent's key.
    if key.get("requested") != key_id:
        return "precondition:requested-key"
    if key.get("in_history") is not False:
        return "precondition:key-history"
    if key.get("configured_key_id") not in (key_id, None):
        return "precondition:configured-key"
    if not in_flight_zero(obs):
        return "precondition:in-flight"
    if data.get("kid_record_count") != 1:
        return "precondition:kid-record-count"
    return None


def build_observation(run, data):
    last = run.recs[-1]
    if last["type"] not in ("attempt-intent", "anchored") or last["type"] == "anchored" and run.recs[-2]["type"] != "attempt-intent":
        raise Refusal("bad-sequence", "attempt-observation must follow an attempt-intent")
    return {"deploy_exit": data.get("deploy_exit"), "observation_1": need_dict(data, "observation_1"),
            "observation_2": need_dict(data, "observation_2"), "late": bool(data.get("late"))}


def recheck_ok(A, runs, att, obs):
    S = att["intent"].get("store_id")
    key = obs.get("key") if isinstance(obs.get("key"), dict) else {}
    prof = profile_of(obs)
    fl = floor_value(obs)
    return bool(obs_db(obs) and binding_of(obs) == "match" and store_id_of(obs) == S
                and continues(obs, A.stable, highwater(runs, S)) and prof and fl is not None
                and prof.get("issuer_revision") == fl and not prof.get("revoked") and key.get("trusted") is True
                and key.get("requested") == att["intent"].get("key_id"))


def build_self_check(A, runs, run, data, a):
    last = run.recs[-1]
    att = next((x for x in A.attempts if x["run"] is run and x["obs"] is not None and x["cls"] == "committed"), None)
    late = bool(data.get("late"))
    obs = need_dict(data, "observation")
    if att is None:
        raise Refusal("bad-sequence", "the run has no committed attempt")
    ok_last = last["type"] == "attempt-observation" or (late and last["type"] == "self-check" and last.get("passed") is False)
    if not ok_last:
        raise Refusal("bad-sequence", f"self-check cannot follow {last['type']}")
    if late:
        if not a.anchors:
            raise Refusal("anchors-required", "a late self-check needs --anchors")
        _, anchors = load_anchors(a.anchors)
        problems = full_errors(runs, [], anchors)
        if problems:
            raise Refusal("recheck-refused", "verify --anchors failed", reasons=problems)
        others = [x for x in A.attempts if x is not att and x["cls"] in BLOCKING | {"committed", "witness-ahead"}]
        if A.permanent or A.unfinished or others or any(x["obs"] is None for x in A.attempts):
            raise Refusal("recheck-refused", "a blocking state exists in the directory")
        if not recheck_ok(A, runs, att, obs):
            I = att["intent"]["observation"]
            cls = "unknown"
            if common_conditions(I, obs, obs, att["intent"].get("store_id")):
                cls = raw_classification({"intent": att["intent"], "obs": {"observation_1": obs, "observation_2": obs}})
            raise Refusal("recheck-refused", "observation no longer classifies committed", classification=cls)
    return {"passed": bool(data.get("passed")), "exit_status": data.get("exit_status"),
            "result_line": data.get("result_line"), "observation": obs, "late": late}


def build_close(A, run, data):
    done = bool(data.get("done"))
    last = run.recs[-1]
    if done and not (last["type"] == "self-check" and last.get("passed") is True):
        raise Refusal("bad-sequence", "done=true needs a passing self-check as the last line")
    cls = data.get("classification")
    if cls is None:
        mine = [x["cls"] for x in A.attempts if x["run"] is run and x["cls"]]
        cls = mine[-1] if mine else None
    return {"classification": cls, "done": done}


# ---------------------------------------------------------------- verify / classify / require-open-attempt

def anchors_arg(a):
    if not getattr(a, "anchors", None):
        return None
    return load_anchors(a.anchors)[1]


def cmd_verify(a):
    runs, foreign = load_runs(a.dir)
    anchors = anchors_arg(a)
    errs = structure_errors(runs, foreign)
    if anchors is not None:
        errs += anchor_errors(runs, anchors)
    return (0 if not errs else 1), {"ok": not errs, "errors": errs, "runs": len(runs), "anchors_checked": anchors is not None}


def cmd_classify(a):
    runs, foreign = load_runs(a.dir)
    try:
        anchors = anchors_arg(a)
    except Refusal:
        anchors = None
    errs = full_errors(runs, foreign, anchors)
    A = analyze(runs)
    attempts = [{"run_id": x["run"].id, "attempt": x["n"], "classification": ("unknown" if errs and x["cls"] else x["cls"]),
                 "intent_sha256": x["hash"], "anchored": x["anchored"]} for x in A.attempts]
    return 0, {"ok": not errs, "verify_errors": errs, "attempts": attempts, "directory": directory_verdict(A, not errs)}


def cmd_require_open_attempt(a):
    scope, identity = jarg(a.scope, "--scope"), jarg(a.identity, "--identity")
    runs, foreign = load_runs(a.dir)
    anchors = anchors_arg(a)
    errs = structure_errors(runs, foreign) + (anchor_errors(runs, anchors) if anchors is not None else [])
    if errs:
        raise Refusal("verify-failed", errs[0], errors=errs)
    A = analyze(runs)
    if A.permanent or A.unfinished:
        raise Refusal("blocked", "permanent refusal or unfinished init-intent in the directory")
    if A.store_ids != {a.store}:
        raise Refusal("store-id-mismatch", "every init record must carry the store id")
    for x in A.attempts:
        if x["obs"] is not None and x["cls"] in BLOCKING | {"committed", "witness-ahead"}:
            raise Refusal("blocked", f"attempt classified {x['cls']}")
    open_attempts = [x for x in A.attempts if x["obs"] is None]
    if len(open_attempts) != 1:
        raise Refusal("no-open-attempt", f"{len(open_attempts)} open attempts")
    att = open_attempts[0]
    run = att["run"]
    if runs[-1] is not run:
        raise Refusal("not-newest-run")
    recs = run.recs
    if len(recs) < 2 or recs[-1]["type"] != "anchored" or recs[-2] is not att["intent"] \
            or recs[-1].get("for_seq") != att["intent"]["seq"] or recs[-1].get("for_line_sha256") != att["hash"] \
            or not recs[-1].get("anchor_comment_id"):
        raise Refusal("intent-not-anchored", "the open intent must be followed only by its anchored line")
    intent = att["intent"]
    obs = intent.get("observation") if isinstance(intent.get("observation"), dict) else {}
    if intent.get("store_id") != a.store or intent.get("key_id") != a.key_id or intent.get("scope") != scope:
        raise Refusal("intent-mismatch", "store, key id or scope differ from the intent")
    if binding_of(obs) != "match" or store_id_of(obs) != a.store or floor_value(obs) is not None \
            or profile_of(obs) is not None or obs_db(obs) is None:
        raise Refusal("intent-observation", "the intent observation does not show a clean match")
    if obs_db(obs).get("identity") != identity:
        raise Refusal("identity-mismatch", "the observed identity differs from the intent's")
    if not continues(obs, A.stable, highwater(runs, a.store)):
        raise Refusal("store-changed", "the intent observation does not continue the store")
    return 0, {"run_id": run.id, "intent_sha256": att["hash"]}


# ---------------------------------------------------------------- check-location

def _docker(args):
    if not args or args[0] not in ("inspect", "info", "ps"):
        raise Refusal("docker-not-allowed", "only read-only docker calls are allowed")
    try:
        done = subprocess.run(["docker", *args], capture_output=True, text=True, timeout=60, check=True)
    except (OSError, subprocess.SubprocessError) as exc:
        raise Refusal("docker-failed", f"docker {args[0]}: {exc}")
    return done.stdout


def _json_file(path):
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def _unescape(s):
    return re.sub(r"\\([0-7]{3})", lambda m: chr(int(m.group(1), 8)), s)


def parse_mountinfo(text):
    out = []
    for line in text.splitlines():
        f = line.split(" ")
        if len(f) < 10 or "-" not in f:
            continue
        out.append({"dev": f[2], "root": _unescape(f[3]), "mp": _unescape(f[4])})
    return out


def canonical_from(source, root):
    if not source.startswith("/") or not source.split("/")[1]:
        return None
    return "/" + source.split("/")[1] + ("" if root == "/" else root)


def overlaps(x, y):
    x, y = posixpath.normpath(x), posixpath.normpath(y)
    return x == y or x.startswith(y.rstrip("/") + "/") or y.startswith(x.rstrip("/") + "/")


def share_of(path):
    return "/".join(posixpath.normpath(path).split("/")[:3])


def cmd_check_location(a):
    target = a.target or a.dir
    errors = []
    runner = a.runner_container or socket.gethostname()
    if a.runner_inspect:
        runner_doc = _json_file(a.runner_inspect)
    else:
        runner_doc = json.loads(_docker(["inspect", runner]))
    runner_doc = runner_doc[0] if isinstance(runner_doc, list) else runner_doc
    if a.project_inspect:
        project = _json_file(a.project_inspect)
    else:
        ids = _docker(["ps", "-aq", "--filter", f"label=com.docker.compose.project={a.project}"]).split()
        project = json.loads(_docker(["inspect", *ids])) if ids else []
    docker_info = _json_file(a.docker_info) if a.docker_info else json.loads(_docker(["info", "--format", "{{json .}}"]))
    docker_root = docker_info.get("DockerRootDir") or ""
    if a.mountinfo:
        with open(a.mountinfo, "r", encoding="utf-8") as handle:
            minfo = parse_mountinfo(handle.read())
    else:
        with open("/proc/self/mountinfo", "r", encoding="utf-8") as handle:
            minfo = parse_mountinfo(handle.read())

    def mi(dest):
        hits = [e for e in minfo if e["mp"] == dest]
        return hits[-1] if hits else None

    mounts = runner_doc.get("Mounts") or []
    ev = next((m for m in mounts if m.get("Destination") == target), None)
    canonical = ev_dev = source = None
    if ev is None:
        errors.append("mount-missing")
    else:
        source = ev.get("Source", "")
        if ev.get("Type") != "bind":
            errors.append("mount-not-bind")
        entry = mi(target)
        if entry is None:
            errors.append("mountinfo-missing")
        else:
            ev_dev = entry["dev"]
            canonical = canonical_from(source, entry["root"])
            if canonical is None or canonical != source:
                errors.append("source-not-canonical")
        if canonical is None or "source-not-canonical" in errors:
            canonical = None
    if canonical:
        if docker_root and overlaps(canonical, docker_root):
            errors.append("docker-root")
        for m in mounts:
            if m is ev:
                continue
            dest = m.get("Destination", "")
            src = m.get("Source", "")
            entry = mi(dest) if m.get("Type") == "bind" else None
            path = (canonical_from(src, entry["root"]) if entry else None) or src
            dev = entry["dev"] if entry else None
            if dev is not None and ev_dev is not None and dev != ev_dev:
                continue
            if overlaps(canonical, path):
                errors.append(f"overlap:{dest}")
            elif share_of(canonical) == share_of(path):
                errors.append(f"same-share:{dest}")
        for c in project:
            if c.get("Id") and c.get("Id") == runner_doc.get("Id"):
                continue
            service = (c.get("Config") or {}).get("Labels", {}).get("com.docker.compose.service")
            for m in c.get("Mounts") or []:
                if service == a.platform_service and m.get("Source") == source and m.get("Destination") == target \
                        and m.get("RW") is False:
                    continue
                path = m.get("Source", "")
                name = c.get("Name") or service or "?"
                if overlaps(canonical, path):
                    errors.append(f"overlap:{name}:{m.get('Destination')}")
                elif share_of(canonical) == share_of(path):
                    errors.append(f"same-share:{name}:{m.get('Destination')}")
    # directory checks (read only)
    append_only = None
    try:
        st = os.stat(a.dir)
    except OSError as exc:
        raise Refusal("directory-unreadable", str(exc))
    if st.st_uid != a.owner_uid:
        errors.append("directory-owner")
    if st.st_gid != a.group_gid:
        errors.append("directory-group")
    if stat.S_IMODE(st.st_mode) != int(str(a.dir_mode), 8):
        errors.append("directory-mode")
    if not os.access(a.dir, os.W_OK | os.X_OK):
        errors.append("not-writable")
    if os.statvfs(a.dir).f_flag & os.ST_RDONLY:
        errors.append("read-only-filesystem")
    try:
        import array
        fd = os.open(a.dir, os.O_RDONLY | os.O_DIRECTORY)
        try:
            buf = array.array("l", [0])
            fcntl.ioctl(fd, FS_IOC_GETFLAGS, buf, True)
            append_only = bool(buf[0] & FS_APPEND_FL)
        finally:
            os.close(fd)
    except (OSError, ImportError, OverflowError):
        append_only = None
    for name in sorted(os.listdir(a.dir)):
        path = os.path.join(a.dir, name)
        if RUN_FILE.match(name) and stat.S_ISREG(os.lstat(path).st_mode):
            fst = os.lstat(path)
            if stat.S_IMODE(fst.st_mode) != 0o640:
                errors.append(f"run-file-mode:{name}")
            if fst.st_gid != a.group_gid:
                errors.append(f"run-file-group:{name}")
        elif name in SYSTEM_DIRS:
            continue
        else:
            errors.append(f"foreign-entry:{name}")
    out = {"ok": not errors, "errors": errors, "path": target, "canonical_source": canonical,
           "share": share_of(canonical) if canonical else None, "dir_append_only": append_only}
    return (0 if not errors else 1), out


# ---------------------------------------------------------------- CLI

def build_parser():
    p = argparse.ArgumentParser(prog="attempt_evidence.py", description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="cmd", required=True)

    c = sub.add_parser("check-location")
    c.add_argument("--dir", required=True)
    c.add_argument("--target")
    c.add_argument("--runner-container")
    c.add_argument("--project", default=os.environ.get("COMPOSE_PROJECT_NAME") or "oteryn-staging")
    c.add_argument("--platform-service", default="platform")
    c.add_argument("--owner-uid", type=int, default=0)
    c.add_argument("--group-gid", type=int, default=82)
    c.add_argument("--dir-mode", default="2750")
    c.add_argument("--runner-inspect")
    c.add_argument("--project-inspect")
    c.add_argument("--docker-info")
    c.add_argument("--mountinfo")

    o = sub.add_parser("open")
    o.add_argument("--dir", required=True)
    o.add_argument("--anchors")
    o.add_argument("--pre-init", required=True)
    o.add_argument("--location", required=True)
    o.add_argument("--cutover-ref")
    o.add_argument("--group-gid", type=int, default=82)

    ap = sub.add_parser("append")
    ap.add_argument("type")
    ap.add_argument("--dir", required=True)
    ap.add_argument("--run", required=True)
    ap.add_argument("--data")
    ap.add_argument("--comment-id")
    ap.add_argument("--anchors")

    for name in ("verify", "classify"):
        v = sub.add_parser(name)
        v.add_argument("--dir", required=True)
        v.add_argument("--anchors")

    r = sub.add_parser("require-open-attempt")
    r.add_argument("--dir", required=True)
    r.add_argument("--store", required=True)
    r.add_argument("--key-id", required=True)
    r.add_argument("--scope", required=True)
    r.add_argument("--identity", required=True)
    r.add_argument("--anchors")
    return p


HANDLERS = {
    "check-location": cmd_check_location, "open": cmd_open, "append": cmd_append, "verify": cmd_verify,
    "classify": cmd_classify, "require-open-attempt": cmd_require_open_attempt,
}


def main(argv=None, out=None):
    out = out or sys.stdout
    args = build_parser().parse_args(argv)
    try:
        code, payload = HANDLERS[args.cmd](args)
    except Refusal as exc:
        code = 1
        payload = {"ok": False, "error": exc.code, "message": exc.message, **exc.extra}
        if args.cmd == "require-open-attempt":
            sys.stderr.write(f"attempt_evidence: require-open-attempt refused: {exc.code}: {exc.message}\n")
            return 1
    except OSError as exc:
        code = 1
        payload = {"ok": False, "error": "io-error", "message": str(exc)}
        if args.cmd == "require-open-attempt":
            sys.stderr.write(f"attempt_evidence: require-open-attempt failed: {exc}\n")
            return 1
    out.write(json.dumps(payload, sort_keys=True) + "\n")
    out.flush()
    return code


if __name__ == "__main__":
    sys.exit(main())
