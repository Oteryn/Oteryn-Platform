#!/usr/bin/env python3
"""Apply the bounded Issue #365 validator repair without rewriting the frozen source."""

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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("target", type=Path)
    args = parser.parse_args()

    original = load_original()
    generated = original.prepare(args.source.read_text(encoding="utf-8"))

    broken = """    '05-media-snapshot.sh': "cat > "$RUN_ROOT/media-snapshot.php" <<'PHP'",
"""
    repaired = r"""    '05-media-snapshot.sh': "cat > \"$RUN_ROOT/media-snapshot.php\" <<'PHP'",
"""
    count = generated.count(broken)
    if count != 1:
        raise SystemExit(
            "media snapshot selector repair: expected one match, "
            f"found {count}"
        )

    args.target.write_text(generated.replace(broken, repaired, 1), encoding="utf-8")
    args.target.chmod(0o700)


if __name__ == "__main__":
    main()
