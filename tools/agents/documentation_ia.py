#!/usr/bin/env python3
"""Validate the deterministic Oteryn Platform Documentation/Agent IA catalog."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

DEFAULT_CATALOG = Path("docs/agents/DOCUMENTATION_IA_CATALOG.json")
REQUIRED_PROMPT_FIELDS = {
    "id", "path", "version", "classification", "status", "owner", "scope",
    "executable", "supersession", "provenance",
}
REQUIRED_HANDOVER_FIELDS = {
    "id", "path", "status", "authoritative", "lifecycle", "superseded_by", "provenance",
}
PROMPT_CLASSIFICATIONS = {"reusable", "one_shot_historical"}
PROMPT_STATUSES = {
    "reusable": "active_reusable",
    "one_shot_historical": "historical_do_not_run",
}
HANDOVER_STATUSES = {"historical_snapshot", "frozen_evidence_handoff"}
HANDOVER_LIFECYCLES = {"expired", "supersede_on_live_transition"}


class DocumentationIAError(RuntimeError):
    pass


def _nonempty(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _load(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise DocumentationIAError(f"{path}: invalid catalog JSON") from exc
    if not isinstance(value, dict):
        raise DocumentationIAError(f"{path}: catalog must be a JSON object")
    return value


def _markdown_inventory(root: Path, repo_root: Path) -> set[str]:
    if not root.exists():
        raise DocumentationIAError(f"inventory root does not exist: {root}")
    return {
        path.relative_to(repo_root).as_posix()
        for path in sorted(root.glob("*.md"))
        if path.name.casefold() != "readme.md"
    }


def _unique(entries: list[dict[str, Any]], field: str, label: str, errors: list[str]) -> None:
    seen: set[str] = set()
    for entry in entries:
        value = entry.get(field)
        if not _nonempty(value):
            errors.append(f"{label}: missing non-empty {field}")
            continue
        if value in seen:
            errors.append(f"{label}: duplicate {field} {value!r}")
        seen.add(value)


def validate_catalog(repo_root: Path, catalog_path: Path) -> list[str]:
    catalog = _load(catalog_path)
    errors: list[str] = []

    if catalog.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    if not _nonempty(catalog.get("catalog_id")):
        errors.append("catalog_id must be non-empty")
    head = catalog.get("inventory_head")
    if not isinstance(head, str) or len(head) != 40 or any(ch not in "0123456789abcdef" for ch in head.casefold()):
        errors.append("inventory_head must be a 40-character Git commit SHA")

    scope = catalog.get("inventory_scope")
    if not isinstance(scope, dict):
        errors.append("inventory_scope must be an object")
        return errors
    prompt_root_rel = scope.get("prompt_root")
    handover_root_rel = scope.get("handover_root")
    if not _nonempty(prompt_root_rel) or not _nonempty(handover_root_rel):
        errors.append("inventory_scope prompt_root/handover_root must be non-empty")
        return errors

    prompts = catalog.get("prompts")
    handovers = catalog.get("handovers")
    if not isinstance(prompts, list) or not all(isinstance(item, dict) for item in prompts):
        errors.append("prompts must be an object list")
        prompts = []
    if not isinstance(handovers, list) or not all(isinstance(item, dict) for item in handovers):
        errors.append("handovers must be an object list")
        handovers = []

    _unique(prompts, "id", "prompt", errors)
    _unique(prompts, "path", "prompt", errors)
    _unique(handovers, "id", "handover", errors)
    _unique(handovers, "path", "handover", errors)

    prompt_root = repo_root / prompt_root_rel
    handover_root = repo_root / handover_root_rel
    actual_prompts = _markdown_inventory(prompt_root, repo_root)
    actual_handovers = _markdown_inventory(handover_root, repo_root)
    catalog_prompts = {entry.get("path") for entry in prompts if _nonempty(entry.get("path"))}
    catalog_handovers = {entry.get("path") for entry in handovers if _nonempty(entry.get("path"))}

    if catalog_prompts != actual_prompts:
        missing = sorted(actual_prompts - catalog_prompts)
        stale = sorted(catalog_prompts - actual_prompts)
        errors.append(f"prompt inventory mismatch: uncatalogued={missing}, missing_files={stale}")
    if catalog_handovers != actual_handovers:
        missing = sorted(actual_handovers - catalog_handovers)
        stale = sorted(catalog_handovers - actual_handovers)
        errors.append(f"handover inventory mismatch: uncatalogued={missing}, missing_files={stale}")

    for entry in prompts:
        label = f"prompt {entry.get('path', '<unknown>')}"
        if set(entry) != REQUIRED_PROMPT_FIELDS:
            errors.append(f"{label}: fields do not match catalog contract")
            continue
        for field in ("id", "path", "version", "owner", "scope"):
            if not _nonempty(entry[field]):
                errors.append(f"{label}: {field} must be non-empty")
        classification = entry["classification"]
        if classification not in PROMPT_CLASSIFICATIONS:
            errors.append(f"{label}: invalid classification {classification!r}")
            continue
        if entry["status"] != PROMPT_STATUSES[classification]:
            errors.append(f"{label}: status does not match classification")
        if not isinstance(entry["executable"], bool):
            errors.append(f"{label}: executable must be boolean")
        elif entry["executable"] != (classification == "reusable"):
            errors.append(f"{label}: executable contradicts classification")
        supersession = entry["supersession"]
        if not isinstance(supersession, dict) or set(supersession) != {"target", "reason"}:
            errors.append(f"{label}: supersession must contain target and reason")
        else:
            target = supersession["target"]
            if target is not None and not _nonempty(target):
                errors.append(f"{label}: supersession.target must be null or non-empty")
            if not _nonempty(supersession["reason"]):
                errors.append(f"{label}: supersession.reason must be non-empty")
            if classification == "reusable" and target is not None:
                errors.append(f"{label}: reusable prompt cannot have a supersession target")
        provenance = entry["provenance"]
        if not isinstance(provenance, list) or not provenance or not all(_nonempty(item) for item in provenance):
            errors.append(f"{label}: provenance must be a non-empty string list")

    for entry in handovers:
        label = f"handover {entry.get('path', '<unknown>')}"
        if set(entry) != REQUIRED_HANDOVER_FIELDS:
            errors.append(f"{label}: fields do not match catalog contract")
            continue
        for field in ("id", "path", "status", "lifecycle", "superseded_by"):
            if not _nonempty(entry[field]):
                errors.append(f"{label}: {field} must be non-empty")
        if entry["status"] not in HANDOVER_STATUSES:
            errors.append(f"{label}: invalid status {entry['status']!r}")
        if entry["authoritative"] is not False:
            errors.append(f"{label}: retained handovers must be authoritative=false")
        if entry["lifecycle"] not in HANDOVER_LIFECYCLES:
            errors.append(f"{label}: invalid lifecycle {entry['lifecycle']!r}")
        provenance = entry["provenance"]
        if not isinstance(provenance, list) or not provenance or not all(_nonempty(item) for item in provenance):
            errors.append(f"{label}: provenance must be a non-empty string list")

    chain = scope.get("instruction_chain")
    if not isinstance(chain, dict) or set(chain) != {"effective_paths", "measured_absent_overrides"}:
        errors.append("instruction_chain must contain effective_paths and measured_absent_overrides")
    else:
        effective = chain["effective_paths"]
        absent = chain["measured_absent_overrides"]
        if not isinstance(effective, list) or not effective or not all(_nonempty(item) for item in effective):
            errors.append("instruction_chain.effective_paths must be a non-empty string list")
        else:
            for relative in effective:
                if not (repo_root / relative).is_file():
                    errors.append(f"instruction_chain effective path missing: {relative}")
        if not isinstance(absent, list) or not all(_nonempty(item) for item in absent):
            errors.append("instruction_chain.measured_absent_overrides must be a string list")
        else:
            for relative in absent:
                if (repo_root / relative).exists():
                    errors.append(f"instruction_chain absent override now exists: {relative}")

    authority = catalog.get("lifecycle_authority")
    if not _nonempty(authority) or not (repo_root / authority).is_file():
        errors.append("lifecycle_authority must reference an existing file")

    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument("--catalog", type=Path, default=DEFAULT_CATALOG)
    args = parser.parse_args(argv)
    try:
        errors = validate_catalog(args.repo_root, args.catalog)
    except DocumentationIAError as exc:
        print(f"documentation-ia error: {exc}", file=sys.stderr)
        return 1
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    catalog = _load(args.catalog)
    reusable = sum(item["classification"] == "reusable" for item in catalog["prompts"])
    historical = len(catalog["prompts"]) - reusable
    print(
        f"Validated Documentation/Agent IA: {len(catalog['prompts'])} prompts "
        f"({reusable} reusable, {historical} historical) and "
        f"{len(catalog['handovers'])} handovers."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
