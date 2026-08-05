#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from collections import defaultdict
from pathlib import Path
from typing import Iterable, Mapping

ADR_DIRECTORY = Path("docs/architecture/adr")
README_NAME = "README.md"

ADR_FILENAME_PATTERN = re.compile(
    r"^(?P<prefix>\d{4})-(?P<slug>[a-z0-9]+(?:-[a-z0-9]+)*)\.md$"
)
DIRECT_STATUS_PATTERN = re.compile(
    r"^(?:-\s*)?Status:\s*(?P<status>Proposed|Accepted|Superseded|Rejected)\b.*$",
    re.MULTILINE,
)
SECTION_STATUS_PATTERN = re.compile(
    r"^## Status\s*$\n(?:[ \t]*\n)*(?P<status>Proposed|Accepted|Superseded|Rejected)\b[^\n]*$",
    re.MULTILINE,
)
SUPERSEDED_BY_PATTERN = re.compile(
    r"^- Superseded by:\s*`?(?P<target>(?:docs/architecture/adr/)?"
    r"\d{4}-[a-z0-9]+(?:-[a-z0-9]+)*\.md)`?\s*$",
    re.MULTILINE,
)
INVENTORY_ENTRY_PATTERN = re.compile(r"^- `(?P<filename>[^`]+)`\s*$")

LEGACY_DUPLICATE_PATHS: dict[str, tuple[str, ...]] = {
    "0008": (
        "0008-oteryn-frontend-information-and-shell-architecture.md",
        "0008-risk-based-continuous-e2e-validation.md",
    ),
    "0010": (
        "0010-native-gameplay-protocol-selection.md",
        "0010-wiki-module-and-persistence-foundation.md",
    ),
    "0011": (
        "0011-safe-editorial-media-boundary.md",
        "0011-single-native-protocol-version.md",
    ),
    "0015": (
        "0015-machine-enforced-portal-acceptance-ledger.md",
        "0015-wiki-launch-content-provisioning.md",
    ),
    "0016": (
        "0016-character-bazaar-wallet-and-escrow.md",
        "0016-versioned-game-catalog-snapshots.md",
    ),
    "0017": (
        "0017-account-security-lifecycle.md",
        "0017-platform-support-moderation-boundary.md",
    ),
    "0018": (
        "0018-game-catalog-unknown-verified-boundary.md",
        "0018-read-only-community-data-boundary.md",
    ),
    "0021": (
        "0021-provider-neutral-payment-security-core.md",
        "0021-require-canary-owned-character-deletion-lifecycle.md",
    ),
}


def _normalized_allowlist(
    legacy_duplicates: Mapping[str, Iterable[str]],
) -> dict[str, tuple[str, ...]]:
    return {
        str(prefix): tuple(sorted(str(path) for path in paths))
        for prefix, paths in legacy_duplicates.items()
    }


def _lifecycle_declarations(content: str) -> list[str]:
    declarations = [
        match.group("status") for match in DIRECT_STATUS_PATTERN.finditer(content)
    ]
    declarations.extend(
        match.group("status") for match in SECTION_STATUS_PATTERN.finditer(content)
    )
    return declarations


def _inventory_entries(readme: Path) -> tuple[list[str], list[str]]:
    if not readme.is_file():
        return [], [f"missing ADR inventory: {readme.as_posix()}"]

    entries: list[str] = []
    in_inventory = False
    for raw_line in readme.read_text(encoding="utf-8").splitlines():
        line = raw_line.rstrip()
        if line == "## Inventory":
            in_inventory = True
            continue
        if in_inventory and line.startswith("## "):
            break
        if not in_inventory:
            continue
        match = INVENTORY_ENTRY_PATTERN.fullmatch(line)
        if match:
            entries.append(match.group("filename"))

    errors: list[str] = []
    if not in_inventory:
        errors.append(f"missing '## Inventory' section in {readme.as_posix()}")
    if len(entries) != len(set(entries)):
        duplicates = sorted(
            name for name in set(entries) if entries.count(name) > 1
        )
        errors.append(f"duplicate README inventory entries: {', '.join(duplicates)}")
    return entries, errors


