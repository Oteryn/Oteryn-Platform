#!/usr/bin/env python3
"""Fail-closed consistency checks for duplicated Oteryn agent-governance rules."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPOSITORY_FULL_NAME = "blakinio/Oteryn-Platform"
REPO_ROOT = Path(__file__).resolve().parents[2]

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

REPO_TOKEN = re.compile(r"(?<![A-Za-z0-9_.-])([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)(?![A-Za-z0-9_.-])")
QUOTED_REPO_TOKEN = re.compile(r"`([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)`")
MUTATION_TERM = r"(?:writ(?:e|es|ing|ten)|writable|edit(?:s|ed|ing)?|modif(?:y|ies|ied|ying)|push(?:es|ed|ing)?|commit(?:s|ted|ting)?|merge(?:s|d|ing)?|mutat(?:e|es|ed|ing|ion|ions))"
MUTATION_WORD = re.compile(rf"\b{MUTATION_TERM}\b", re.I)
POSITIVE_AUTH = re.compile(r"\b(?:allow|allows|allowed|authorize|authorizes|authorized|permit|permits|permitted|may|can|grant|grants|granted)\b", re.I)
NEGATIVE_AUTH = re.compile(
    r"\b(?:not\s+allowed|not\s+authorized|not\s+permitted|may\s+not|must\s+not\s+grant|"
    r"(?:do|does|did)\s+not\s+grant|never\s+grant(?:s|ed)?|not\s+grant(?:s|ed)?|"
    r"cannot|can't|can’t|never\s+allowed|unauthorized)\b",
    re.I,
)


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


def _normalize_inline_markdown(text: str) -> str:
    """Strip inline emphasis while preserving repository-token identity exactly."""
    protected: list[str] = []

    def protect(match: re.Match[str]) -> str:
        protected.append(match.group(1))
        return f"\x00REPO{len(protected) - 1}\x00"

    normalized = REPO_TOKEN.sub(protect, text)
    normalized = re.sub(r"[*_`]", "", normalized)
    for index, repository in enumerate(protected):
        normalized = normalized.replace(f"\x00REPO{index}\x00", repository)
    return normalized


def _fence_delimiter(stripped: str) -> tuple[str, int, str] | None:
    match = re.match(r"^(`{3,}|~{3,})(.*)$", stripped)
    if not match:
        return None
    token = match.group(1)
    return token[0], len(token), match.group(2)


def _markdown_outside_fences(markdown: str) -> str:
    """Return only authoritative Markdown content outside fenced examples."""
    lines: list[str] = []
    active_fence: tuple[str, int] | None = None
    for raw in markdown.splitlines():
        delimiter = _fence_delimiter(raw.strip())
        if active_fence is not None:
            if delimiter:
                char, length, remainder = delimiter
                if char == active_fence[0] and length >= active_fence[1] and not remainder.strip():
                    active_fence = None
            continue
        if delimiter:
            char, length, _remainder = delimiter
            active_fence = (char, length)
            continue
        lines.append(raw)
    return "\n".join(lines)


def _normalize_marker_line(raw: str) -> str:
    """Strip line-wide Markdown emphasis from a prospective policy marker."""
    match = re.match(r"^(?P<indent>\s*)[*_]{1,6}(?P<body>.+?)[*_]{1,6}\s*$", raw)
    if match:
        return match.group("indent") + match.group("body")
    return raw


def _normalize_emphasized_marker_lines(markdown: str) -> str:
    """Strip line-wide Markdown emphasis outside fenced examples."""
    lines: list[str] = []
    active_fence: tuple[str, int] | None = None
    for raw in markdown.splitlines():
        stripped = raw.strip()
        delimiter = _fence_delimiter(stripped)
        if delimiter:
            char, length, remainder = delimiter
            if active_fence is None:
                active_fence = (char, length)
            elif char == active_fence[0] and length >= active_fence[1] and not remainder.strip():
                active_fence = None
            lines.append(raw)
            continue
        if active_fence is None:
            raw = _normalize_marker_line(raw)
        lines.append(raw)
    return "\n".join(lines)


def _yaml_lists(markdown: str, key: str) -> list[list[str]]:
    pattern = re.compile(rf"(?m)^\s*{re.escape(key)}:\s*\n(?P<body>(?:\s+-\s+[^\n]+\n?)+)")
    declarations: list[list[str]] = []
    for match in pattern.finditer(markdown):
        values = [item.strip().strip("'\"") for item in re.findall(r"(?m)^\s*-\s+([^#\n]+?)\s*$", match.group("body"))]
        if values:
            declarations.append(values)
    if not declarations:
        raise PolicyConsistencyError(f"cannot parse YAML list {key}")
    return declarations


def _yaml_int(markdown: str, key: str) -> int:
    matches = re.findall(rf"(?m)^\s*{re.escape(key)}:\s*(\d+)\s*$", markdown)
    if not matches:
        raise PolicyConsistencyError(f"cannot parse integer policy value {key}")
    values = {int(value) for value in matches}
    if len(values) != 1:
        raise PolicyConsistencyError(f"conflicting integer policy values for {key}: {sorted(values)}")
    return next(iter(values))


def _require_marker(errors: list[str], source: str, text: str, marker: str) -> None:
    if marker not in text:
        errors.append(f"{source}: missing required governance marker: {marker}")


def _require_regex_value(errors: list[str], source: str, text: str, pattern: str, expected: int, label: str) -> None:
    authoritative = _markdown_outside_fences(text)
    matches = list(re.finditer(pattern, _normalize_inline_markdown(authoritative), flags=re.IGNORECASE))
    if not matches:
        errors.append(f"{source}: cannot locate duplicated budget marker for {label}")
        return
    values = [int(match.group("value")) for match in matches]
    conflicting = sorted({value for value in values if value != expected})
    if conflicting:
        errors.append(f"{source}: {label} drift; canonical={expected}, conflicting declarations={conflicting}")


def _text_contract_declarations(markdown: str, marker: str) -> list[list[str]]:
    """Collect marker-owned text fences while ignoring declarations inside outer examples."""
    lines = markdown.splitlines()
    declarations: list[list[str]] = []
    active_fence: tuple[str, int] | None = None
    pending_marker = False
    index = 0

    while index < len(lines):
        raw = lines[index]
        stripped = raw.strip()
        delimiter = _fence_delimiter(stripped)

        if active_fence is not None:
            if delimiter:
                char, length, remainder = delimiter
                if char == active_fence[0] and length >= active_fence[1] and not remainder.strip():
                    active_fence = None
            index += 1
            continue

        normalized_line = _normalize_marker_line(raw).strip()
        if marker.casefold() in normalized_line.casefold():
            pending_marker = True
            index += 1
            continue

        if pending_marker and not stripped:
            index += 1
            continue

        if pending_marker and delimiter:
            char, length, remainder = delimiter
            if char == "`" and remainder.strip().casefold() == "text":
                body: list[str] = []
                opening = (char, length)
                index += 1
                while index < len(lines):
                    body_raw = lines[index]
                    body_delimiter = _fence_delimiter(body_raw.strip())
                    if body_delimiter:
                        close_char, close_length, close_remainder = body_delimiter
                        if close_char == opening[0] and close_length >= opening[1] and not close_remainder.strip():
                            break
                    body.append(body_raw)
                    index += 1
                values = [part.strip() for part in "\n".join(body).strip().split("|") if part.strip()]
                if values:
                    declarations.append(values)
                pending_marker = False
                index += 1
                continue
            pending_marker = False

        if delimiter:
            char, length, _remainder = delimiter
            active_fence = (char, length)
            index += 1
            continue

        if pending_marker:
            pending_marker = False
        index += 1

    return declarations


def _inline_backtick_declarations(markdown: str, marker: str) -> list[list[str]]:
    declarations: list[list[str]] = []
    for statement in _logical_markdown_statements(markdown):
        if marker.casefold() in statement.casefold():
            values = re.findall(r"`([^`]+)`", statement)
            if values:
                declarations.append(values)
    return declarations


def _require_all_declarations(errors: list[str], source: str, declarations: list[list[str]], expected: list[str], label: str) -> None:
    if not declarations:
        errors.append(f"{source}: cannot locate duplicated declaration for {label}")
        return
    conflicting = [decl for decl in declarations if decl != expected]
    if conflicting:
        errors.append(f"{source}: {label} drift; canonical={expected}, conflicting declarations={conflicting}")


def _logical_markdown_statements(markdown: str) -> list[str]:
    statements: list[str] = []
    current: list[str] = []
    active_fence: tuple[str, int] | None = None

    def flush() -> None:
        if current:
            statements.append(" ".join(current))
            current.clear()

    for raw in markdown.splitlines():
        stripped = raw.strip()
        delimiter = _fence_delimiter(stripped)
        if delimiter:
            char, length, remainder = delimiter
            if active_fence is None:
                flush()
                active_fence = (char, length)
            elif char == active_fence[0] and length >= active_fence[1] and not remainder.strip():
                active_fence = None
            continue
        if active_fence is not None:
            continue
        if not stripped or stripped.startswith("#"):
            flush()
            continue
        bullet = re.match(r"^\s*[-*+]\s+(.*)$", raw)
        if bullet:
            flush()
            current.append(bullet.group(1).strip())
        elif current:
            current.append(stripped)
        else:
            current.append(stripped)
    flush()
    return statements


def _policy_clauses(statement: str) -> list[str]:
    """Split independent grant clauses while preserving dependent condition language."""
    modal_adverb = r"(?:also|additionally|still|now|explicitly|autonomously)"
    new_grant = (
        rf"(?:additionally\s+)?(?:(?:the|a|an|any)\s+)?(?:agents?\s+)?"
        rf"(?:autonomous(?:ly)?\s+)?(?:{MUTATION_TERM}\b|"
        rf"(?:may|can)\s+(?:{modal_adverb}\s+)*{MUTATION_TERM}\b)"
    )
    pattern = (
        rf"\s*;\s*|(?<=\.)\s+|\s*,?\s+(?:but|however|while)\s+|"
        rf"\s+and\s+(?={new_grant})"
    )
    return [value.strip() for value in re.split(pattern, statement, flags=re.I) if value.strip()]


def _has_positive_mutation_grant(clause: str) -> bool:
    normalized = _normalize_inline_markdown(clause)
    if not MUTATION_WORD.search(normalized):
        return False
    if NEGATIVE_AUTH.search(normalized):
        positives = list(POSITIVE_AUTH.finditer(normalized))
        if not positives:
            return False
        for match in positives:
            window = normalized[max(0, match.start() - 28): match.end() + 28]
            if not NEGATIVE_AUTH.search(window):
                return True
        return False
    return bool(POSITIVE_AUTH.search(normalized))


def _repo_specific_window(clause: str, repository: str, radius: int = 140) -> str:
    lowered = _normalize_inline_markdown(clause).casefold()
    index = lowered.find(repository.casefold())
    if index < 0:
        return lowered
    return lowered[max(0, index - radius): index + len(repository) + radius]


def _repo_has_conditional_user_authorization(clause: str, repository: str) -> bool:
    window = _repo_specific_window(clause, repository, 220)
    if "only when" not in window:
        return False
    if not any(value in window for value in ("current task", "write task", "separate permission")):
        return False

    active_authorization = re.search(
        r"\b(?:the\s+)?(?:user|project\s+owner|owner)\b.{0,100}?"
        r"\bexplicitly\s+(?:authoriz(?:e|es|ed)|grant(?:s|ed)?|permit(?:s|ted)?|approve(?:s|d)?)\b",
        window,
    )
    passive_authorization = re.search(
        r"\bexplicit\s+(?:permission|authorization)\b.{0,80}?"
        r"\b(?:is|was|has\s+been)\s+(?:granted|given|provided|approved)\b.{0,100}?"
        r"\bby\s+(?:the\s+)?(?:user|project\s+owner|owner)\b",
        window,
    )
    return bool(active_authorization or passive_authorization)


def _repo_has_positive_read_only_assertion(clause: str, repository: str) -> bool:
    window = _repo_specific_window(clause, repository, 120)
    if "read-only" not in window:
        return False
    if re.search(r"(?:\b(?:not|never|no\s+longer|is\s+not|was\s+not)\s+read-only\b|\b(?:isn't|isn’t|wasn't|wasn’t)\s+read-only\b)", window):
        return False
    return bool(re.search(r"\b(?:is|are|as|remain|remains|must\s+remain|treat)\b.*\bread-only\b", window))


def _slash_token_is_prose(normalized: str, match: re.Match[str]) -> bool:
    """Reject slash compounds only when following syntax proves compound-prose usage."""
    after = normalized[match.end():match.end() + 40]
    return bool(re.match(r"\s+(?:metadata|creation|mutation|state|status|result|boundary|restriction|rules?|policy|operations?)\b", after, flags=re.I))


def _repository_identifiers_in_grant_clause(clause: str) -> list[str]:
    if not _has_positive_mutation_grant(clause):
        return []
    quoted_repositories = {value.casefold() for value in QUOTED_REPO_TOKEN.findall(clause)}
    normalized = _normalize_inline_markdown(clause)
    repositories: set[str] = set()
    for match in REPO_TOKEN.finditer(normalized):
        repository = match.group(1)
        before = normalized[max(0, match.start() - 150):match.start()]
        after = normalized[match.end():match.end() + 100]
        mutation_before = bool(re.search(
            rf"\b{MUTATION_TERM}\b"
            r"(?:\s+(?:the|a|an|any|repository|repo|files?|content|code|changes?|branches?|commits?|metadata|access|operations?)){0,6}"
            r"(?:\s+(?:in|to|into|of|on|within|for))?\s*$",
            before,
            flags=re.I,
        ))
        writable_after = bool(re.match(r"\s+(?:is\s+|are\s+)?writable\b", after, flags=re.I))
        passive_mutation_after = bool(re.match(
            rf"\s+(?:(?:may|can)\s+be\s+{MUTATION_TERM}\b|"
            rf"(?:is|are)\s+(?:explicitly\s+)?allowed\s+to\s+be\s+{MUTATION_TERM}\b)",
            after,
            flags=re.I,
        ))
        if not (mutation_before or writable_after or passive_mutation_after):
            continue
        if repository.casefold() not in quoted_repositories and _slash_token_is_prose(normalized, match):
            continue
        local = normalized[max(0, match.start() - 160):match.end() + 180]
        if POSITIVE_AUTH.search(local):
            repositories.add(repository)
    return sorted(repositories)


def _reject_contradictory_repository_mutation_grants(errors: list[str], source: str, policy_text: str) -> None:
    for statement in _logical_markdown_statements(policy_text):
        for clause in _policy_clauses(statement):
            for repository in _repository_identifiers_in_grant_clause(clause):
                if repository == REPOSITORY_FULL_NAME:
                    continue
                if _repo_has_positive_read_only_assertion(clause, repository):
                    continue
                if _repo_has_conditional_user_authorization(clause, repository):
                    continue
                errors.append(f"{source}: contradictory repository mutation authorization in authoritative policy: {repository}")


def validate_policy(root: Path = REPO_ROOT) -> list[str]:
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

        for duplicate in _yaml_lists(anti_stall, "checkpoint_task_statuses"):
            if duplicate != canonical_statuses:
                errors.append(f"docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md: checkpoint task statuses drift; canonical={canonical_statuses}, duplicate={duplicate}")
        for duplicate in _yaml_lists(anti_stall, "terminal_invocation_results"):
            if duplicate != canonical_terminal:
                errors.append(f"docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md: terminal invocation results drift; canonical={canonical_terminal}, duplicate={duplicate}")

        _require_all_declarations(errors, "docs/agents/AGENTS.md", _text_contract_declarations(docs_agents, "Use these checkpoint task statuses only:"), canonical_statuses, "checkpoint task statuses")
        _require_all_declarations(errors, "docs/agents/AGENTS.md", _text_contract_declarations(docs_agents, "Use these terminal invocation results only:"), canonical_terminal, "terminal invocation results")
        _require_all_declarations(errors, "AGENTS.override.md", _inline_backtick_declarations(override, "checkpoint task status:"), canonical_statuses, "checkpoint task statuses")
        _require_all_declarations(errors, "AGENTS.override.md", _inline_backtick_declarations(override, "terminal invocation result:"), canonical_terminal, "terminal invocation results")

        budget_keys = {key: _yaml_int(anti_stall, key) for key in (
            "normal_foreground_runtime_minutes", "large_foreground_runtime_minutes", "no_progress_minutes",
            "max_ci_state_checks_per_exact_head", "max_unchanged_external_state_checks", "terminal_ci_wait_budget_minutes",
            "terminal_ci_minimum_poll_interval_minutes", "max_terminal_ci_state_checks_per_check_generation",
            "max_additional_tasks_after_terminal_entry_task", "minimum_remaining_minutes_to_start_additional_task",
        )}
        for pattern, key in (
            (r"Default to (?P<value>\d+) minutes per foreground invocation", "normal_foreground_runtime_minutes"),
            (r"allow (?P<value>\d+) minutes only when", "large_foreground_runtime_minutes"),
            (r"Stop after (?P<value>\d+) minutes without measurable progress", "no_progress_minutes"),
            (r"exception is capped at (?P<value>\d+) minutes", "terminal_ci_wait_budget_minutes"),
            (r"permits at most (?P<value>\d+) checks per materially new required-check generation", "max_terminal_ci_state_checks_per_check_generation"),
            (r"only when at least (?P<value>\d+) minutes remains", "minimum_remaining_minutes_to_start_additional_task"),
        ):
            _require_regex_value(errors, "AGENTS.override.md", override, pattern, budget_keys[key], key)

        ordinary_checks = budget_keys["max_ci_state_checks_per_exact_head"]
        external_checks = budget_keys["max_unchanged_external_state_checks"]
        if ordinary_checks != external_checks:
            errors.append("ANTI_STALL_AND_EXECUTION_BUDGET.md: root bootstrap combines ordinary CI and external checks, but their canonical limits differ")
        elif ordinary_checks != 2 or "at most twice per exact head" not in override:
            errors.append(f"AGENTS.override.md: ordinary CI/external-state check limit drift; canonical={ordinary_checks}, duplicate marker='at most twice per exact head'")

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
            "AGENTS.override.md": ("exact-head self-review", "real E2E", "required CI on the exact final head", "zero unresolved review threads", "terminal task record", "released ownership"),
            "docs/agents/AGENTS.md": ("exact-head full-diff self-review", "real E2E", "zero unresolved material findings", "task archival", "ownership release"),
            "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md": ("## Mandatory self-review", "## E2E", "## Exact-head CI and Actions economy", "## Related PR hygiene", "## Terminal closeout"),
        }
        completion_source_text = {"AGENTS.override.md": override, "docs/agents/AGENTS.md": docs_agents, "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md": delivery}
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