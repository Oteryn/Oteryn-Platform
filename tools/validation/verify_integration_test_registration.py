#!/usr/bin/env python3
"""Fail closed when Integration tests are not bound to an executable proving workflow."""

from __future__ import annotations

import json
import re
import shlex
import textwrap
from pathlib import Path
from typing import Any


REGISTRY_PATH = Path("tests/Integration/REGISTRY.json")
INTEGRATION_ROOT = Path("tests/Integration")
WORKFLOW_ROOT = Path(".github/workflows")


class RegistrationError(RuntimeError):
    """Raised when the Integration-test execution registry is incomplete or stale."""


def _load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise RegistrationError(f"missing integration-test registry: {path.as_posix()}") from exc
    except json.JSONDecodeError as exc:
        raise RegistrationError(
            f"invalid JSON in {path.as_posix()}: line {exc.lineno} column {exc.colno}: {exc.msg}"
        ) from exc


def _non_empty_string(value: object, field: str, test_path: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise RegistrationError(f"{test_path}: {field} must be a non-empty string")
    return value


def _relative_file(root: Path, value: str, field: str, test_path: str) -> Path:
    relative = Path(value)
    if relative.is_absolute() or ".." in relative.parts:
        raise RegistrationError(f"{test_path}: {field} must be a repository-relative path")
    resolved_root = root.resolve()
    resolved = (resolved_root / relative).resolve()
    try:
        resolved.relative_to(resolved_root)
    except ValueError as exc:
        raise RegistrationError(f"{test_path}: {field} escapes repository root: {value}") from exc
    if not resolved.is_file():
        raise RegistrationError(f"{test_path}: {field} does not exist: {value}")
    return resolved


def discover_integration_tests(root: Path) -> list[str]:
    integration_root = root / INTEGRATION_ROOT
    if not integration_root.is_dir():
        return []
    return sorted(
        path.relative_to(root).as_posix()
        for path in integration_root.rglob("*Test.php")
        if path.is_file()
    )


def _mapping_child_block(text: str, root_key: str, child_key: str) -> str | None:
    """Return one direct child block from a top-level YAML mapping."""

    lines = text.splitlines()
    root_index: int | None = None
    for index, line in enumerate(lines):
        if line == f"{root_key}:":
            root_index = index
            break
    if root_index is None:
        return None

    root_end = len(lines)
    for index in range(root_index + 1, len(lines)):
        line = lines[index]
        if line and not line.startswith((" ", "\t", "#")):
            root_end = index
            break

    child_prefix = f"  {child_key}:"
    child_index: int | None = None
    for index in range(root_index + 1, root_end):
        line = lines[index]
        if line.startswith(child_prefix) and line[len(child_prefix) :].strip() == "":
            child_index = index
            break
    if child_index is None:
        return None

    child_end = root_end
    for index in range(child_index + 1, root_end):
        line = lines[index]
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip(" "))
        if indent <= 2:
            child_end = index
            break
    return "\n".join(lines[child_index:child_end])


def _direct_child_mapping_block(block_text: str, child_key: str) -> str | None:
    """Return a direct mapping child from an already isolated YAML block."""

    lines = block_text.splitlines()
    if not lines:
        return None
    parent_indent = len(lines[0]) - len(lines[0].lstrip(" "))
    child_indent = parent_indent + 2
    prefix = " " * child_indent + f"{child_key}:"

    child_index: int | None = None
    for index, line in enumerate(lines[1:], start=1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip(" "))
        if indent <= parent_indent:
            break
        if indent == child_indent and line.startswith(prefix) and line[len(prefix) :].strip() == "":
            child_index = index
            break
    if child_index is None:
        return None

    child_end = len(lines)
    for index in range(child_index + 1, len(lines)):
        line = lines[index]
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip(" "))
        if indent <= child_indent:
            child_end = index
            break
    return "\n".join(lines[child_index:child_end])


