#!/usr/bin/env python3
"""Fail-closed consistency checks for duplicated Oteryn agent-governance rules."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Iterable

REPOSITORY_FULL_NAME = "blakinio/Oteryn-Platform"
REPO_ROOT = Path(__file__).resolve().parents[2]
KNOWN_PLAIN_REPOSITORY_OWNERS = {"blakinio", "opentibiabr"}

CHECKED_PATHS = (
    "AGENTS.md",
    "AGENTS.override.md",
    "docs/agents/AGENTS.md",
    "docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md",
    "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md",
    "docs/agents/GOVERNANCE_CONTRACT.json",
)

NUMBER_WORDS = {
    0: "zero", 1: "one", 2: "two", 3: "three", 4: "four", 5: "five", 6: "six",
    7: "seven", 8: "eight", 9: "nine", 10: "ten", 11: "eleven", 12: "twelve",
}


class PolicyConsistencyError(RuntimeError):
    """Raised when a governance source cannot be parsed deterministically."""


def _read_text(root: Path, relative_path: str) -> str:
    try:
        return (root / relative_path).read_text(encoding="utf-8")
    except OSError as exc:
        raise PolicyConsistencyError(f"cannot read {relative_path}: {exc}") from exc


def _read_json(root: Path, relative_path: str) -> dict[str, object]:
    try:
        decoded = json.loads(_read_text(root, relative_path))
    except json.JSONDecodeError as exc:
        raise PolicyConsistencyError(f"invalid JSON in {relative_path}: {exc}") from exc
    if not isinstance(decoded, dict):
        raise PolicyConsistencyError(f"{relative_path} must contain a JSON object")
    return decoded


def _yaml_list(markdown: str, key: str) -> list[str]:
    lines = markdown.splitlines()
    for index, line in enumerate(lines):
        if line.strip() != f"{key}:":
            continue
        values: list[str] = []
        for candidate in lines[index + 1 :]:
            match = re.match(r"^\s*-\s+([^#]+?)\s*$", candidate)
            if match:
                values.append(match.group(1).strip().strip("'\""))
                continue
            if candidate.strip() == "":
                if values:
                    break
                continue
            break
        if values:
            return values
    raise PolicyConsistencyError(f"cannot parse YAML list {key}")


def _yaml_int(markdown: str, key: str) -> int:
    match = re.search(rf"(?m)^\s*{re.escape(key)}:\s*(\d+)\s*$", markdown)
    if not match:
        raise PolicyConsistencyError(f"cannot parse integer policy value {key}")
    return int(match.group(1))


def _backticked_values_from_line(markdown: str, marker: str) -> list[str]:
    for line in markdown.splitlines():
        if marker in line:
            return re.findall(r"`([^`]+)`", line)
    raise PolicyConsistencyError(f"cannot find policy line containing {marker!r}")


def _require_exact_line(errors: list[str], source: str, text: str, values: Iterable[str], label: str) -> None:
    expected = " | ".join(values)
    if not re.search(rf"(?m)^{re.escape(expected)}$", text):
        errors.append(f"{source}: {label} drift; expected exact line: {expected}")


def _require_marker(errors: list[str], source: str, text: str, marker: str) -> None:
    if marker not in text:
        errors.append(f"{source}: missing required governance marker: {marker}")


def _normalize_inline_markdown(text: str) -> str:
    return re.sub(r"[*_`]", "", text)


def _require_regex_value(errors: list[str], source: str, text: str, pattern: str, expected: int, label: str) -> None:
    matches = list(re.finditer(pattern, _normalize_inline_markdown(text), flags=re.IGNORECASE))
    if not matches:
        errors.append(f"{source}: cannot locate duplicated budget marker for {label}")
        return
    actual_values = [int(match.group("value")) for match in matches]
    conflicting = sorted({value for value in actual_values if value != expected})
    if conflicting:
        errors.append(f"{source}: {label} drift; canonical={expected}, conflicting declarations={conflicting}")


def _logical_markdown_statements(markdown: str) -> list[str]:
    """Join wrapped Markdown lines while keeping separate bullets/statements separate."""
    statements: list[str] = []
    current: list[str] = []

    def flush() -> None:
        if current:
            statements.append(" ".join(current))
            current.clear()

    for raw in markdown.splitlines():
        stripped = raw.strip()
        if not stripped or stripped.startswith("#") or stripped.startswith("```"):
            flush()
            continue
        bullet = re.match(r"^\s*[-*+]\s+(.*)$", raw)
        if bullet:
            flush()
            current.append(bullet.group(1).strip())
            continue
        if current:
            current.append(stripped)
        else:
            current.append(stripped)
    flush()
    return statements


def _repository_identifiers(statement: str) -> list[str]:
    """Extract repository-shaped identifiers without mistaking prose like commit/PR for repos."""
    repository_pattern = r"([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)"
    identifiers = set(re.findall(rf"`{repository_pattern}`", statement))
    for candidate in re.findall(
        rf"(?<![A-Za-z0-9_.-]){repository_pattern}(?![A-Za-z0-9_.-])", statement
    ):
        owner = candidate.split("/", 1)[0].casefold()
        if owner in KNOWN_PLAIN_REPOSITORY_OWNERS:
            identifiers.add(candidate)
    return sorted(identifiers)


def _is_current_task_user_authorization_exception(lowered: str) -> bool:
    conditional = "only when" in lowered or "unless" in lowered
    explicitly_authorized = bool(
        re.search(r"\b(?:user|project\s+owner)\b.*\bexplicit(?:ly)?\b.*\b(?:authoriz\w*|grant\w*|permission)\b", lowered)
    )
    task_scoped = "current task" in lowered or "write task" in lowered or "separate permission" in lowered
    return conditional and explicitly_authorized and task_scoped


def _repo_has_positive_read_only_assertion(statement: str, repository: str) -> bool:
    """Return true only when read-only wording actually applies to this repository."""
    lowered = statement.casefold()
    repo = repository.casefold()
    for match in re.finditer(re.escape(repo), lowered):
        tail = lowered[match.start():]
        ro_index = tail.find("read-only")
        if ro_index < 0:
            continue
        assertion = tail[: ro_index + len("read-only")]
        if ";" in assertion or re.search(r"\bbut\b", assertion):
            continue
        if re.search(
            r"(?:\b(?:not|never|no\s+longer|is\s+not|was\s+not)\s+read-only\b|"
            r"\b(?:isn't|isn’t|wasn't|wasn’t)\s+read-only\b)",
            assertion,
        ):
            continue
        if re.search(r"\b(?:is|are|as|remain|remains|must\s+remain)\b.*\bread-only\b", assertion):
            return True
    return False


def _has_positive_mutation_authorization(lowered: str) -> bool:
    """Recognize actual grants while ignoring nearby denial/negation language."""
    positive = re.compile(r"\b(?:allowed|allow|allows|authorize|authorizes|authorized|authorization|permit|permits|permitted|may|can)\b")
    for match in positive.finditer(lowered):
        before = lowered[max(0, match.start() - 24):match.start()]
        after = lowered[match.end():match.end() + 12]
        if re.search(r"\b(?:not|never|no|isn't|isn’t|cannot|can't|can’t)\s*$", before):
            continue
        if match.group(0) in {"may", "can"} and re.match(r"\s+(?:not|never)\b", after):
            continue
        return True
    return False


def _statement_grants_repository_mutation(lowered: str) -> bool:
    mutation = bool(
        re.search(r"\b(?:write|writes|edit|edits|push|pushes|merge|merges|mutat(?:e|es|ion))\b", lowered)
    )
    return mutation and _has_positive_mutation_authorization(lowered)


def _reject_contradictory_repository_mutation_grants(
    errors: list[str], source: str, policy_text: str
) -> None:
    for statement in _logical_markdown_statements(policy_text):
        lowered = statement.casefold()
        repositories = _repository_identifiers(statement)
        foreign = sorted({value for value in repositories if value != REPOSITORY_FULL_NAME})
        if not foreign or not _statement_grants_repository_mutation(lowered):
            continue
        if _is_current_task_user_authorization_exception(lowered):
            continue
        contradictory = [repo for repo in foreign if not _repo_has_positive_read_only_assertion(statement, repo)]
        if contradictory:
            errors.append(
                f"{source}: contradictory repository mutation authorization in authoritative policy: "
                f"{', '.join(contradictory)}"
            )


def validate_policy(root: Path = REPO_ROOT) -> list[str]:
    """Return all material cross-document policy consistency findings."""
    errors: list[str] = []
    try:
        root_agents = _read_text(root, "AGENTS.md")
        override = _read_text(root, "AGENTS.override.md")
        docs_agents = _read_text(root, "docs/agents/AGENTS.md")
        anti_stall = _read_text(root, "docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md")
        delivery = _read_text(root, "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md")
        contract = _read_json(root, "docs/agents/GOVERNANCE_CONTRACT.json")

        shared = contract.get("shared_checkpoint_contract")
        if not isinstance(shared, dict):
            raise PolicyConsistencyError("GOVERNANCE_CONTRACT.json lacks shared_checkpoint_contract")
        statuses = shared.get("allowed_statuses")
        terminal_results = shared.get("terminal_invocation_results")
        if not isinstance(statuses, list) or not all(isinstance(value, str) for value in statuses):
            raise PolicyConsistencyError("shared_checkpoint_contract.allowed_statuses must be a string list")
        if not isinstance(terminal_results, list) or not all(isinstance(value, str) for value in terminal_results):
            raise PolicyConsistencyError("shared_checkpoint_contract.terminal_invocation_results must be a string list")
        canonical_statuses = list(statuses)
        canonical_terminal = list(terminal_results)

        anti_statuses = _yaml_list(anti_stall, "checkpoint_task_statuses")
        if anti_statuses != canonical_statuses:
            errors.append(
                "docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md: checkpoint task statuses drift; "
                f"canonical={canonical_statuses}, duplicate={anti_statuses}"
            )
        anti_terminal = _yaml_list(anti_stall, "terminal_invocation_results")
        if anti_terminal != canonical_terminal:
            errors.append(
                "docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md: terminal invocation results drift; "
                f"canonical={canonical_terminal}, duplicate={anti_terminal}"
            )

        _require_exact_line(errors, "docs/agents/AGENTS.md", docs_agents, canonical_statuses, "checkpoint task statuses")
        _require_exact_line(errors, "docs/agents/AGENTS.md", docs_agents, canonical_terminal, "terminal invocation results")

        override_statuses = _backticked_values_from_line(override, "checkpoint task status:")
        if override_statuses != canonical_statuses:
            errors.append(
                "AGENTS.override.md: checkpoint task statuses drift; "
                f"canonical={canonical_statuses}, duplicate={override_statuses}"
            )
        override_terminal = _backticked_values_from_line(override, "terminal invocation result:")
        if override_terminal != canonical_terminal:
            errors.append(
                "AGENTS.override.md: terminal invocation results drift; "
                f"canonical={canonical_terminal}, duplicate={override_terminal}"
            )

        budget_keys = {
            key: _yaml_int(anti_stall, key)
            for key in (
                "normal_foreground_runtime_minutes",
                "large_foreground_runtime_minutes",
                "no_progress_minutes",
                "max_ci_state_checks_per_exact_head",
                "max_unchanged_external_state_checks",
                "terminal_ci_wait_budget_minutes",
                "terminal_ci_minimum_poll_interval_minutes",
                "max_terminal_ci_state_checks_per_check_generation",
                "max_additional_tasks_after_terminal_entry_task",
                "minimum_remaining_minutes_to_start_additional_task",
            )
        }
        patterns = (
            (r"Default to (?P<value>\d+) minutes per foreground invocation", "normal_foreground_runtime_minutes"),
            (r"allow (?P<value>\d+) minutes only when", "large_foreground_runtime_minutes"),
            (r"Stop after (?P<value>\d+) minutes without measurable progress", "no_progress_minutes"),
            (r"exception is capped at (?P<value>\d+) minutes", "terminal_ci_wait_budget_minutes"),
            (r"permits at most (?P<value>\d+) checks per materially new required-check generation", "max_terminal_ci_state_checks_per_check_generation"),
            (r"only when at least (?P<value>\d+) minutes remains", "minimum_remaining_minutes_to_start_additional_task"),
        )
        for pattern, key in patterns:
            _require_regex_value(errors, "AGENTS.override.md", override, pattern, budget_keys[key], key)

        ordinary_checks = budget_keys["max_ci_state_checks_per_exact_head"]
        external_checks = budget_keys["max_unchanged_external_state_checks"]
        if ordinary_checks != external_checks:
            errors.append(
                "ANTI_STALL_AND_EXECUTION_BUDGET.md: root bootstrap combines ordinary CI and external checks, "
                "but their canonical limits differ"
            )
        elif ordinary_checks != 2 or "at most twice per exact head" not in override:
            errors.append(
                "AGENTS.override.md: ordinary CI/external-state check limit drift; "
                f"canonical={ordinary_checks}, duplicate marker='at most twice per exact head'"
            )

        poll_minutes = budget_keys["terminal_ci_minimum_poll_interval_minutes"]
        poll_word = NUMBER_WORDS.get(poll_minutes)
        if poll_word is None or f"requires at least {poll_word} minutes between unchanged checks" not in override:
            errors.append(f"AGENTS.override.md: terminal CI poll interval drift; canonical={poll_minutes} minutes")

        additional_tasks = budget_keys["max_additional_tasks_after_terminal_entry_task"]
        additional_word = NUMBER_WORDS.get(additional_tasks)
        if additional_word is None or f"at most {additional_word} additional task may be started" not in override:
            errors.append(f"AGENTS.override.md: additional-task limit drift; canonical={additional_tasks}")

        scope_markers = {
            "AGENTS.md": [
                f"The only repository where autonomous write operations are allowed by this file is `{REPOSITORY_FULL_NAME}`.",
                f"verify that `repository_full_name` is exactly `{REPOSITORY_FULL_NAME}`",
            ],
            "AGENTS.override.md": [
                f"default authorization for work launched from `{REPOSITORY_FULL_NAME}` is **WWW Platform only**",
                "must **not be accessed, read, inspected, searched, fetched, branched, edited, reviewed, audited, merged or otherwise operated on unless the project owner explicitly grants separate permission",
            ],
        }
        source_text = {"AGENTS.md": root_agents, "AGENTS.override.md": override}
        for source, markers in scope_markers.items():
            for marker in markers:
                _require_marker(errors, source, source_text[source], marker)
        _reject_contradictory_repository_mutation_grants(errors, "AGENTS.md", root_agents)
        _reject_contradictory_repository_mutation_grants(errors, "AGENTS.override.md", override)

        completion_markers = {
            "AGENTS.override.md": (
                "exact-head self-review", "real E2E", "required CI on the exact final head",
                "zero unresolved review threads", "terminal task record", "released ownership",
            ),
            "docs/agents/AGENTS.md": (
                "exact-head full-diff self-review", "real E2E", "zero unresolved material findings",
                "task archival", "ownership release",
            ),
            "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md": (
                "## Mandatory self-review", "## E2E", "## Exact-head CI and Actions economy",
                "## Related PR hygiene", "## Terminal closeout",
            ),
        }
        completion_source_text = {
            "AGENTS.override.md": override,
            "docs/agents/AGENTS.md": docs_agents,
            "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md": delivery,
        }
        for source, markers in completion_markers.items():
            for marker in markers:
                _require_marker(errors, source, completion_source_text[source], marker)

    except PolicyConsistencyError as exc:
        errors.append(str(exc))
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
