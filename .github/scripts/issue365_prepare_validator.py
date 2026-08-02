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


def replace_one_variant(
    text: str,
    variants: tuple[tuple[str, str], ...],
    label: str,
) -> str:
    matches = [(old, new) for old, new in variants if text.count(old) == 1]
    if len(matches) != 1:
        counts = {old: text.count(old) for old, _ in variants}
        raise SystemExit(f"{label}: expected one unique variant, found {counts}")
    old, new = matches[0]
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

    hash_anchor = (
        "docker exec \"$app_container\" bash -lc "
        "'sha256sum \"$RUN_ROOT\"/runtime/*.sh > "
        "\"$RUN_ROOT/runtime/SHA256SUMS\"'"
    )
    runtime_patch = r"""docker exec -i "$app_container" python3 - <<'PY'
from pathlib import Path

path = Path('/evidence/issue365-run/runtime/02-observer-patch.sh')
text = path.read_text(encoding='utf-8')
old = '    "        $this->saveSession($request);\\n        return $response;\\n",\n'
new = '    "        $this->saveSession($request);\\n\\n        return $response;\\n",\n'
count = text.count(old)
if count != 1:
    raise SystemExit(
        'Laravel 13 runtime observer pattern: '
        f'expected one match, found {count}'
    )
path.write_text(text.replace(old, new, 1), encoding='utf-8')
PY

docker exec "$app_container" bash -lc 'sha256sum "$RUN_ROOT"/runtime/*.sh > "$RUN_ROOT/runtime/SHA256SUMS"'""".rstrip()
    generated = replace_once(
        generated,
        hash_anchor,
        runtime_patch,
        "generated Laravel runtime observer repair",
    )

    verification_anchor = """docker exec "$app_container" bash -lc '
  set -euo pipefail
  test -f app/Support/Issue365Trace.php"""
    verification_replacement = """docker exec "$app_container" bash -lc '
  set -euo pipefail
  START_SESSION=vendor/laravel/framework/src/Illuminate/Session/Middleware/StartSession.php
  test -f app/Support/Issue365Trace.php"""
    generated = replace_once(
        generated,
        verification_anchor,
        verification_replacement,
        "post-install StartSession scope repair",
    )

    generated = replace_once(
        generated,
        "npx playwright test \\",
        "apt-get update >/dev/null && apt-get install -y --no-install-recommends php-cli php-mysql php-mbstring php-xml php-curl php-redis >/dev/null\n      command -v php\n      php -v\n      npx playwright test \\",
        "Playwright PHP runtime repair",
    )

    generated = replace_one_variant(
        generated,
        (
            (
                "row.get('storage_exists') or not row.get('thumbnail_exists')",
                "not row.get('storage_exists') or row.get('thumbnail_exists')",
            ),
            (
                'row.get("storage_exists") or not row.get("thumbnail_exists")',
                'not row.get("storage_exists") or row.get("thumbnail_exists")',
            ),
            (
                "row['storage_exists'] or not row['thumbnail_exists']",
                "not row['storage_exists'] or row['thumbnail_exists']",
            ),
            (
                'row["storage_exists"] or not row["thumbnail_exists"]',
                'not row["storage_exists"] or row["thumbnail_exists"]',
            ),
        ),
        "one-corrupt storage invariant repair",
    )

    args.target.write_text(generated, encoding="utf-8")
    args.target.chmod(0o700)


if __name__ == "__main__":
    main()