def _mapping_has_direct_key(block_text: str | None, key: str) -> bool:
    if block_text is None:
        return False
    lines = block_text.splitlines()
    if not lines:
        return False
    parent_indent = len(lines[0]) - len(lines[0].lstrip(" "))
    child_indent = parent_indent + 2
    prefix = " " * child_indent + f"{key}:"
    for line in lines[1:]:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip(" "))
        if indent <= parent_indent:
            break
        if indent == child_indent and line.startswith(prefix):
            return True
    return False


def _strip_yaml_comment(line: str) -> str:
    """Strip an unquoted YAML/shell comment from one conservative line."""

    single_quoted = False
    double_quoted = False
    escaped = False
    for index, char in enumerate(line):
        if char == "\\" and double_quoted and not escaped:
            escaped = True
            continue
        if char == '"' and not single_quoted and not escaped:
            double_quoted = not double_quoted
        elif char == "'" and not double_quoted:
            single_quoted = not single_quoted
        elif char == "#" and not single_quoted and not double_quoted and (
            index == 0 or line[index - 1].isspace()
        ):
            return line[:index].rstrip()
        escaped = False
    return line.rstrip()


def _event_contains_trigger_marker(event: str, event_block: str, trigger: str) -> bool:
    marker = trigger.strip()
    input_match = re.fullmatch(r"([A-Za-z0-9_-]+):", marker)
    if event == "workflow_dispatch" and input_match is not None:
        inputs_block = _direct_child_mapping_block(event_block, "inputs")
        if inputs_block is None:
            return False
        return _direct_child_mapping_block(inputs_block, input_match.group(1)) is not None

    executable_lines = []
    for line in event_block.splitlines()[1:]:
        if line.lstrip().startswith("#"):
            continue
        code = _strip_yaml_comment(line)
        if code.strip():
            executable_lines.append(code)
    return marker in "\n".join(executable_lines)


def _mapping_direct_scalar(block_text: str, key: str) -> str | None:
    lines = block_text.splitlines()
    if not lines:
        return None
    parent_indent = len(lines[0]) - len(lines[0].lstrip(" "))
    child_indent = parent_indent + 2
    prefix = " " * child_indent + f"{key}:"
    multiline_indicators = {"|", "|-", "|+", ">", ">-", ">+"}

    for line in lines[1:]:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip(" "))
        if indent <= parent_indent:
            break
        if indent != child_indent or not line.startswith(prefix):
            continue
        value = _strip_yaml_comment(line[len(prefix) :]).strip()
        if not value or value in multiline_indicators:
            return None
        return value
    return None


def _direct_sequence_item_blocks(block_text: str, key: str) -> list[str]:
    sequence = _direct_child_mapping_block(block_text, key)
    if sequence is None:
        return []
    lines = sequence.splitlines()
    parent_indent = len(lines[0]) - len(lines[0].lstrip(" "))
    item_indent = parent_indent + 2
    starts: list[int] = []
    for index, line in enumerate(lines[1:], start=1):
        if line.startswith(" " * item_indent + "- "):
            starts.append(index)
    blocks: list[str] = []
    for position, start in enumerate(starts):
        end = starts[position + 1] if position + 1 < len(starts) else len(lines)
        blocks.append("\n".join(lines[start:end]))
    return blocks


def _parse_step_run(step_text: str) -> str | None:
    lines = step_text.splitlines()
    if not lines:
        return None
    item_indent = len(lines[0]) - len(lines[0].lstrip(" "))
    field_indent = item_indent + 2
    multiline_indicators = {"|", "|-", "|+", ">", ">-", ">+"}

    candidates: list[tuple[int, str]] = []
    first = lines[0][item_indent + 2 :]
    if first.startswith("run:"):
        candidates.append((0, first[len("run:") :]))
    prefix = " " * field_indent + "run:"
    for index, line in enumerate(lines[1:], start=1):
        if line.startswith(prefix):
            candidates.append((index, line[len(prefix) :]))

    if len(candidates) != 1:
        return None
    index, raw_value = candidates[0]
    value = _strip_yaml_comment(raw_value).strip()
    if value in multiline_indicators:
        body: list[str] = []
        for line in lines[index + 1 :]:
            if not line.strip():
                body.append("")
                continue
            indent = len(line) - len(line.lstrip(" "))
            if indent <= field_indent:
                break
            body.append(line)
        return textwrap.dedent("\n".join(body))
    if not value:
        return None
    return value


