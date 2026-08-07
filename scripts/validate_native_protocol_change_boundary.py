#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
import sys
from collections.abc import Iterable
from pathlib import Path

PRODUCER_TASK = "docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-producer.md"

RUNTIME_ROOTS = (
    "app/",
    "services/",
    "database/",
    "config/",
    "routes/",
    "tests/",
)

ALLOWED_RUNTIME_PREFIXES = (
    "app/GameAuth/Worlds/",
    "services/game-gateway/",
    "tests/Feature/GameAuth/",
)

ALLOWED_RUNTIME_EXACT = frozenset(
    {
        "app/Http/Controllers/GameAuth/GameLoginContextController.php",
        "database/migrations/2026_08_05_130000_migrate_native_protocol_identity_to_version.php",
    }
)

NATIVE_PRODUCER_SIGNAL_EXACT = frozenset(
    {
        "docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md",
        "docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_MIGRATION.md",
        "docs/contracts/oteryn_native_gameplay_v1.proto",
        "docs/architecture/adr/0011-single-native-protocol-version.md",
        "docs/architecture/OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md",
        "docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md",
        "docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md",
        "docs/agents/prompts/OTS_NATIVE_PROTOCOL_SINGLE_VERSION_COMPLETION_AGENT.md",
        "scripts/validate_native_protocol_contract.py",
    }
)

NATIVE_PRODUCER_SIGNAL_PREFIXES = (
    "docs/contracts/fixtures/oteryn-native-v1/",
)


class BoundaryViolation(ValueError):
    pass


def _is_runtime(path: str) -> bool:
    return path.startswith(RUNTIME_ROOTS)


def _is_allowed_runtime(path: str) -> bool:
    return path in ALLOWED_RUNTIME_EXACT or path.startswith(ALLOWED_RUNTIME_PREFIXES)


def is_native_producer_signal(path: str) -> bool:
    return (
        path == PRODUCER_TASK
        or path in NATIVE_PRODUCER_SIGNAL_EXACT
        or path.startswith(NATIVE_PRODUCER_SIGNAL_PREFIXES)
    )


def evaluate_changed_paths(
    changed_paths: Iterable[str],
    *,
    producer_task_exists: bool,
) -> str:
    changed = [path.strip() for path in changed_paths if path.strip()]

    if not any(is_native_producer_signal(path) for path in changed):
        return "NOT_APPLICABLE"

    runtime = [path for path in changed if _is_runtime(path)]
    if not runtime:
        return "DOCUMENTATION_ONLY"

    if PRODUCER_TASK not in changed or not producer_task_exists:
        raise BoundaryViolation(
            "runtime producer change is missing its active governed task record"
        )

    invalid = [path for path in runtime if not _is_allowed_runtime(path)]
    if invalid:
        raise BoundaryViolation(
            f"producer correction escaped its governed runtime boundary: {invalid}"
        )

    return "GOVERNED_RUNTIME"


def changed_paths_from_git(base_ref: str) -> list[str]:
    merge_base = subprocess.check_output(
        ["git", "merge-base", "HEAD", base_ref],
        text=True,
    ).strip()
    return subprocess.check_output(
        ["git", "diff", "--name-only", f"{merge_base}...HEAD"],
        text=True,
    ).splitlines()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate routing for the native-protocol governed runtime boundary."
    )
    parser.add_argument("--base-ref", default="origin/main")
    args = parser.parse_args(argv)

    changed = changed_paths_from_git(args.base_ref)

    try:
        result = evaluate_changed_paths(
            changed,
            producer_task_exists=Path(PRODUCER_TASK).is_file(),
        )
    except BoundaryViolation as exc:
        print(f"native protocol producer boundary audit: FAIL: {exc}", file=sys.stderr)
        return 1

    if result == "NOT_APPLICABLE":
        print(
            "native protocol producer boundary audit: NOT_APPLICABLE "
            "(no native-protocol producer signal in changed paths)"
        )
    elif result == "DOCUMENTATION_ONLY":
        print("native protocol producer boundary audit: PASS (producer documentation only)")
    else:
        print("native protocol producer boundary audit: PASS (governed runtime change)")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