def validate_repository(
    root: Path,
    *,
    legacy_duplicates: Mapping[str, Iterable[str]] = LEGACY_DUPLICATE_PATHS,
) -> list[str]:
    root = root.resolve()
    adr_dir = root / ADR_DIRECTORY
    readme = adr_dir / README_NAME
    errors: list[str] = []

    if not adr_dir.is_dir():
        return [f"missing ADR directory: {adr_dir.as_posix()}"]

    adr_files = sorted(
        path for path in adr_dir.iterdir() if path.is_file() and path.name != README_NAME
    )
    actual_names = [path.name for path in adr_files]
    files_by_prefix: dict[str, list[str]] = defaultdict(list)

    for path in adr_files:
        filename_match = ADR_FILENAME_PATTERN.fullmatch(path.name)
        if filename_match is None:
            errors.append(f"invalid ADR filename: {path.name}")
            continue

        prefix = filename_match.group("prefix")
        files_by_prefix[prefix].append(path.name)
        content = path.read_text(encoding="utf-8")
        statuses = _lifecycle_declarations(content)
        if len(statuses) != 1:
            errors.append(
                f"{path.name}: expected exactly one lifecycle declaration, found "
                f"{len(statuses)}"
            )
            continue

        status = statuses[0]
        supersession_matches = list(SUPERSEDED_BY_PATTERN.finditer(content))

        if status == "Superseded" and len(supersession_matches) != 1:
            errors.append(
                f"{path.name}: Superseded ADR must declare exactly one "
                "'- Superseded by:' path"
            )
        elif status != "Superseded" and supersession_matches:
            errors.append(
                f"{path.name}: declares '- Superseded by:' but status is {status}"
            )

        for match in supersession_matches:
            target = Path(match.group("target")).name
            if target not in actual_names:
                errors.append(
                    f"{path.name}: supersession target does not exist: {target}"
                )
            elif target == path.name:
                errors.append(f"{path.name}: cannot supersede itself")

    inventory, inventory_errors = _inventory_entries(readme)
    errors.extend(inventory_errors)
    inventory_set = set(inventory)
    actual_set = set(actual_names)
    if inventory_set != actual_set:
        missing = sorted(actual_set - inventory_set)
        stale = sorted(inventory_set - actual_set)
        if missing:
            errors.append(
                "README inventory is missing ADR files: " + ", ".join(missing)
            )
        if stale:
            errors.append(
                "README inventory contains non-existent ADR files: "
                + ", ".join(stale)
            )

    allowlist = _normalized_allowlist(legacy_duplicates)
    for prefix, names in sorted(files_by_prefix.items()):
        actual = tuple(sorted(names))
        if len(actual) <= 1:
            if prefix in allowlist:
                errors.append(
                    f"legacy duplicate allowlist drift for {prefix}: "
                    f"expected {list(allowlist[prefix])}, found {list(actual)}"
                )
            continue
        expected = allowlist.get(prefix)
        if expected is None:
            errors.append(
                f"unapproved duplicate ADR prefix {prefix}: {', '.join(actual)}"
            )
        elif actual != expected:
            errors.append(
                f"legacy duplicate allowlist drift for {prefix}: "
                f"expected {list(expected)}, found {list(actual)}"
            )

    for prefix, expected in sorted(allowlist.items()):
        actual = tuple(sorted(files_by_prefix.get(prefix, [])))
        if actual != expected and not any(
            error.startswith(f"legacy duplicate allowlist drift for {prefix}:")
            for error in errors
        ):
            errors.append(
                f"legacy duplicate allowlist drift for {prefix}: "
                f"expected {list(expected)}, found {list(actual)}"
            )

    return errors


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate the Oteryn Platform ADR registry."
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="Repository root. Defaults to the root containing this script.",
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    errors = validate_repository(args.root)
    adr_dir = args.root.resolve() / ADR_DIRECTORY
    count = (
        len(
            [
                path
                for path in adr_dir.iterdir()
                if path.is_file() and path.name != README_NAME
            ]
        )
        if adr_dir.is_dir()
        else 0
    )
    result = {
        "adr_count": count,
        "legacy_duplicate_prefixes": sorted(LEGACY_DUPLICATE_PATHS),
        "errors": errors,
        "result": "PASS" if not errors else "FAIL",
    }
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    elif errors:
        print("ADR registry validation failed:")
        for error in errors:
            print(f"- {error}")
    else:
        print(
            "ADR registry validation passed: "
            f"{count} ADRs, {len(LEGACY_DUPLICATE_PATHS)} preserved "
            "legacy duplicate prefixes."
        )
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
