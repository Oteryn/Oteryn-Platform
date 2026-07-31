#!/usr/bin/env python3
"""Summarize durable agent task state without reading chat history."""

from __future__ import annotations

import argparse
import collections.abc
import dataclasses
import datetime
import json
import pathlib
import re
import sys
import typing

ACTIVE_RAW = {
    "active",
    "in_progress",
    "in-progress",
    "investigating",
    "implementing",
    "validating",
    "running",
}
BLOCKED_RAW = {"blocked"}
WAITING_RAW = {"waiting", "waiting_ci", "waiting_external", "waiting_dependency"}
READY_RAW = {"ready"}
DONE_RAW = {"done", "complete", "completed", "merged", "closed", "archived"}
IGNORED_FILENAMES = {
    "README.md",
    "TASK_TEMPLATE.md",
    "CONTEXT_HANDOFF.md",
    "EXECUTION_PROTOCOL.md",
}
V2_REQUIRED_FIELDS = ("task_kind", "context_pressure", "decomposition_decision")


@dataclasses.dataclass(frozen=True)
class Task:
    task_id: str
    lane: str
    path: str
    raw_status: str
    state: str
    updated_at: str
    age_minutes: int | None
    branch: str
    pr: str
    phase: str
    execution_mode: str
    next_action: str
    blocker: str
    task_kind: str
    context_pressure: str
    context_growth: str
    decomposition_decision: str
    estimate_confidence: str
    session_id: str
    session_role: str
    validation_level: str
    policy_version: str
    policy_status: str
    session_rotation_count: int
    heavy_validation_runs: int
    stale_takeover_count: int
    human_interruptions: int


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build a compact Control Room view from durable task records."
    )
    parser.add_argument(
        "--config",
        default="docs/agents/PROJECT_LANES.json",
        help="Path to the repository lane configuration.",
    )
    parser.add_argument(
        "--format",
        choices=("markdown", "json"),
        default="markdown",
        help="Output format.",
    )
    parser.add_argument(
        "--lane",
        action="append",
        default=[],
        help="Only include the selected lane. May be repeated.",
    )
    parser.add_argument(
        "--stale-after-minutes",
        type=int,
        default=None,
        help="Override the configured stale threshold.",
    )
    parser.add_argument(
        "--now",
        default=None,
        help="ISO-8601 timestamp used for deterministic runs.",
    )
    parser.add_argument(
        "--fail-on-stale",
        action="store_true",
        help="Exit with status 2 when at least one task is stale.",
    )
    return parser.parse_args()


def parse_iso(value: str) -> datetime.datetime | None:
    value = value.strip().strip("\"'")
    if not value:
        return None
    if value.endswith("Z"):
        value = value[:-1] + "+00:00"
    try:
        parsed = datetime.datetime.fromisoformat(value)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=datetime.UTC)
    return parsed.astimezone(datetime.UTC)


def scalar_map(text: str) -> dict[str, str]:
    """Read simple top-level YAML scalars from frontmatter and checkpoint blocks."""
    values: dict[str, str] = {}

    if text.startswith("---"):
        end = text.find("\n---", 3)
        if end >= 0:
            read_scalar_lines(text[3:end].splitlines(), values)

    match = re.search(r"(?m)^## Context checkpoint\s*$", text)
    if match:
        remainder = text[match.end() :]
        fence = re.search(r"```(?:yaml|yml)\s*\n", remainder, re.IGNORECASE)
        if fence:
            block_end = remainder.find("```", fence.end())
            if block_end >= 0:
                read_scalar_lines(remainder[fence.end() : block_end].splitlines(), values)

    return values


def read_scalar_lines(
    lines: collections.abc.Iterable[str], values: dict[str, str]
) -> None:
    for raw in lines:
        if not raw or raw[0].isspace() or raw.lstrip().startswith("#") or ":" not in raw:
            continue
        key, value = raw.split(":", 1)
        key = key.strip()
        value = value.strip().strip("\"'")
        if key and value and value not in {"[]", "{}"}:
            values[key] = value


