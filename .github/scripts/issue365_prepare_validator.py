#!/usr/bin/env python3
"""Apply bounded Issue #365 repairs to the generated validator."""

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

    # Preserve the literal selector inside the generated observer extractor.
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

    # Laravel 13.20.0 has a blank line between saveSession() and return.
    # The old pattern lives inside the generated runtime observer script, not
    # in the parent validator. Patch that extracted script before it is hashed
    # and executed, while keeping the frozen checkout and runbook untouched.
    hash_anchor = (
        'docker exec "$app_container" bash -lc '\
        "'sha256sum \"$RUN_ROOT\"/runtime/*.sh > "
        "\"$RUN_ROOT/runtime/SHA256SUMS\"'"
    )
    runtime_patch = r'''docker exec -i "$app_container" python3 - <<'PY'
from pathlib import Path

path = Path('/evidence/issue365-run/runtime/02-observer-patch.sh')
text = path.read_text(encoding='utf-8')
old = r'''    "        $this->saveSession($request);\n        return $response;\n",
'''
new = r'''    "        $this->saveSession($request);\n\n        return $response;\n",
'''
count = text.count(old)
if count != 1:
    raise SystemExit(
        'Laravel 13 runtime observer pattern: '
        f'expected one match, found {count}'
    )
path.write_text(text.replace(old, new, 1), encoding='utf-8')
PY

docker exec "$app_container" bash -lc 'sha256sum "$RUN_ROOT"/runtime/*.sh > "$RUN_ROOT/runtime/SHA256SUMS"' '''.rstrip()
    generated = replace_once(
        generated,
        hash_anchor,
        runtime_patch,
        "generated Laravel runtime observer repair",
    )

    args.target.write_text(generated, encoding="utf-8")
    args.target.chmod(0o700)


if __name__ == "__main__":
    main()
