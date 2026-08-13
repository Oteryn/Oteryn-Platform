#!/usr/bin/env python3
"""Fail closed when external GitHub Actions references are not immutable SHAs."""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

USES_RE = re.compile(r"^\s*(?:-\s*)?uses\s*:\s*(.*?)\s*$")
FULL_SHA_RE = re.compile(r"^[0-9a-fA-F]{40}$")
EXTERNAL_TARGET_RE = re.compile(
    r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*$"
)
EXCLUDED_ACTION_DIRS = {".git", "vendor", "node_modules"}


@dataclass(frozen=True)
class Finding:
    path: str
    line: int
    value: str
    kind: str
    message: str


def _strip_comment(raw: str) -> str:
    raw = raw.strip()
    if not raw:
        return raw
    if raw[0] in "\"'":
        quote = raw[0]
        end = raw.find(quote, 1)
        if end == -1:
            return raw
        return raw[: end + 1]
    return raw.split(" #", 1)[0].strip()


def _unquote(raw: str) -> str:
    raw = raw.strip()
    if len(raw) >= 2 and raw[0] == raw[-1] and raw[0] in "\"'":
        return raw[1:-1]
    return raw


def classify_uses(value: str) -> tuple[str, str | None]:
    if value.startswith("./"):
        return "local", None
    if value.startswith("docker://"):
        return "docker", None
    if "@" not in value:
        return "malformed", "external uses reference must contain @<ref>"
    target, ref = value.rsplit("@", 1)
    if not target or not ref or not EXTERNAL_TARGET_RE.fullmatch(target):
        return "malformed", "external uses reference must match owner/repo[/path]@ref"
    if not FULL_SHA_RE.fullmatch(ref):
        return "mutable", "external uses reference must use a full 40-character commit SHA"
    return "external_sha", None


def iter_source_files(root: Path) -> list[Path]:
    files: set[Path] = set()
    workflows = root / ".github" / "workflows"
    if workflows.is_dir():
        files.update(p for p in workflows.rglob("*") if p.suffix in {".yml", ".yaml"})
    for name in ("action.yml", "action.yaml"):
        for path in root.rglob(name):
            if not EXCLUDED_ACTION_DIRS.intersection(path.relative_to(root).parts):
                files.add(path)
    return sorted(files)


def scan_files(files: list[Path], display_root: Path | None = None) -> tuple[list[dict], list[Finding]]:
    inventory: list[dict] = []
    findings: list[Finding] = []
    for path in files:
        text = path.read_text(encoding="utf-8")
        for line_no, line in enumerate(text.splitlines(), 1):
            if line.lstrip().startswith("#"):
                continue
            match = USES_RE.match(line)
            if not match:
                continue
            raw = _strip_comment(match.group(1))
            value = _unquote(raw)
            kind, error = classify_uses(value)
            shown = str(path.relative_to(display_root)) if display_root else str(path)
            inventory.append({"path": shown, "line": line_no, "uses": value, "kind": kind})
            if error:
                findings.append(Finding(shown, line_no, value, kind, error))
    return inventory, findings


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--inventory-json", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    files = iter_source_files(root)
    inventory, findings = scan_files(files, root)
    if args.inventory_json:
        print(json.dumps(inventory, indent=2, sort_keys=True))
    if findings:
        for finding in findings:
            print(
                f"{finding.path}:{finding.line}: {finding.message}: {finding.value}",
                file=sys.stderr,
            )
        print(f"Rejected {len(findings)} mutable or malformed external uses reference(s).", file=sys.stderr)
        return 1
    print(f"Validated {len(inventory)} uses reference(s) across {len(files)} workflow/action file(s).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