def first_list_item(text: str, key: str) -> str:
    pattern = re.compile(
        rf"(?ms)^{re.escape(key)}:\s*\n(?P<body>(?:[ \t]+- .*(?:\n|$))*)"
    )
    match = pattern.search(text)
    if not match:
        return ""
    item = re.search(r"(?m)^[ \t]+- (.+)$", match.group("body"))
    return item.group(1).strip() if item else ""


def string_list(value: object, label: str) -> list[str]:
    if value is None:
        return []
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise ValueError(f"{label} must be a list of strings")
    return typing.cast(list[str], value)


def object_map(value: object, label: str) -> dict[str, object]:
    if not isinstance(value, dict) or not all(isinstance(key, str) for key in value):
        raise ValueError(f"{label} must be an object")
    return typing.cast(dict[str, object], value)


def object_map_list(value: object, label: str) -> list[dict[str, object]]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    return [object_map(item, f"{label} item") for item in value]


def non_negative_int(values: dict[str, str], key: str) -> int:
    raw = values.get(key, "").strip()
    if not raw:
        return 0
    try:
        parsed = int(raw)
    except ValueError:
        return 0
    return max(0, parsed)


def infer_task_id(path: pathlib.Path, text: str, values: dict[str, str]) -> str:
    for key in ("task_id", "id"):
        if values.get(key):
            return values[key]
    pattern = re.compile(
        r"\b(?:FTAI|OTH|OTERYN|CAN|OTC2?|OTS)-[A-Z0-9][A-Z0-9-]*\b", re.I
    )
    match = pattern.search(path.stem) or pattern.search(text)
    return match.group(0) if match else path.stem


def infer_lane(
    task_id: str,
    path: pathlib.Path,
    text: str,
    values: dict[str, str],
    lanes: list[dict[str, object]],
    default_lane: str,
) -> str:
    explicit = values.get("project_lane") or values.get("lane") or values.get("project")
    valid_ids = {str(lane["id"]) for lane in lanes}
    if explicit in valid_ids:
        return explicit

    haystack = f"{task_id} {path.as_posix()} {text[:12000]}".casefold()
    for lane in lanes:
        lane_id = str(lane["id"])
        keywords = [
            item.casefold()
            for item in string_list(lane.get("match_keywords"), "lane match_keywords")
        ]
        if any(keyword and keyword in haystack for keyword in keywords):
            return lane_id

    upper_id = task_id.upper()
    matching: list[tuple[int, str]] = []
    for lane in lanes:
        lane_id = str(lane["id"])
        for prefix in string_list(lane.get("task_prefixes"), "lane task_prefixes"):
            prefix_value = prefix.upper()
            if upper_id.startswith(prefix_value + "-"):
                matching.append((len(prefix_value), lane_id))
    if matching:
        return max(matching)[1]
    return default_lane


def normalize_state(raw_status: str, age_minutes: int | None, stale_after: int) -> str:
    value = raw_status.strip().casefold().replace(" ", "_")
    if value in DONE_RAW:
        return "DONE"
    if value in BLOCKED_RAW:
        return "BLOCKED"
    if value in WAITING_RAW:
        return "WAITING"
    if value in READY_RAW:
        return "READY"
    if value in ACTIVE_RAW:
        if age_minutes is not None and age_minutes > stale_after:
            return "STALE"
        return "RUNNING"
    return "UNKNOWN"


def infer_policy_status(values: dict[str, str]) -> str:
    explicit = values.get("policy_version", "").strip().casefold()
    if explicit in {"2", "v2"}:
        return "V2"
    if all(values.get(field, "").strip() for field in V2_REQUIRED_FIELDS):
        return "V2"
    return "LEGACY"


