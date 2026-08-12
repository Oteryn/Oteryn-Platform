#!/usr/bin/env python3
"""Classify every GitHub Actions workflow and fail closed on unmanaged workflow shapes."""

from __future__ import annotations

import re
from collections import Counter
from pathlib import Path


WORKFLOW_ROOT = Path(".github/workflows")
TOP_LEVEL_NAME = re.compile(r"(?m)^name:\s+\S")
TOP_LEVEL_ON = re.compile(r"(?m)^on:\s*$")
# GitHub Actions accepts a permissions mapping, an explicit empty mapping, or
# the read-all/write-all shorthands at workflow scope. Keep the anchor at
# column zero so a job-level permissions block cannot satisfy this contract.
TOP_LEVEL_PERMISSIONS = re.compile(
    r"(?m)^permissions:\s*(?:\{\s*\}|read-all|write-all)?\s*$"
)
DIRECT_MAPPING_KEY = re.compile(r"^  ([A-Za-z0-9_-]+):(?:\s.*)?$")
DOMAIN_EVENTS = frozenset({"pull_request", "pull_request_target", "push"})
MANUAL_EVENTS = frozenset({"issue_comment", "workflow_dispatch"})
SUPPORTED_EVENTS = frozenset({*DOMAIN_EVENTS, *MANUAL_EVENTS, "schedule", "workflow_call"})


class WorkflowInventoryError(RuntimeError):
    """Raised when the workflow inventory cannot be classified safely."""


def _top_level_on_events(text: str) -> set[str]:
    """Return only direct children of the top-level ``on`` mapping.

    A two-space key elsewhere (for example a job named ``pull_request``) must
    never be mistaken for a workflow event. The repository contract already
    requires block-style ``on:`` at column zero, so a conservative indentation
    parser is safer here than a workflow-wide regex.
    """

    lines = text.splitlines()
    on_index: int | None = None
    for index, line in enumerate(lines):
        if line == "on:":
            on_index = index
            break
    if on_index is None:
        return set()

    events: set[str] = set()
    for line in lines[on_index + 1 :]:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not line.startswith((" ", "\t")):
            break
        match = DIRECT_MAPPING_KEY.match(line)
        if match:
            events.add(match.group(1))
    return events


def classify_workflow(path: Path, text: str) -> str:
    name = path.name
    events = _top_level_on_events(text)
    unsupported_events = events - SUPPORTED_EVENTS

    if not events:
        raise WorkflowInventoryError(f"unclassified workflow: {path.as_posix()} (no supported top-level event)")
    if unsupported_events:
        rendered = ", ".join(sorted(unsupported_events))
        raise WorkflowInventoryError(
            f"unclassified workflow: {path.as_posix()} (unsupported top-level event(s): {rendered})"
        )

    if name == "ci.yml":
        return "required_core"
    if name in {"agent-governance.yml", "branch-lifecycle.yml"}:
        return "governance"
    if name.startswith("build-"):
        return "build"
    if name.startswith("deploy-") or "staging-control" in name or "main-operation" in name:
        return "deployment_operation"
    if "workflow_call" in events:
        return "reusable_validation"
    if "schedule" in events and not (DOMAIN_EVENTS & events):
        return "scheduled_validation"
    if DOMAIN_EVENTS & events:
        return "domain_validation"
    if MANUAL_EVENTS & events:
        return "manual_validation"

    raise WorkflowInventoryError(f"unclassified workflow: {path.as_posix()}")


def validate_inventory(root: Path) -> dict[str, str]:
    root = root.resolve()
    workflow_root = root / WORKFLOW_ROOT
    if not workflow_root.is_dir():
        raise WorkflowInventoryError(f"missing workflow directory: {WORKFLOW_ROOT.as_posix()}")

    workflow_paths = sorted(
        [*workflow_root.glob("*.yml"), *workflow_root.glob("*.yaml")],
        key=lambda path: path.name,
    )
    if not workflow_paths:
        raise WorkflowInventoryError("workflow inventory is empty")

    classifications: dict[str, str] = {}
    findings: list[str] = []
    for path in workflow_paths:
        relative = path.relative_to(root).as_posix()
        text = path.read_text(encoding="utf-8")
        required_markers = {
            "name:": TOP_LEVEL_NAME,
            "on:": TOP_LEVEL_ON,
            "permissions:": TOP_LEVEL_PERMISSIONS,
        }
        for marker, pattern in required_markers.items():
            if pattern.search(text) is None:
                findings.append(f"{relative}: missing required top-level workflow marker {marker}")
        try:
            classifications[relative] = classify_workflow(path, text)
        except WorkflowInventoryError as exc:
            findings.append(str(exc))

    if findings:
        raise WorkflowInventoryError("Workflow inventory validation failed:\n- " + "\n- ".join(findings))
    return classifications


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    try:
        classifications = validate_inventory(root)
    except WorkflowInventoryError as exc:
        print(str(exc))
        return 1

    counts = Counter(classifications.values())
    summary = ", ".join(f"{key}={counts[key]}" for key in sorted(counts))
    print(f"Classified {len(classifications)} workflow(s): {summary}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
