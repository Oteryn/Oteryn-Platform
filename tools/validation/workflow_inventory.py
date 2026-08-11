#!/usr/bin/env python3
"""Classify every GitHub Actions workflow and fail closed on unmanaged workflow shapes."""

from __future__ import annotations

from collections import Counter
from pathlib import Path


WORKFLOW_ROOT = Path(".github/workflows")


class WorkflowInventoryError(RuntimeError):
    """Raised when the workflow inventory cannot be classified safely."""


def _contains_event(text: str, event: str) -> bool:
    marker = f"  {event}:"
    return marker in text


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
        for marker in ("name:", "on:", "permissions:"):
            if marker not in text:
                findings.append(f"{relative}: missing required workflow marker {marker}")
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
