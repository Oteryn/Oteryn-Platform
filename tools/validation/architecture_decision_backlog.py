#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import json
import re
from pathlib import Path
from typing import Any

BACKLOG_PATH = Path("docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json")
PROGRAMME_PATH = Path("docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md")
SCHEMA_VERSION = 1
REGISTRY_NAME = "Oteryn Platform Architecture Decision Backlog"
ACTIVE_STATES = ["analysis_ready", "blocked", "decision_required", "deferred", "discovered"]
SEVERITIES = {"low", "medium", "high", "critical"}
EVIDENCE_STATES = {"PROVEN", "DERIVED", "UNKNOWN", "CONFLICT"}
ID_RE = re.compile(r"^ARCH-DEC-\d{4}$")
ADR_RE = re.compile(r"^docs/architecture/adr/\d{4}-[a-z0-9]+(?:-[a-z0-9]+)*\.md$")
PROJECTION_RE = re.compile(r"^active_architecture_decision_ids:\s*(\[[^\n]*\])\s*$", re.M)
NEXT_ACTION_RE = re.compile(r"^next_action:\s*\S.*$", re.M)

ROOT_FIELDS = {"schema_version", "registry_name", "authority", "lifecycle", "records"}
AUTHORITY_FIELDS = {"role", "accepted_decision_authority", "canonical_routing", "non_authority_statement"}
LIFECYCLE_FIELDS = {"active_states", "terminal_handling"}
RECORD_FIELDS = {
    "decision_id", "title", "state", "severity", "decision_owner",
    "recommended_owner", "problem_statement", "decision_question",
    "blocking_owner_question", "canonical_owners", "evidence", "options",
    "recommendation", "dependencies", "blockers", "references", "created_at",
    "updated_at", "implementation_authorized", "deferral_reason", "revisit_trigger",
}
DEPENDENCY_FIELDS = {"decision_ids", "issues", "local_paths"}
REFERENCE_FIELDS = {"issue", "proposed_adr", "related_prs"}
OPTION_FIELDS = {"option_id", "title", "description", "trade_offs"}


def _fields(value: object, expected: set[str], context: str, errors: list[str]) -> dict[str, Any]:
    if not isinstance(value, dict):
        errors.append(f"{context}: expected object")
        return {}
    unknown = sorted(set(value) - expected)
    missing = sorted(expected - set(value))
    if unknown:
        errors.append(f"{context}: unknown fields: {', '.join(unknown)}")
    if missing:
        errors.append(f"{context}: missing fields: {', '.join(missing)}")
    return value


def _text(value: object, context: str, errors: list[str], *, optional: bool = False) -> str | None:
    if value is None and optional:
        return None
    if not isinstance(value, str) or not value.strip():
        errors.append(f"{context}: expected non-empty string")
        return None
    return value


def _texts(value: object, context: str, errors: list[str], *, nonempty: bool = False) -> list[str]:
    if not isinstance(value, list):
        errors.append(f"{context}: expected array")
        return []
    if nonempty and not value:
        errors.append(f"{context}: expected at least one item")
    result: list[str] = []
    for index, item in enumerate(value):
        text = _text(item, f"{context}[{index}]", errors)
        if text is not None:
            result.append(text)
    return result


def _integers(value: object, context: str, errors: list[str]) -> list[int]:
    if not isinstance(value, list):
        errors.append(f"{context}: expected array")
        return []
    result: list[int] = []
    for index, item in enumerate(value):
        if isinstance(item, bool) or not isinstance(item, int) or item <= 0:
            errors.append(f"{context}[{index}]: expected positive integer")
        else:
            result.append(item)
    if len(result) != len(set(result)):
        errors.append(f"{context}: duplicate integers are not allowed")
    return result


def _normal(value: str) -> str:
    return " ".join(re.findall(r"[a-z0-9]+", value.casefold()))


def _path(root: Path, value: object, context: str, errors: list[str]) -> str | None:
    text = _text(value, context, errors)
    if text is None:
        return None
    candidate = Path(text)
    if candidate.is_absolute() or ".." in candidate.parts:
        errors.append(f"{context}: path must be repository-relative without '..': {text}")
    elif not (root / candidate).exists():
        errors.append(f"{context}: local path does not exist: {text}")
    return text


