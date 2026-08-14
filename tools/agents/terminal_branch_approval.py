#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

import branch_lifecycle
import terminal_branch_cleanup as terminal

SCHEMA_VERSION = 1
ISSUE_NUMBER = 1050
CONFIRMATION = "APPROVE_REVIEWED_TERMINAL_CLOSED_UNMERGED_BRANCHES_ISSUE_1050"
EXPECTED_FIELDS = {
    "schema_version",
    "issue",
    "apply_on_main",
    "confirmation",
    "candidate_count",
    "entries_sha256",
    "policy_sha256",
    "source_artifact",
    "reviewed_at",
    "reviewed_by",
    "review_summary",
}

ValidationError = branch_lifecycle.ValidationError


def _text(value: Any, field: str) -> str:
    return branch_lifecycle._text(value, field)


def entries_sha256(entries: Any) -> str:
    if not isinstance(entries, list):
        raise ValidationError("terminal generated manifest.entries: expected array")
    return hashlib.sha256(branch_lifecycle.canonical_json(entries).encode()).hexdigest()


def validate_approval(
    approval: dict[str, Any],
    manifest: dict[str, Any],
    report: dict[str, Any],
    *,
    require_apply: bool,
) -> dict[str, Any]:
    branch_lifecycle._exact_fields(approval, EXPECTED_FIELDS, "terminal approval")
    if approval["schema_version"] != SCHEMA_VERSION:
        raise ValidationError("terminal approval.schema_version: unsupported")
    if approval["issue"] != ISSUE_NUMBER:
        raise ValidationError(f"terminal approval.issue: expected {ISSUE_NUMBER}")
    if approval["confirmation"] != CONFIRMATION:
        raise ValidationError("terminal approval.confirmation: exact phrase required")
    if require_apply and approval["apply_on_main"] is not True:
        raise ValidationError("terminal approval.apply_on_main must be true for apply")
    if not isinstance(approval["candidate_count"], int) or isinstance(approval["candidate_count"], bool):
        raise ValidationError("terminal approval.candidate_count: expected integer")
    if approval["candidate_count"] < 1:
        raise ValidationError("terminal approval.candidate_count must be positive")
    expected_entries_hash = _text(approval["entries_sha256"], "terminal approval.entries_sha256")
    expected_policy_hash = _text(approval["policy_sha256"], "terminal approval.policy_sha256")
    _text(approval["source_artifact"], "terminal approval.source_artifact")
    _text(approval["reviewed_at"], "terminal approval.reviewed_at")
    _text(approval["reviewed_by"], "terminal approval.reviewed_by")
    _text(approval["review_summary"], "terminal approval.review_summary")

    validated_entries = terminal.validate_manifest(manifest, report, require_apply=False)
    actual_entries_hash = entries_sha256(validated_entries)
    if approval["candidate_count"] != len(validated_entries):
        raise ValidationError("terminal approval candidate count drift")
    if expected_entries_hash != actual_entries_hash:
        raise ValidationError("terminal approval candidate entries drift")
    if expected_policy_hash != manifest.get("policy_sha256"):
        raise ValidationError("terminal approval policy drift")
    if expected_policy_hash != report.get("policy_sha256"):
        raise ValidationError("terminal approval report policy drift")

    return {
        "apply_on_main": approval["apply_on_main"],
        "candidate_count": len(validated_entries),
        "entries_sha256": actual_entries_hash,
        "issue": ISSUE_NUMBER,
        "policy_sha256": expected_policy_hash,
        "result": "PASS",
        "schema_version": 1,
    }


def materialize_runtime_manifest(
    approval: dict[str, Any],
    manifest: dict[str, Any],
    report: dict[str, Any],
) -> dict[str, Any]:
    validate_approval(approval, manifest, report, require_apply=True)
    runtime = dict(manifest)
    runtime["apply_on_main"] = True
    terminal.validate_manifest(runtime, report, require_apply=True)
    return runtime


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser(description="Validate reviewed terminal branch candidate digest")
    value.add_argument("--approval", type=Path, required=True)
    value.add_argument("--manifest", type=Path, required=True)
    value.add_argument("--report", type=Path, required=True)
    value.add_argument("--output", type=Path, required=True)
    value.add_argument("--materialize", type=Path)
    value.add_argument("--require-apply", action="store_true")
    return value


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    approval_raw = args.approval.read_text(encoding="utf-8")
    approval = json.loads(approval_raw)
    if approval_raw != branch_lifecycle.canonical_json(approval):
        raise ValidationError("terminal approval JSON is not canonical")
    manifest = branch_lifecycle.load_json(args.manifest)
    report = branch_lifecycle.load_json(args.report)
    result = validate_approval(
        approval,
        manifest,
        report,
        require_apply=args.require_apply,
    )
    args.output.write_text(branch_lifecycle.canonical_json(result), encoding="utf-8")
    if args.materialize is not None:
        runtime = materialize_runtime_manifest(approval, manifest, report)
        args.materialize.write_text(branch_lifecycle.canonical_json(runtime), encoding="utf-8")
    print(
        f"Validated reviewed terminal candidate set: {result['candidate_count']} branches"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValidationError, json.JSONDecodeError, OSError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