def task_from_file(
    path: pathlib.Path,
    lanes: list[dict[str, object]],
    default_lane: str,
    now: datetime.datetime,
    stale_after: int,
) -> Task:
    text = path.read_text(encoding="utf-8")
    values = scalar_map(text)
    task_id = infer_task_id(path, text, values)
    lane = infer_lane(task_id, path, text, values, lanes, default_lane)
    updated_raw = values.get("updated_at") or values.get("updated") or ""
    updated = parse_iso(updated_raw)
    age_minutes = None
    if updated:
        age_minutes = max(0, int((now - updated).total_seconds() // 60))
    raw_status = values.get("status", "unknown")
    state = normalize_state(raw_status, age_minutes, stale_after)
    blocker = values.get("blocker") or first_list_item(text, "blockers")
    if blocker.strip().casefold() in {"none", "n/a", "no", "[]"}:
        blocker = ""

    return Task(
        task_id=task_id,
        lane=lane,
        path=path.as_posix(),
        raw_status=raw_status,
        state=state,
        updated_at=updated_raw,
        age_minutes=age_minutes,
        branch=values.get("branch", ""),
        pr=values.get("pr", ""),
        phase=values.get("phase", ""),
        execution_mode=values.get("execution_mode", ""),
        next_action=values.get("next_action", ""),
        blocker=blocker,
        task_kind=values.get("task_kind", ""),
        context_pressure=values.get("context_pressure", ""),
        context_growth=values.get("context_growth", ""),
        decomposition_decision=values.get("decomposition_decision", ""),
        estimate_confidence=values.get("estimate_confidence", ""),
        session_id=values.get("session_id", ""),
        session_role=values.get("session_role", ""),
        validation_level=values.get("validation_level", ""),
        policy_version=values.get("policy_version", ""),
        policy_status=infer_policy_status(values),
        session_rotation_count=non_negative_int(values, "session_rotation_count"),
        heavy_validation_runs=non_negative_int(values, "heavy_validation_runs"),
        stale_takeover_count=non_negative_int(values, "stale_takeover_count"),
        human_interruptions=non_negative_int(values, "human_interruptions"),
    )


def load_tasks(
    config: dict[str, object], now: datetime.datetime, stale_after: int
) -> list[Task]:
    lanes = object_map_list(config.get("lanes"), "lanes")
    if not lanes:
        raise ValueError("configuration must define at least one lane")
    default_lane = str(config.get("default_lane") or lanes[0]["id"])
    task_globs = string_list(config.get("task_globs"), "task_globs")
    if not task_globs:
        task_globs = ["docs/agents/tasks/active/*.md"]

    paths: set[pathlib.Path] = set()
    for pattern in task_globs:
        paths.update(pathlib.Path.cwd().glob(pattern))

    return [
        task_from_file(path, lanes, default_lane, now, stale_after)
        for path in sorted(paths)
        if path.name not in IGNORED_FILENAMES and "/archive/" not in path.as_posix()
    ]


def task_dict(task: Task) -> dict[str, object]:
    return {
        "task_id": task.task_id,
        "lane": task.lane,
        "path": task.path,
        "raw_status": task.raw_status,
        "state": task.state,
        "updated_at": task.updated_at,
        "age_minutes": task.age_minutes,
        "branch": task.branch,
        "pr": task.pr,
        "phase": task.phase,
        "execution_mode": task.execution_mode,
        "next_action": task.next_action,
        "blocker": task.blocker,
        "task_kind": task.task_kind,
        "context_pressure": task.context_pressure,
        "context_growth": task.context_growth,
        "decomposition_decision": task.decomposition_decision,
        "estimate_confidence": task.estimate_confidence,
        "session_id": task.session_id,
        "session_role": task.session_role,
        "validation_level": task.validation_level,
        "policy_version": task.policy_version,
        "policy_status": task.policy_status,
        "session_rotation_count": task.session_rotation_count,
        "heavy_validation_runs": task.heavy_validation_runs,
        "stale_takeover_count": task.stale_takeover_count,
        "human_interruptions": task.human_interruptions,
    }


def coordination_metrics(tasks: list[Task]) -> dict[str, int]:
    return {
        "active_tasks": sum(task.state == "RUNNING" for task in tasks),
        "active_sessions": sum(
            task.state == "RUNNING" and bool(task.session_id) for task in tasks
        ),
        "legacy_tasks": sum(task.policy_status == "LEGACY" for task in tasks),
        "session_rotations": sum(task.session_rotation_count for task in tasks),
        "task_splits": sum(
            task.decomposition_decision.strip().casefold() == "split" for task in tasks
        ),
        "heavy_validation_runs": sum(task.heavy_validation_runs for task in tasks),
        "stale_takeovers": sum(task.stale_takeover_count for task in tasks),
        "human_interruptions": sum(task.human_interruptions for task in tasks),
    }


def task_details(task: Task) -> list[str]:
    candidates = (
        ("kind", task.task_kind),
        ("phase", task.phase),
        ("context", task.context_pressure),
        ("decomposition", task.decomposition_decision),
        ("role", task.session_role),
        ("mode", task.execution_mode),
        ("validation", task.validation_level),
        ("PR", task.pr),
    )
    details = [f"{label}={value}" for label, value in candidates if value]
    if task.age_minutes is not None:
        details.append(f"age={task.age_minutes}m")
    if task.policy_status == "LEGACY":
        details.append("policy=legacy")
    return details


def append_task(lines: list[str], task: Task, state: str) -> None:
    details = task_details(task)
    suffix = f" ({', '.join(details)})" if details else ""
    lines.append(f"- `{task.task_id}`{suffix}")
    if task.blocker and state in {"BLOCKED", "WAITING", "STALE"}:
        lines.append(f"  - blocker: {task.blocker}")
    if task.next_action:
        lines.append(f"  - next: {task.next_action}")


def append_lane(
    lines: list[str], lane: str, lane_tasks: list[Task], state_order: tuple[str, ...]
) -> None:
    counts = {state: sum(task.state == state for task in lane_tasks) for state in state_order}
    summary = ", ".join(f"{state} {count}" for state, count in counts.items() if count)
    lines.extend([f"## {lane}", "", summary or "No active task records.", ""])

    for state in state_order:
        selected = [task for task in lane_tasks if task.state == state]
        if not selected:
            continue
        lines.extend([f"### {state}", ""])
        for task in selected:
            append_task(lines, task, state)
        lines.append("")


def markdown(config: dict[str, object], tasks: list[Task], stale_after: int) -> str:
    lanes = [
        str(lane["id"]) for lane in object_map_list(config.get("lanes"), "lanes")
    ]
    rollout = object_map(config.get("rollout", {}), "rollout")
    metrics_line = ", ".join(
        f"{key}={value}" for key, value in coordination_metrics(tasks).items()
    )
    lines = [
        f"# Control Room — {config.get('repository', 'repository')}",
        "",
        f"Stale threshold: {stale_after} minutes. "
        "STALE is derived; it does not rewrite task files.",
        f"Policy rollout: {rollout.get('enforcement_mode', 'advisory')}.",
        f"Metrics: {metrics_line}",
        "",
    ]
    state_order = ("STALE", "BLOCKED", "WAITING", "RUNNING", "READY", "DONE", "UNKNOWN")
    for lane in lanes:
        append_lane(lines, lane, [task for task in tasks if task.lane == lane], state_order)
    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    args = parse_args()
    config_path = pathlib.Path(args.config)
    config = json.loads(config_path.read_text(encoding="utf-8"))
    execution = object_map(config.get("execution", {}), "execution")
    configured_stale_value = execution.get("stale_after_minutes", 45)
    if not isinstance(configured_stale_value, int):
        raise ValueError("execution.stale_after_minutes must be an integer")
    stale_after = args.stale_after_minutes or configured_stale_value
    now = parse_iso(args.now) if args.now else datetime.datetime.now(datetime.UTC)
    if now is None:
        raise SystemExit("--now must be a valid ISO-8601 timestamp")

    tasks = load_tasks(config, now, stale_after)
    if args.lane:
        selected = set(args.lane)
        tasks = [task for task in tasks if task.lane in selected]

    if args.format == "json":
        payload = {
            "repository": config.get("repository"),
            "schema_version": config.get("schema_version"),
            "generated_at": now.isoformat(),
            "stale_after_minutes": stale_after,
            "rollout": config.get("rollout", {}),
            "metrics": coordination_metrics(tasks),
            "tasks": [task_dict(task) for task in tasks],
        }
        print(json.dumps(payload, indent=2, ensure_ascii=False))
    else:
        print(markdown(config, tasks, stale_after), end="")

    return 2 if args.fail_on_stale and any(task.state == "STALE" for task in tasks) else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"control-room error: {exc}", file=sys.stderr)
        raise SystemExit(1)
