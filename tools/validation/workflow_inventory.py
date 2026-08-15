#!/usr/bin/env python3
"""Classify GitHub Actions workflows and enforce their lifecycle registry."""

from __future__ import annotations

import json
import re
from collections import Counter
from pathlib import Path
from typing import Any


WORKFLOW_ROOT = Path(".github/workflows")
LIFECYCLE_POLICY = Path("docs/agents/CI_WORKFLOW_LIFECYCLE.json")
TOP_LEVEL_NAME = re.compile(r"(?m)^name:\s+\S")
TOP_LEVEL_ON = re.compile(r"(?m)^on:\s*$")
TOP_LEVEL_PERMISSIONS = re.compile(
    r"(?m)^permissions:\s*(?:\{\s*\}|read-all|write-all)?\s*$"
)
DIRECT_MAPPING_KEY = re.compile(
    r'^  (?:(?P<plain>[A-Za-z0-9_-]+)|"(?P<double>[A-Za-z0-9_-]+)"|\'(?P<single>[A-Za-z0-9_-]+)\'):(?:\s.*)?$'
)
DOMAIN_EVENTS = frozenset({"pull_request", "pull_request_target", "push"})
MANUAL_EVENTS = frozenset({"issue_comment", "workflow_dispatch"})
SUPPORTED_EVENTS = frozenset({*DOMAIN_EVENTS, *MANUAL_EVENTS, "schedule", "workflow_call"})


class WorkflowInventoryError(RuntimeError):
    """Raised when the workflow inventory cannot be classified safely."""


def _top_level_on_events(text: str) -> set[str]:
    """Return only direct children of the top-level ``on`` mapping."""

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
        if line.startswith("\t"):
            raise WorkflowInventoryError("unparseable top-level workflow event indentation")
        indent = len(line) - len(line.lstrip(" "))
        if indent == 1:
            raise WorkflowInventoryError("unparseable top-level workflow event indentation")
        if indent != 2:
            continue
        match = DIRECT_MAPPING_KEY.match(line)
        if match is None:
            raise WorkflowInventoryError(
                f"unparseable top-level workflow event key: {line.strip()}"
            )
        event = next(group for group in match.groups() if group is not None)
        events.add(event)
    return events


def classify_workflow(path: Path, text: str) -> str:
    name = path.name
    events = _top_level_on_events(text)
    unsupported_events = events - SUPPORTED_EVENTS

    if not events:
        raise WorkflowInventoryError(
            f"unclassified workflow: {path.as_posix()} (no supported top-level event)"
        )
    if unsupported_events:
        rendered = ", ".join(sorted(unsupported_events))
        raise WorkflowInventoryError(
            f"unclassified workflow: {path.as_posix()} "
            f"(unsupported top-level event(s): {rendered})"
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
        raise WorkflowInventoryError(
            f"missing workflow directory: {WORKFLOW_ROOT.as_posix()}"
        )

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
                findings.append(
                    f"{relative}: missing required top-level workflow marker {marker}"
                )
        try:
            classifications[relative] = classify_workflow(path, text)
        except WorkflowInventoryError as exc:
            findings.append(str(exc))

    if findings:
        raise WorkflowInventoryError(
            "Workflow inventory validation failed:\n- " + "\n- ".join(findings)
        )
    return classifications


def _load_lifecycle_policy(path: Path) -> dict[str, Any]:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise WorkflowInventoryError(
            f"missing workflow lifecycle policy: {path.as_posix()}"
        ) from exc
    except json.JSONDecodeError as exc:
        raise WorkflowInventoryError(
            f"invalid workflow lifecycle JSON at line {exc.lineno}: {exc.msg}"
        ) from exc
    if not isinstance(raw, dict) or raw.get("schema_version") != 1:
        raise WorkflowInventoryError("workflow lifecycle policy must use schema_version 1")
    return raw


def _string_list(raw: dict[str, Any], key: str) -> list[str]:
    value = raw.get(key)
    if not isinstance(value, list) or not all(
        isinstance(item, str) and item.strip() for item in value
    ):
        raise WorkflowInventoryError(f"workflow lifecycle {key} must be a string list")
    normalized = [item.strip() for item in value]
    if len(normalized) != len(set(normalized)):
        raise WorkflowInventoryError(f"workflow lifecycle {key} contains duplicates")
    return normalized


def validate_lifecycle_policy(
    root: Path,
    policy_path: Path | None = None,
) -> dict[str, object]:
    """Fail closed on silent workflow growth, retired workflow return, or stale registry."""

    root = root.resolve()
    policy_file = (
        (root / LIFECYCLE_POLICY)
        if policy_path is None
        else (policy_path if policy_path.is_absolute() else root / policy_path)
    )
    raw = _load_lifecycle_policy(policy_file)
    registered = _string_list(raw, "registered_workflows")
    retired = _string_list(raw, "retired_workflows")
    manual_only = _string_list(raw, "manual_only_workflows")
    workflow_budget = raw.get("workflow_budget")

    if not isinstance(workflow_budget, int) or workflow_budget <= 0:
        raise WorkflowInventoryError("workflow lifecycle workflow_budget must be positive")
    if set(registered) & set(retired):
        raise WorkflowInventoryError("registered and retired workflows overlap")
    if not set(manual_only).issubset(set(registered)):
        raise WorkflowInventoryError("manual_only_workflows must be registered")

    workflow_root = root / WORKFLOW_ROOT
    actual_paths = sorted(
        [*workflow_root.glob("*.yml"), *workflow_root.glob("*.yaml")],
        key=lambda path: path.name,
    )
    actual = {path.name for path in actual_paths}
    registered_set = set(registered)
    retired_set = set(retired)

    findings: list[str] = []
    missing = sorted(registered_set - actual)
    unexpected = sorted(actual - registered_set)
    reintroduced = sorted(actual & retired_set)

    if missing:
        findings.append(f"registered workflows missing from repository: {missing}")
    if unexpected:
        findings.append(
            "unregistered workflow files require explicit lifecycle review: "
            f"{unexpected}"
        )
    if reintroduced:
        findings.append(f"retired workflows were reintroduced: {reintroduced}")
    if len(actual) > workflow_budget:
        findings.append(
            f"workflow budget exceeded: actual={len(actual)} budget={workflow_budget}"
        )

    for name in manual_only:
        path = workflow_root / name
        if not path.is_file():
            continue
        events = _top_level_on_events(path.read_text(encoding="utf-8"))
        if events != {"workflow_dispatch"}:
            findings.append(
                f"{name}: manual-only workflow must trigger only on workflow_dispatch, "
                f"found {sorted(events)}"
            )

    if findings:
        raise WorkflowInventoryError(
            "Workflow lifecycle validation failed:\n- " + "\n- ".join(findings)
        )

    return {
        "registered": len(registered),
        "actual": len(actual),
        "budget": workflow_budget,
        "retired": len(retired),
        "manual_only": len(manual_only),
    }


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    try:
        classifications = validate_inventory(root)
        lifecycle = validate_lifecycle_policy(root)
    except WorkflowInventoryError as exc:
        print(str(exc))
        return 1

    counts = Counter(classifications.values())
    summary = ", ".join(f"{key}={counts[key]}" for key in sorted(counts))
    print(f"Classified {len(classifications)} workflow(s): {summary}")
    print(
        "Workflow lifecycle PASS: "
        f"actual={lifecycle['actual']} budget={lifecycle['budget']} "
        f"retired={lifecycle['retired']} manual_only={lifecycle['manual_only']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
