#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

import checkpoint

HEADING = "## Source branch closeout"
REQUIRED_FIELDS = {
    "source_branch_disposition",
    "source_branch_reason",
    "source_branch_evidence",
}
ALLOWED_TERMINAL_DISPOSITIONS = {
    "auto_delete_after_merge",
    "delete_on_close",
    "retain",
}
PLACEHOLDER_VALUES = {
    "",
    "none",
    "unknown",
    "pending",
    "n/a",
    "tbd",
    "todo",
    "later",
}


class SourceBranchCloseoutError(ValueError):
    pass


def extract_closeout_block(text: str, *, source: Path | None = None) -> str | None:
    location = str(source) if source else "<text>"
    matches = list(re.finditer(r"(?m)^## Source branch closeout\s*$", text))
    if not matches:
        return None
    if len(matches) != 1:
        raise SourceBranchCloseoutError(
            f"{location}: expected exactly one {HEADING} section; found {len(matches)}"
        )
    remainder = text[matches[0].end() :]
    next_heading = re.search(r"(?m)^##\s+", remainder)
    section = remainder[: next_heading.start()] if next_heading else remainder
    fences = list(re.finditer(r"```(?:yaml|yml)\s*\n", section, flags=re.IGNORECASE))
    if len(fences) != 1:
        raise SourceBranchCloseoutError(
            f"{location}: {HEADING} must contain exactly one fenced YAML block"
        )
    start = fences[0].end()
    end = section.find("```", start)
    if end < 0:
        raise SourceBranchCloseoutError(f"{location}: source branch closeout fence is not closed")
    return section[start:end].strip("\n")


def parse_closeout_block(block: str, *, source: Path | None = None) -> dict[str, str]:
    location = str(source) if source else "<source-branch-closeout>"
    values: dict[str, str] = {}
    for lineno, raw in enumerate(block.splitlines(), start=1):
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        if raw != raw.lstrip():
            raise SourceBranchCloseoutError(
                f"{location}:{lineno}: source branch closeout fields must be top-level scalars"
            )
        if ":" not in raw:
            raise SourceBranchCloseoutError(
                f"{location}:{lineno}: invalid source branch closeout line"
            )
        key, value = raw.split(":", 1)
        key = key.strip()
        value = value.strip()
        if key in values:
            raise SourceBranchCloseoutError(
                f"{location}:{lineno}: duplicate source branch closeout field {key!r}"
            )
        values[key] = value
    unknown = sorted(set(values) - REQUIRED_FIELDS)
    if unknown:
        raise SourceBranchCloseoutError(
            f"{location}: unsupported source branch closeout fields: {', '.join(unknown)}"
        )
    return values


def _is_placeholder(value: str) -> bool:
    normalized = " ".join(value.casefold().split())
    return normalized in PLACEHOLDER_VALUES or normalized.startswith("pending ")


def validate_closeout_text(text: str, *, source: Path | None = None) -> list[str]:
    location = str(source) if source else "<text>"
    try:
        block = extract_closeout_block(text, source=source)
        if block is None:
            return [f"{location}: missing {HEADING} section"]
        values = parse_closeout_block(block, source=source)
    except SourceBranchCloseoutError as exc:
        return [str(exc)]

    errors: list[str] = []
    missing = sorted(REQUIRED_FIELDS - set(values))
    for field in missing:
        errors.append(f"{location}: missing source branch closeout field {field}")

    disposition = values.get("source_branch_disposition", "").strip()
    if disposition not in ALLOWED_TERMINAL_DISPOSITIONS:
        errors.append(
            f"{location}: source_branch_disposition must be terminal: "
            f"{', '.join(sorted(ALLOWED_TERMINAL_DISPOSITIONS))}"
        )

    for field in ("source_branch_reason", "source_branch_evidence"):
        value = values.get(field, "").strip()
        if _is_placeholder(value):
            errors.append(f"{location}: {field} must contain non-pending terminal evidence")
    return errors


def validate_closeout_file(path: Path) -> list[str]:
    try:
        return validate_closeout_text(path.read_text(encoding="utf-8"), source=path)
    except OSError as exc:
        return [f"{path}: cannot read task record: {exc}"]


def validate_completed_active_tasks(active_root: Path) -> tuple[int, list[str]]:
    checked = 0
    errors: list[str] = []
    if not active_root.exists():
        return checked, errors
    for path in sorted(active_root.glob("*.md")):
        if path.name.casefold() == "readme.md":
            continue
        try:
            parsed = checkpoint.parse_task_checkpoint(path)
        except (OSError, checkpoint.CheckpointError) as exc:
            errors.append(str(exc))
            continue
        if parsed is None or str(parsed.data.get("status", "")).strip() != "completed":
            continue
        checked += 1
        errors.extend(validate_closeout_file(path))
    return checked, errors


def changed_archive_files(
    archive_root: Path, base_sha: str, *, repository_root: Path | None = None
) -> list[Path]:
    repo = (repository_root or Path.cwd()).resolve()
    archive = archive_root.resolve()
    try:
        archive_rel = archive.relative_to(repo)
    except ValueError as exc:
        raise SourceBranchCloseoutError(
            f"archive directory {archive} is outside repository root {repo}"
        ) from exc
    process = subprocess.run(
        [
            "git",
            "diff",
            "--name-only",
            "--diff-filter=AMR",
            base_sha,
            "HEAD",
            "--",
            archive_rel.as_posix(),
        ],
        cwd=repo,
        text=True,
        capture_output=True,
        check=False,
    )
    if process.returncode != 0:
        raise SourceBranchCloseoutError(
            f"git diff for changed archive tasks failed: {process.stderr.strip()}"
        )
    result: list[Path] = []
    for raw in process.stdout.splitlines():
        rel = Path(raw.strip())
        if not raw.strip() or rel.suffix != ".md" or rel.name.casefold() == "readme.md":
            continue
        path = (repo / rel).resolve()
        try:
            path.relative_to(archive)
        except ValueError:
            raise SourceBranchCloseoutError(
                f"changed archive path escaped archive root: {rel.as_posix()}"
            )
        if path.is_file():
            result.append(path)
    return sorted(set(result))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Enforce terminal source-branch disposition for completed/newly archived tasks."
    )
    parser.add_argument("--active", type=Path, default=Path("docs/agents/tasks/active"))
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--archive-changed-from")
    args = parser.parse_args(argv)

    checked_active, errors = validate_completed_active_tasks(args.active)
    checked_archive = 0

    if bool(args.archive) != bool(args.archive_changed_from):
        parser.error("--archive and --archive-changed-from must be supplied together")
    if args.archive and args.archive_changed_from:
        try:
            archive_paths = changed_archive_files(args.archive, args.archive_changed_from)
        except SourceBranchCloseoutError as exc:
            errors.append(str(exc))
            archive_paths = []
        checked_archive = len(archive_paths)
        for path in archive_paths:
            errors.extend(validate_closeout_file(path))

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(
        "Validated terminal source-branch closeout: "
        f"{checked_active} completed active task(s), {checked_archive} changed archive task(s)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
