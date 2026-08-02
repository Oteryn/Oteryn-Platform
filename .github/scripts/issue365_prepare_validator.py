#!/usr/bin/env python3
"""Inspect the generated Issue #365 validator without starting the matrix."""

from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path
from types import ModuleType


def load_original() -> ModuleType:
    original_path = Path(__file__).with_name(
        "issue365_prepare_validator_original.py"
    )
    spec = importlib.util.spec_from_file_location(
        "issue365_prepare_validator_original",
        original_path,
    )
    if spec is None or spec.loader is None:
        raise SystemExit(f"cannot load original validator: {original_path}")

    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("target", type=Path)
    args = parser.parse_args()

    original = load_original()
    generated = original.prepare(args.source.read_text(encoding="utf-8"))

    media_broken = """    '05-media-snapshot.sh': "cat > "$RUN_ROOT/media-snapshot.php" <<'PHP'",
"""
    media_repaired = r"""    '05-media-snapshot.sh': "cat > \"$RUN_ROOT/media-snapshot.php\" <<'PHP'",
"""
    generated = replace_once(
        generated,
        media_broken,
        media_repaired,
        "media snapshot selector repair",
    )

    lines = generated.splitlines()
    matches = [
        index
        for index, line in enumerate(lines)
        if "saveSession" in line or "return $response" in line
    ]
    print("ISSUE365_GENERATED_STARTSESSION_MATCH_COUNT=" + str(len(matches)))
    for match in matches:
        start = max(0, match - 4)
        end = min(len(lines), match + 5)
        print(f"ISSUE365_GENERATED_CONTEXT_BEGIN={start + 1}:{end}")
        for index in range(start, end):
            print(f"{index + 1:05d}: {lines[index]}")
        print("ISSUE365_GENERATED_CONTEXT_END")

    args.target.write_text(generated, encoding="utf-8")
    args.target.chmod(0o700)
    raise SystemExit("diagnostic-only gate complete; matrix intentionally not started")


if __name__ == "__main__":
    main()
