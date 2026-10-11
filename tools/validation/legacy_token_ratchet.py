#!/usr/bin/env python3
"""Ratchet guard: the legacy-engine token count per group may only fall.

The token regex is written with a bracketed letter so this file never matches
itself. Counts are matching lines (``git grep -ic``) per path group, plus one
group counting tracked path names (``git ls-files``). The baseline lives in
``docs/agents/LEGACY_TOKEN_BASELINE.json``.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

PATTERN = "c[a]nary"
PATHS_GROUP = "paths"
DEFAULT_BASELINE = "docs/agents/LEGACY_TOKEN_BASELINE.json"
TOKEN_RE = re.compile(PATTERN, re.IGNORECASE)

# First matching prefix wins; a prefix ending in "/" matches a directory tree,
# any other prefix matches that exact path. Unmatched files fall into "other".
DEFAULT_RULES: list[tuple[str, str]] = [
    ("docs/agents/tasks/archive/", "history"),
    ("docs/agents/reports/", "history"),
    ("docs/agents/evidence/", "history"),
    ("docs/agents/handovers/", "history"),
    ("docs/evidence/", "history"),
    ("CHANGELOG.md", "history"),
    ("config/", "config"),
    (".env.example", "config"),
    ("deploy/synology/.env.example", "config"),
    ("database/", "database"),
    ("tests/ci/", "ci_deploy"),
    (".github/", "ci_deploy"),
    ("scripts/", "ci_deploy"),
    ("deploy/", "ci_deploy"),
    ("ops/", "ci_deploy"),
    ("tests/", "tests"),
    ("app/", "runtime"),
    ("resources/", "runtime"),
    ("routes/", "runtime"),
    ("lang/", "runtime"),
    ("services/", "runtime"),
    ("bootstrap/", "runtime"),
    ("public/", "runtime"),
    ("docs/", "docs"),
    ("README.md", "docs"),
    ("THIRD_PARTY_NOTICES.md", "docs"),
]
OTHER_GROUP = "other"

# Task records must name their real head branch (checked against the pull
# request by tools/agents), and that name can contain the token. Only lines
# holding exactly a branch identifier under these keys, inside task records,
# are not counted; free-form prose never is.
TASK_RECORD_PREFIX = "docs/agents/tasks/"
EXEMPT_LINE_RE = re.compile(r"^(branch|source_branch): [A-Za-z0-9._/-]+$")


def exempt_lines(root: Path, path: str) -> int:
    """Count token lines of a task record that are exempt branch-name lines."""
    if not path.startswith(TASK_RECORD_PREFIX):
        return 0
    try:
        text = (root / path).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return 0
    return sum(
        1
        for line in text.splitlines()
        if TOKEN_RE.search(line) and EXEMPT_LINE_RE.match(line.rstrip())
    )


class RatchetError(Exception):
    pass


def git(root: Path, *args: str) -> bytes:
    proc = subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, check=False
    )
    if proc.returncode not in (0, 1):
        raise RatchetError(proc.stderr.decode("utf-8", "replace").strip())
    return proc.stdout


def group_of(path: str, rules: list[tuple[str, str]]) -> str:
    for prefix, group in rules:
        if path == prefix or (prefix.endswith("/") and path.startswith(prefix)):
            return group
    return OTHER_GROUP


def measure(root: Path, rules: list[tuple[str, str]]) -> dict[str, dict[str, int]]:
    """Return {group: {"count": lines, "files": files}} for the tracked tree."""
    groups = sorted({g for _, g in rules} | {OTHER_GROUP})
    result = {g: {"count": 0, "files": 0} for g in groups}
    out = git(root, "grep", "-I", "-i", "-c", "-z", "-e", PATTERN)
    for record in out.split(b"\n"):
        if not record:
            continue
        raw_path, _, raw_count = record.rpartition(b"\0")
        path, count = raw_path.decode("utf-8", "surrogateescape"), int(raw_count)
        count -= exempt_lines(root, path)
        if count <= 0:
            continue
        entry = result[group_of(path, rules)]
        entry["count"] += count
        entry["files"] += 1
    names = git(root, "ls-files", "-z").split(b"\0")
    hits = [n for n in names if n and TOKEN_RE.search(n.decode("utf-8", "surrogateescape"))]
    result[PATHS_GROUP] = {"count": len(hits), "files": len(hits)}
    return result


def load_baseline(path: Path) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise RatchetError(f"cannot read baseline {path}: {exc}") from exc
    if data.get("version") != 1 or not isinstance(data.get("groups"), dict):
        raise RatchetError(f"baseline {path} has an unsupported shape")
    if data.get("pattern") != PATTERN:
        raise RatchetError(f"baseline pattern must be {PATTERN!r}")
    return data


def rules_from(baseline: dict) -> list[tuple[str, str]]:
    return [(str(p), str(g)) for p, g in baseline.get("rules", [])]


def compare(baseline: dict, current: dict[str, dict[str, int]]) -> tuple[list[str], list[str]]:
    """Return (failures, notes). A rise in an enforced group is a failure."""
    failures: list[str] = []
    notes: list[str] = []
    for name in sorted(set(baseline["groups"]) | set(current)):
        base = baseline["groups"].get(name)
        now = current.get(name, {"count": 0})["count"]
        if base is None:
            if now:
                failures.append(f"{name}: new group with {now} occurrences, not in baseline")
            continue
        allowed = int(base["count"])
        enforce = bool(base.get("enforce", True))
        if now > allowed:
            line = f"{name}: {now} > baseline {allowed} (+{now - allowed})"
            (failures if enforce else notes).append(
                line if enforce else f"report-only {line}"
            )
        elif now < allowed:
            notes.append(
                f"{name}: {now} < baseline {allowed}; lower the baseline with --write"
            )
    return failures, notes


def build_baseline(
    current: dict[str, dict[str, int]],
    rules: list[tuple[str, str]],
    previous: dict | None,
) -> dict:
    old = (previous or {}).get("groups", {})
    return {
        "version": 1,
        "pattern": PATTERN,
        "case_insensitive": True,
        "metric": "matching lines (git grep -ic) per group; paths = tracked path names (git ls-files)",
        "notes": list((previous or {}).get("notes", [])),
        "rules": [list(r) for r in rules],
        "groups": {
            name: {
                "count": data["count"],
                "files": data["files"],
                "enforce": bool(old.get(name, {}).get("enforce", True)),
            }
            for name, data in sorted(current.items())
        },
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--baseline", default=DEFAULT_BASELINE)
    parser.add_argument(
        "--write",
        action="store_true",
        help="rewrite the baseline from the tree; refuses to raise any group",
    )
    args = parser.parse_args(argv)
    root = Path(args.root)
    baseline_path = root / args.baseline
    try:
        if args.write and not baseline_path.exists():
            current = measure(root, DEFAULT_RULES)
            baseline_path.write_text(
                json.dumps(build_baseline(current, DEFAULT_RULES, None), indent=2) + "\n",
                encoding="utf-8",
            )
            print(f"wrote initial baseline {args.baseline}")
            return 0
        baseline = load_baseline(baseline_path)
        rules = rules_from(baseline)
        current = measure(root, rules)
        failures, notes = compare(baseline, current)
        for note in notes:
            print(f"note: {note}")
        if failures:
            for failure in failures:
                print(f"FAIL: {failure}", file=sys.stderr)
            return 1
        if args.write:
            baseline_path.write_text(
                json.dumps(build_baseline(current, rules, baseline), indent=2) + "\n",
                encoding="utf-8",
            )
            print(f"rewrote baseline {args.baseline}")
            return 0
    except RatchetError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 2
    summary = ", ".join(f"{k}={v['count']}" for k, v in sorted(current.items()))
    print(f"legacy token ratchet ok: {summary}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