def _date(value: object, context: str, errors: list[str]) -> dt.date | None:
    text = _text(value, context, errors)
    if text is None:
        return None
    try:
        return dt.date.fromisoformat(text)
    except ValueError:
        errors.append(f"{context}: expected ISO date YYYY-MM-DD")
        return None


def _canonical(data: object) -> str:
    return json.dumps(data, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def load(path: Path) -> tuple[dict[str, Any] | None, list[str]]:
    if not path.is_file():
        return None, [f"missing architecture decision backlog: {path.as_posix()}"]
    content = path.read_text(encoding="utf-8")
    try:
        data = json.loads(content)
    except json.JSONDecodeError as exc:
        return None, [f"{path.as_posix()}: invalid JSON at {exc.lineno}:{exc.colno}: {exc.msg}"]
    if not isinstance(data, dict):
        return None, [f"{path.as_posix()}: root must be an object"]
    errors = [] if content == _canonical(data) else [
        f"{path.as_posix()}: JSON must use canonical sorted-key, two-space serialization with one trailing newline"
    ]
    return data, errors


def _record(root: Path, value: object, index: int, errors: list[str]) -> dict[str, Any]:
    context = f"records[{index}]"
    record = _fields(value, RECORD_FIELDS, context, errors)
    identifier = _text(record.get("decision_id"), f"{context}.decision_id", errors)
    if identifier is not None and not ID_RE.fullmatch(identifier):
        errors.append(f"{context}.decision_id: expected ARCH-DEC-NNNN, got {identifier!r}")
    for field in ("title", "decision_owner", "recommended_owner", "problem_statement", "decision_question"):
        _text(record.get(field), f"{context}.{field}", errors)

    state = record.get("state")
    if state not in ACTIVE_STATES:
        errors.append(f"{context}.state: unsupported or terminal lifecycle value {state!r}")
    if record.get("severity") not in SEVERITIES:
        errors.append(f"{context}.severity: unsupported value {record.get('severity')!r}")

    owners = _texts(record.get("canonical_owners"), f"{context}.canonical_owners", errors, nonempty=True)
    if len(owners) != len(set(owners)):
        errors.append(f"{context}.canonical_owners: duplicate paths are not allowed")
    for owner_index, owner in enumerate(owners):
        _path(root, owner, f"{context}.canonical_owners[{owner_index}]", errors)

    evidence = _fields(record.get("evidence"), EVIDENCE_STATES, f"{context}.evidence", errors)
    seen: dict[str, str] = {}
    for label in sorted(EVIDENCE_STATES):
        for fact in _texts(evidence.get(label), f"{context}.evidence.{label}", errors):
            normalized = _normal(fact)
            if not normalized:
                errors.append(f"{context}.evidence.{label}: fact normalizes to empty")
            elif normalized in seen:
                errors.append(
                    f"{context}.evidence: normalized fact appears more than once: "
                    f"{fact!r} ({seen[normalized]} and {label})"
                )
            else:
                seen[normalized] = label

    options = record.get("options")
    if not isinstance(options, list):
        errors.append(f"{context}.options: expected array")
        options = []
    option_ids: list[str] = []
    for option_index, raw in enumerate(options):
        option_context = f"{context}.options[{option_index}]"
        option = _fields(raw, OPTION_FIELDS, option_context, errors)
        option_id = _text(option.get("option_id"), f"{option_context}.option_id", errors)
        if option_id is not None:
            option_ids.append(option_id)
        _text(option.get("title"), f"{option_context}.title", errors)
        _text(option.get("description"), f"{option_context}.description", errors)
        _texts(option.get("trade_offs"), f"{option_context}.trade_offs", errors, nonempty=True)
    if len(option_ids) != len(set(option_ids)):
        errors.append(f"{context}.options: duplicate option_id values")

    recommendation = _text(record.get("recommendation"), f"{context}.recommendation", errors, optional=True)
    owner_question = _text(
        record.get("blocking_owner_question"),
        f"{context}.blocking_owner_question",
        errors,
        optional=True,
    )
    blockers = _texts(record.get("blockers"), f"{context}.blockers", errors)
    deferral_reason = _text(record.get("deferral_reason"), f"{context}.deferral_reason", errors, optional=True)
    revisit_trigger = _text(record.get("revisit_trigger"), f"{context}.revisit_trigger", errors, optional=True)

    if state in {"analysis_ready", "decision_required"}:
        if len(options) < 2:
            errors.append(f"{context}: {state} requires at least two options")
        if recommendation is None:
            errors.append(f"{context}: {state} requires a recommendation")
    if state == "decision_required":
        if owner_question is None:
            errors.append(f"{context}: decision_required requires one blocking_owner_question")
    elif record.get("blocking_owner_question") is not None:
        errors.append(f"{context}: blocking_owner_question is only allowed for decision_required")
    if state == "blocked":
        if not blockers:
            errors.append(f"{context}: blocked requires at least one blocker")
    elif blockers:
        errors.append(f"{context}: blockers are only allowed for blocked records")
    if state == "deferred":
        if deferral_reason is None:
            errors.append(f"{context}: deferred requires deferral_reason")
        if revisit_trigger is None:
            errors.append(f"{context}: deferred requires revisit_trigger")
    elif record.get("deferral_reason") is not None or record.get("revisit_trigger") is not None:
        errors.append(f"{context}: deferral fields are only allowed for deferred records")

    if record.get("implementation_authorized") is not False:
        errors.append(f"{context}.implementation_authorized: must be false")

    dependencies = _fields(
        record.get("dependencies"), DEPENDENCY_FIELDS, f"{context}.dependencies", errors
    )
    dependency_ids = _texts(
        dependencies.get("decision_ids"), f"{context}.dependencies.decision_ids", errors
    )
    _integers(dependencies.get("issues"), f"{context}.dependencies.issues", errors)
    dependency_paths = _texts(
        dependencies.get("local_paths"), f"{context}.dependencies.local_paths", errors
    )
    for path_index, path in enumerate(dependency_paths):
        _path(root, path, f"{context}.dependencies.local_paths[{path_index}]", errors)

    references = _fields(record.get("references"), REFERENCE_FIELDS, f"{context}.references", errors)
    issue = references.get("issue")
    if isinstance(issue, bool) or not isinstance(issue, int) or issue <= 0:
        errors.append(f"{context}.references.issue: expected positive integer")
    _integers(references.get("related_prs"), f"{context}.references.related_prs", errors)
    proposed = references.get("proposed_adr")
    if proposed is not None:
        text = _text(proposed, f"{context}.references.proposed_adr", errors)
        if text is not None:
            if not ADR_RE.fullmatch(text):
                errors.append(f"{context}.references.proposed_adr: malformed ADR path {text!r}")
            else:
                _path(root, text, f"{context}.references.proposed_adr", errors)

    created = _date(record.get("created_at"), f"{context}.created_at", errors)
    updated = _date(record.get("updated_at"), f"{context}.updated_at", errors)
    if created is not None and updated is not None and updated < created:
        errors.append(f"{context}: updated_at precedes created_at")

    return {
        "decision_id": identifier,
        "owners": owners,
        "question": record.get("decision_question"),
        "dependency_ids": dependency_ids,
    }


def _programme(root: Path, ids: list[str], errors: list[str]) -> None:
    path = root / PROGRAMME_PATH
    if not path.is_file():
        errors.append(f"missing architecture programme: {PROGRAMME_PATH.as_posix()}")
        return
    content = path.read_text(encoding="utf-8")
    if re.search(r"^decision_backlog:\s*$", content, re.M):
        errors.append(f"{PROGRAMME_PATH.as_posix()}: full decision_backlog is forbidden")
    matches = PROJECTION_RE.findall(content)
    if len(matches) != 1:
        errors.append(
            f"{PROGRAMME_PATH.as_posix()}: expected exactly one active_architecture_decision_ids array"
        )
    else:
        try:
            projection = json.loads(matches[0])
        except json.JSONDecodeError as exc:
            errors.append(f"{PROGRAMME_PATH.as_posix()}: invalid active ID projection: {exc.msg}")
        else:
            if not isinstance(projection, list) or not all(isinstance(item, str) for item in projection):
                errors.append(f"{PROGRAMME_PATH.as_posix()}: active ID projection must be a string array")
            elif projection != ids:
                errors.append(
                    f"{PROGRAMME_PATH.as_posix()}: active ID projection {projection!r} "
                    f"does not exactly match registry IDs {ids!r}"
                )
    if len(NEXT_ACTION_RE.findall(content)) != 1:
        errors.append(f"{PROGRAMME_PATH.as_posix()}: expected exactly one non-empty next_action")


def validate_repository(root: Path) -> list[str]:
    root = root.resolve()
    data, errors = load(root / BACKLOG_PATH)
    if data is None:
        return errors

    data = _fields(data, ROOT_FIELDS, "root", errors)
    if data.get("schema_version") != SCHEMA_VERSION:
        errors.append(
            f"root.schema_version: supported value is {SCHEMA_VERSION}, "
            f"got {data.get('schema_version')!r}"
        )
    if data.get("registry_name") != REGISTRY_NAME:
        errors.append(
            f"root.registry_name: expected {REGISTRY_NAME!r}, got {data.get('registry_name')!r}"
        )

    authority = _fields(data.get("authority"), AUTHORITY_FIELDS, "root.authority", errors)
    if authority.get("role") != "unresolved_decision_inventory":
        errors.append("root.authority.role: must be 'unresolved_decision_inventory'")
    for field in ("accepted_decision_authority", "canonical_routing"):
        _path(root, authority.get(field), f"root.authority.{field}", errors)
    non_authority = _text(
        authority.get("non_authority_statement"),
        "root.authority.non_authority_statement",
        errors,
    )
    if non_authority is not None:
        normalized = non_authority.casefold()
        for term in ("accepted", "implementation", "activation"):
            if term not in normalized:
                errors.append(
                    "root.authority.non_authority_statement: "
                    f"must explicitly deny {term} authority"
                )

    lifecycle = _fields(data.get("lifecycle"), LIFECYCLE_FIELDS, "root.lifecycle", errors)
    if lifecycle.get("active_states") != ACTIVE_STATES:
        errors.append("root.lifecycle.active_states: must exactly equal supported active states")
    _text(lifecycle.get("terminal_handling"), "root.lifecycle.terminal_handling", errors)

    raw_records = data.get("records")
    if not isinstance(raw_records, list):
        errors.append("root.records: expected array")
        raw_records = []
    records = [_record(root, value, index, errors) for index, value in enumerate(raw_records)]
    ids = [record["decision_id"] for record in records if isinstance(record["decision_id"], str)]
    if len(ids) != len(set(ids)):
        duplicates = sorted(identifier for identifier in set(ids) if ids.count(identifier) > 1)
        errors.append(f"duplicate decision IDs: {', '.join(duplicates)}")
    if ids != sorted(ids):
        errors.append("root.records: records must be ordered by decision_id")

    known = set(ids)
    obligations: dict[tuple[str, str], str] = {}
    for index, record in enumerate(records):
        identifier = record["decision_id"] or f"records[{index}]"
        for dependency in record["dependency_ids"]:
            if dependency not in known:
                errors.append(f"{identifier}: dependency decision ID does not exist: {dependency}")
            elif dependency == identifier:
                errors.append(f"{identifier}: decision cannot depend on itself")
        if record["owners"] and isinstance(record["question"], str):
            key = (record["owners"][0], _normal(record["question"]))
            if key in obligations:
                errors.append(
                    f"duplicate unresolved obligation for primary owner {key[0]}: "
                    f"{obligations[key]} and {identifier}"
                )
            else:
                obligations[key] = str(identifier)

    _programme(root, ids, errors)
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate the architecture decision backlog.")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    errors = validate_repository(args.root)
    data, _ = load(args.root.resolve() / BACKLOG_PATH)
    count = len(data.get("records", [])) if isinstance(data, dict) else 0
    result = {
        "active_decision_count": count,
        "errors": errors,
        "result": "PASS" if not errors else "FAIL",
        "schema_version": SCHEMA_VERSION,
    }
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    elif errors:
        print("Architecture decision backlog validation failed:")
        for error in errors:
            print(f"- {error}")
    else:
        print(f"Architecture decision backlog validation passed: {count} active records.")
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
