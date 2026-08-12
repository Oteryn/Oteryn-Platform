#!/usr/bin/env python3
"""Fail closed when Integration tests are not bound to an executable proving workflow."""

from __future__ import annotations

import json
import re
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


def _workflow_executes_phpunit_test(workflow_text: str, test_path: str) -> bool:
    executable_text = "\n".join(
        line for line in workflow_text.splitlines() if not line.lstrip().startswith("#")
    )
    pattern = re.compile(
        r"vendor/bin/phpunit[\s\\]+" + re.escape(test_path) + r"(?:[\s\\]|$)"
    )
    return pattern.search(executable_text) is not None


def validate_repository(root: Path) -> list[str]:
    root = root.resolve()
    registry = _load_json(root / REGISTRY_PATH)
    if not isinstance(registry, dict):
        raise RegistrationError("integration-test registry root must be a JSON object")
    if registry.get("schema_version") != 1:
        raise RegistrationError("integration-test registry schema_version must equal 1")

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
            invocation = _non_empty_string(record.get("invocation_marker"), "invocation_marker", test_path)
            trigger = _non_empty_string(record.get("trigger_marker"), "trigger_marker", test_path)
            if invocation != test_path:
                raise RegistrationError(
                    f"{test_path}: invocation_marker must be the exact test path so directory-only execution cannot satisfy registration"
                )
            workflow_relative = Path(workflow)
            if workflow_relative.parent != WORKFLOW_ROOT or workflow_relative.suffix not in {".yml", ".yaml"}:
                raise RegistrationError(f"{test_path}: workflow must be a direct YAML file under {WORKFLOW_ROOT.as_posix()}/")
            workflow_path = _relative_file(root, workflow, "workflow", test_path)
            workflow_text = workflow_path.read_text(encoding="utf-8")
            if not _workflow_executes_phpunit_test(workflow_text, invocation):
                raise RegistrationError(
                    f"{test_path}: workflow {workflow} does not executably invoke {invocation} through vendor/bin/phpunit"
                )
            if trigger not in workflow_text:
                raise RegistrationError(
                    f"{test_path}: workflow {workflow} does not contain trigger marker {trigger}"
                )

            required_environment = record.get("required_environment", [])
            if not isinstance(required_environment, list) or any(
                not isinstance(name, str) or not name.strip() for name in required_environment
            ):
                raise RegistrationError(f"{test_path}: required_environment must be an array of non-empty strings")
            missing_environment = [name for name in required_environment if name not in workflow_text]
            if missing_environment:
                raise RegistrationError(
                    f"{test_path}: workflow {workflow} is missing required environment markers: "
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