def _shell_logical_lines(script: str) -> list[str]:
    logical: list[str] = []
    pending = ""
    for raw in script.splitlines():
        code = _strip_yaml_comment(raw).strip()
        if not code:
            continue
        if pending:
            code = pending + " " + code
            pending = ""
        if code.endswith("\\"):
            pending = code[:-1].rstrip()
            continue
        logical.append(code)
    if pending:
        logical.append(pending)
    return logical


def _line_invokes_phpunit(line: str, test_path: str) -> bool:
    try:
        tokens = shlex.split(line, posix=True)
    except ValueError:
        return False
    if not tokens:
        return False

    index = 0
    if tokens[0] == "env":
        index = 1
        while index < len(tokens) and tokens[index].startswith("-"):
            index += 1
    assignment = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=.*$")
    while index < len(tokens) and assignment.match(tokens[index]):
        index += 1
    if index >= len(tokens) or tokens[index] != "vendor/bin/phpunit":
        return False
    return test_path in tokens[index + 1 :]


def _proving_step_indexes(job_text: str, test_path: str) -> list[int]:
    indexes: list[int] = []
    for index, step in enumerate(_direct_sequence_item_blocks(job_text, "steps")):
        script = _parse_step_run(step)
        if script is None:
            continue
        if any(_line_invokes_phpunit(line, test_path) for line in _shell_logical_lines(script)):
            indexes.append(index)
    return indexes


def _script_exports_to_github_env(script: str, name: str) -> bool:
    assignment = re.escape(name) + r"="
    github_env = r'["\']?\$\{?GITHUB_ENV\}?["\']?'
    redirect = re.compile(r">>\s*" + github_env)
    emitter = re.compile(r"^(?:echo|printf)\b")
    for line in _shell_logical_lines(script):
        if not emitter.search(line):
            continue
        if re.search(assignment, line) is None:
            continue
        if redirect.search(line) is not None:
            return True
    return False


def _environment_available_by_step(job_text: str, name: str, proving_index: int) -> bool:
    job_env = _direct_child_mapping_block(job_text, "env")
    if _mapping_has_direct_key(job_env, name):
        return True

    steps = _direct_sequence_item_blocks(job_text, "steps")
    if proving_index >= len(steps):
        return False

    proving_step_env = _direct_child_mapping_block(steps[proving_index], "env")
    if _mapping_has_direct_key(proving_step_env, name):
        return True

    # GITHUB_ENV exports become available only to later steps. Step-level env
    # is scoped to its own step and cannot satisfy a later proving step.
    for step in steps[:proving_index]:
        script = _parse_step_run(step)
        if script is not None and _script_exports_to_github_env(script, name):
            return True
    return False


