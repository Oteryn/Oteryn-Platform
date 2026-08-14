#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any

import branch_lifecycle
import terminal_branch_cleanup_legacy as legacy
from terminal_branch_cleanup_legacy import *  # noqa: F401,F403

DISPOSITION_RE = re.compile(r"^\s*Branch-Disposition:\s*(delete|retain)\s*$", re.I | re.M)
REASON_RE = re.compile(r"^\s*Branch-Disposition-Reason:\s*(\S.*)\s*$", re.I | re.M)

ValidationError = branch_lifecycle.ValidationError
ApiError = branch_lifecycle.ApiError
_ORIGINAL_CLASSIFY_SNAPSHOT = legacy.classify_snapshot
_ORIGINAL_REVALIDATE_DELETE_ENTRY = legacy._revalidate_delete_entry


def parse_disposition_metadata(body: str) -> tuple[str | None, str | None, str | None]:
    dispositions = [value.casefold() for value in DISPOSITION_RE.findall(body)]
    reasons = [value.strip() for value in REASON_RE.findall(body)]

    if not dispositions and not reasons:
        return None, None, None
    if len(dispositions) != 1:
        if set(dispositions) == {"delete", "retain"}:
            return None, None, "conflicting Branch-Disposition markers"
        return None, None, "expected exactly one Branch-Disposition marker"
    if len(reasons) != 1 or not reasons[0]:
        if not reasons:
            return (
                None,
                None,
                f"Branch-Disposition: {dispositions[0]} requires Branch-Disposition-Reason",
            )
        return None, None, "expected exactly one non-empty Branch-Disposition-Reason"
    return dispositions[0], reasons[0], None


def _pull_by_number(snapshot: dict[str, Any], number: int) -> dict[str, Any] | None:
    pulls = snapshot.get("pulls")
    if not isinstance(pulls, list):
        raise ValidationError("snapshot.pulls must be an array")
    matches = [
        pull
        for pull in pulls
        if isinstance(pull, dict)
        and pull.get("number") == number
        and not isinstance(pull.get("number"), bool)
    ]
    if len(matches) != 1:
        return None
    return matches[0]


def apply_historical_retention_guard(
    report: dict[str, Any], snapshot: dict[str, Any]
) -> dict[str, Any]:
    guarded = dict(report)
    branches: list[dict[str, Any]] = []

    for raw in report.get("branches", []):
        item = dict(raw)
        if item.get("classification") != "TERMINAL_CLOSED_UNMERGED":
            branches.append(item)
            continue

        pr_evidence = item.get("closed_unmerged_pr")
        number = pr_evidence.get("number") if isinstance(pr_evidence, dict) else None
        if not isinstance(number, int) or isinstance(number, bool):
            item["classification"] = "UNMERGED_ORPHAN"
            item["deletion_candidate"] = False
            item["evidence"] = [
                "terminal closed-unmerged candidate lacks one exact pull-request identity; historical deletion fails closed"
            ]
            branches.append(item)
            continue

        pull = _pull_by_number(snapshot, number)
        if pull is None:
            item["classification"] = "UNMERGED_ORPHAN"
            item["deletion_candidate"] = False
            item["evidence"] = [
                f"closed pull request #{number} cannot be resolved uniquely from the live snapshot; historical deletion fails closed"
            ]
            branches.append(item)
            continue

        body = pull.get("body") if isinstance(pull.get("body"), str) else ""
        disposition, reason, error = parse_disposition_metadata(body)
        if error is not None:
            item["classification"] = "UNMERGED_ORPHAN"
            item["deletion_candidate"] = False
            item["evidence"] = [
                f"closed pull request #{number} has malformed branch-disposition metadata: {error}",
                "historical deletion fails closed until the branch disposition is explicitly reconciled",
            ]
        elif disposition == "retain":
            item["classification"] = "UNMERGED_ORPHAN"
            item["deletion_candidate"] = False
            item["evidence"] = [
                f"closed pull request #{number} explicitly retains the source branch",
                f"retention reason: {reason}",
            ]
            if isinstance(pr_evidence, dict):
                retained = dict(pr_evidence)
                retained["disposition"] = "retain"
                retained["disposition_reason"] = reason
                item["closed_unmerged_pr"] = retained
        branches.append(item)

    branches.sort(key=lambda entry: str(entry.get("branch", "")))
    guarded["branches"] = branches
    guarded["counts"] = dict(
        sorted(Counter(str(entry.get("classification")) for entry in branches).items())
    )
    guarded["deletion_candidate_count"] = sum(
        1 for entry in branches if entry.get("deletion_candidate") is True
    )
    return guarded


def classify_snapshot(
    policy: dict[str, Any], snapshot: dict[str, Any], *, root: Path
) -> dict[str, Any]:
    return apply_historical_retention_guard(
        _ORIGINAL_CLASSIFY_SNAPSHOT(policy, snapshot, root=root), snapshot
    )


def _event_disposition(body: str) -> tuple[str | None, str | None]:
    disposition, reason, error = parse_disposition_metadata(body)
    if error is not None:
        raise ValidationError(error)
    return disposition, reason


def _revalidate_delete_entry(
    client: legacy.GitHubClient,
    policy: dict[str, Any],
    report: dict[str, Any],
    entry: dict[str, Any],
    *,
    root: Path,
) -> None:
    _ORIGINAL_REVALIDATE_DELETE_ENTRY(client, policy, report, entry, root=root)

    branch = branch_lifecycle._text(entry.get("branch"), "terminal delete entry.branch")
    expected_sha = branch_lifecycle._text(
        entry.get("head_sha"), f"terminal delete {branch}.head_sha"
    )
    pr_number = entry.get("closed_pr_number")
    if not isinstance(pr_number, int) or isinstance(pr_number, bool):
        raise ValidationError(f"terminal delete {branch}: closed_pr_number must be integer")

    pull, _ = client.request(
        "GET", f"/repos/{client.repo}/pulls/{pr_number}", expected=(200,)
    )
    if not isinstance(pull, dict):
        raise ValidationError(f"terminal delete {branch}: pull request response is not an object")
    if (
        pull.get("state") != "closed"
        or pull.get("merged") is True
        or pull.get("merged_at") is not None
        or not branch_lifecycle._same_repo_pull(pull, report["repository"])
        or branch_lifecycle._pull_branch(pull) != branch
        or branch_lifecycle._pull_sha(pull) != expected_sha
    ):
        raise ValidationError(
            f"terminal delete {branch}: live closed-PR identity drift detected"
        )

    body = pull.get("body") if isinstance(pull.get("body"), str) else ""
    disposition, _, error = parse_disposition_metadata(body)
    if error is not None:
        raise ValidationError(
            f"terminal delete {branch}: malformed live branch-disposition metadata: {error}"
        )
    if disposition == "retain":
        raise ValidationError(
            f"terminal delete {branch}: live pull request now explicitly retains the branch"
        )


def main(argv: list[str] | None = None) -> int:
    legacy.classify_snapshot = classify_snapshot
    legacy._event_disposition = _event_disposition
    legacy._revalidate_delete_entry = _revalidate_delete_entry
    return legacy.main(argv)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValidationError, ApiError, json.JSONDecodeError, OSError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
