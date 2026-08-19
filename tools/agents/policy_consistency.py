#!/usr/bin/env python3
"""Small fail-closed consistency checks for current Platform agent governance."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPOSITORY_FULL_NAME = "Oteryn/Oteryn-Platform"
LEGACY_REPOSITORY = "blakinio/Oteryn-Platform"
REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKED_PATHS = (
    "AGENTS.md",
    "docs/agents/AGENTS.md",
    "docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md",
    "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md",
    "docs/agents/GOVERNANCE_CONTRACT.json",
    "docs/agents/PROJECT_LANES.json",
)


def _text(root: Path, relative: str) -> str:
    return (root / relative).read_text(encoding="utf-8")


def _json(root: Path, relative: str) -> dict:
    value = json.loads(_text(root, relative))
    if not isinstance(value, dict):
        raise ValueError(f"{relative} must contain a JSON object")
    return value


def _pipe_values(text: str, marker: str) -> list[str]:
    pos = text.find(marker)
    if pos < 0:
        return []
    tail = text[pos + len(marker):]
    match = re.search(r"```text\s*\n([^`]+?)\n```", tail, flags=re.S)
    if not match:
        return []
    return [part.strip() for part in match.group(1).strip().split("|") if part.strip()]


def validate_policy(root: Path = REPO_ROOT) -> list[str]:
    errors: list[str] = []
    try:
        agents = _text(root, "AGENTS.md")
        docs_agents = _text(root, "docs/agents/AGENTS.md")
        anti_stall = _text(root, "docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md")
        delivery = _text(root, "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md")
        contract = _json(root, "docs/agents/GOVERNANCE_CONTRACT.json")
        lanes = _json(root, "docs/agents/PROJECT_LANES.json")
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
        return [f"cannot load governance source: {exc}"]

    if (root / "AGENTS.override.md").exists():
        errors.append("AGENTS.override.md must be absent unless intentional same-directory replacement semantics are documented")
    required_root = (
        REPOSITORY_FULL_NAME,
        "GitHub Issue is authoritative",
        "one independently mergeable task -> one branch -> one PR",
        "Do not push ordinary work directly to `main`",
        "classify-changes",
        "test",
    )
    for marker in required_root:
        if marker not in agents:
            errors.append(f"AGENTS.md missing durable marker: {marker}")
    legacy_lines = [line for line in agents.splitlines() if LEGACY_REPOSITORY in line]
    if any("historical" not in line.casefold() for line in legacy_lines):
        errors.append(f"AGENTS.md uses {LEGACY_REPOSITORY} outside an explicit historical statement")

    shared = contract.get("shared_checkpoint_contract")
    if not isinstance(shared, dict):
        errors.append("GOVERNANCE_CONTRACT.json missing shared_checkpoint_contract")
        return errors
    expected_statuses = shared.get("allowed_statuses")
    expected_terminal = shared.get("terminal_invocation_results")
    docs_statuses = _pipe_values(docs_agents, "Use these checkpoint task statuses only:")
    docs_terminal = _pipe_values(docs_agents, "Use these terminal invocation results only:")
    if docs_statuses != expected_statuses:
        errors.append(f"checkpoint task statuses drift: docs={docs_statuses!r} contract={expected_statuses!r}")
    if docs_terminal != expected_terminal:
        errors.append(f"terminal invocation results drift: docs={docs_terminal!r} contract={expected_terminal!r}")

    budget_blocks = re.findall(r"```yaml\s*\n(.*?)\n```", anti_stall, flags=re.S)
    if not any("max_ci_state_checks_per_exact_head:" in block for block in budget_blocks):
        errors.append("ANTI_STALL_AND_EXECUTION_BUDGET.md missing machine-readable CI budget")
    for marker in ("## Mandatory self-review", "## Exact-head CI and Actions economy", "## Terminal closeout"):
        if marker not in delivery:
            errors.append(f"DELIVERY_COMPLETENESS_AND_CLOSEOUT.md missing marker: {marker}")

    repository_value = lanes.get("repository")
    if repository_value is not None and str(repository_value).casefold() != REPOSITORY_FULL_NAME.casefold():
        errors.append(f"PROJECT_LANES.json repository drift: {repository_value!r}")
    return errors


def main() -> int:
    errors = validate_policy()
    if errors:
        for error in errors:
            print(f"policy-consistency: {error}", file=sys.stderr)
        return 1
    print("Agent governance policy consistency: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())