def validate_repository(root: Path) -> list[str]:
    root = root.resolve()
    registry = _load_json(root / REGISTRY_PATH)
    if not isinstance(registry, dict):
        raise RegistrationError("integration-test registry root must be a JSON object")
    if registry.get("schema_version") != 2:
        raise RegistrationError("integration-test registry schema_version must equal 2")

    records = registry.get("tests")
    if not isinstance(records, list):
        raise RegistrationError("integration-test registry tests must be a JSON array")

    discovered = discover_integration_tests(root)
    registered: dict[str, dict[str, object]] = {}
    errors: list[str] = []

    for index, raw_record in enumerate(records):
        if not isinstance(raw_record, dict):
            errors.append(f"registry entry {index}: expected a JSON object")
            continue
        try:
            test_path = _non_empty_string(raw_record.get("path"), "path", f"entry {index}")
        except RegistrationError as exc:
            errors.append(str(exc))
            continue
        if test_path in registered:
            errors.append(f"duplicate integration-test registration: {test_path}")
            continue
        registered[test_path] = raw_record

    discovered_set = set(discovered)
    registered_set = set(registered)
    for path in sorted(discovered_set - registered_set):
        errors.append(f"unregistered Integration test: {path}")
    for path in sorted(registered_set - discovered_set):
        errors.append(f"stale Integration registration references missing test: {path}")

    for test_path in sorted(discovered_set & registered_set):
        record = registered[test_path]
        try:
            workflow = _non_empty_string(record.get("workflow"), "workflow", test_path)
            event = _non_empty_string(record.get("event"), "event", test_path)
            job = _non_empty_string(record.get("job"), "job", test_path)
            invocation = _non_empty_string(record.get("invocation_marker"), "invocation_marker", test_path)
            trigger = _non_empty_string(record.get("trigger_marker"), "trigger_marker", test_path)
            condition = _non_empty_string(record.get("job_condition_marker"), "job_condition_marker", test_path)
            if invocation != test_path:
                raise RegistrationError(
                    f"{test_path}: invocation_marker must be the exact test path so directory-only execution cannot satisfy registration"
                )
            workflow_relative = Path(workflow)
            if workflow_relative.parent != WORKFLOW_ROOT or workflow_relative.suffix not in {".yml", ".yaml"}:
                raise RegistrationError(
                    f"{test_path}: workflow must be a direct YAML file under {WORKFLOW_ROOT.as_posix()}/"
                )
            workflow_path = _relative_file(root, workflow, "workflow", test_path)
            workflow_text = workflow_path.read_text(encoding="utf-8")

            event_block = _mapping_child_block(workflow_text, "on", event)
            if event_block is None:
                raise RegistrationError(f"{test_path}: workflow {workflow} has no top-level on.{event} trigger")
            if not _event_contains_trigger_marker(event, event_block, trigger):
                raise RegistrationError(
                    f"{test_path}: top-level on.{event} trigger does not contain executable marker {trigger}"
                )

            job_block = _mapping_child_block(workflow_text, "jobs", job)
            if job_block is None:
                raise RegistrationError(f"{test_path}: workflow {workflow} has no jobs.{job} proving job")
            actual_condition = _mapping_direct_scalar(job_block, "if")
            if actual_condition != condition:
                raise RegistrationError(
                    f"{test_path}: proving job {job} does not contain required condition marker {condition} as its direct if condition"
                )

            proving_indexes = _proving_step_indexes(job_block, invocation)
            if not proving_indexes:
                raise RegistrationError(
                    f"{test_path}: proving job {job} does not executably invoke {invocation} through a run step using vendor/bin/phpunit"
                )

            required_environment = record.get("required_environment", [])
            if not isinstance(required_environment, list) or any(
                not isinstance(name, str) or not name.strip() for name in required_environment
            ):
                raise RegistrationError(f"{test_path}: required_environment must be an array of non-empty strings")
            missing_environment = [
                name
                for name in required_environment
                if not any(_environment_available_by_step(job_block, name, index) for index in proving_indexes)
            ]
            if missing_environment:
                raise RegistrationError(
                    f"{test_path}: proving job {job} is missing executable required environment provisioning before the test: "
                    + ", ".join(missing_environment)
                )
        except RegistrationError as exc:
            errors.append(str(exc))

    if errors:
        raise RegistrationError("Integration-test registration validation failed:\n- " + "\n- ".join(errors))

    return discovered


def main() -> int:
    repository_root = Path(__file__).resolve().parents[2]
    try:
        tests = validate_repository(repository_root)
    except RegistrationError as exc:
        print(str(exc))
        return 1
    print(f"Validated {len(tests)} explicitly registered Integration test(s).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
