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


class WorkflowInventoryError(RuntimeError):
    """Raised when the workflow inventory cannot be classified safely."""


def _contains_event(text: str, event: str) -> bool:
    return re.search(rf"(?m)^  {re.escape(event)}:\s*", text) is not None


def classify_workflow(path: Path, text: str) -> str:
    name = path.name

    if name == "ci.yml":
        return "required_core"
    if name in {"agent-governance.yml", "branch-lifecycle.yml"}:
        return "governance"
    if name.startswith("build-"):
        return "build"
    if name.startswith("deploy-") or "staging-control" in name or "main-operation" in name:
        return "deployment_operation"
    if _contains_event(text, "workflow_call"):
        return "reusable_validation"
    if _contains_event(text, "schedule") and not (
        _contains_event(text, "pull_request") or _contains_event(text, "push")
    ):
        return "scheduled_validation"
    if _contains_event(text, "pull_request") or _contains_event(text, "push"):
        return "domain_validation"
    if _contains_event(text, "workflow_dispatch"):
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
