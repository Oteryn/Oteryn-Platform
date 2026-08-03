#!/usr/bin/env python3
"""Apply final fail-closed state, locale, accessibility and overflow rules."""

from __future__ import annotations

import argparse
import importlib.util
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot import {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


AUDIT = load_module("portal_exhaustive_audit", ROOT / "portal_exhaustive_audit.py")
RECONCILE = load_module("portal_exhaustive_reconcile", ROOT / "portal_exhaustive_reconcile.py")

REQUIRED_STATE_CATEGORIES = {
    "not_found",
    "csrf_expiry",
    "rate_limit",
    "server_failure",
    "recovery",
}
REQUIRED_LOCALES = {"en", "pl"}
GENERATED_PREFIXES = (
    "OTERYN-AUDIT-CURRENT-STATE-",
    "OTERYN-AUDIT-CURRENT-LOCALE-",
    "OTERYN-AUDIT-CURRENT-ACCESSIBILITY-",
    "OTERYN-AUDIT-CURRENT-OVERFLOW-",
)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def declared_locales(states: list[str]) -> set[str]:
    values = [str(value).casefold().replace("_", "-") for value in states]
    locales: set[str] = set()
    for value in values:
        tokens = set(value.split("-"))
        if "en" in tokens or "english" in value:
            locales.add("en")
        if "pl" in tokens or "polish" in value or "polski" in value:
            locales.add("pl")
    return locales


def strict_state_categories(states: list[str]) -> set[str]:
    normalized = [str(value).casefold().replace("_", "-") for value in states]
    categories: set[str] = set()
    patterns = {
        "not_found": ("not-found", "404", "missing-resource", "missing-record", "missing-page"),
        "csrf_expiry": ("419", "csrf", "page-expired", "session-expired", "expired-session"),
        "rate_limit": ("429", "rate-limit", "rate-limited", "throttle", "too-many-requests"),
        "server_failure": ("500", "503", "server-failure", "internal-error", "service-unavailable", "dependency-unavailable"),
        "recovery": ("dependency-restored", "service-restored", "recovery-success", "retry-success", "recovered"),
    }
    for category, markers in patterns.items():
        if any(marker in state for state in normalized for marker in markers):
            categories.add(category)
    return categories


def evidence_text(records: list[dict[str, Any]]) -> str:
    chunks: list[str] = []
    for record in records:
        chunks.extend(str(value) for value in record.get("evidence_layers", []))
        for evidence in record.get("evidence", []):
            if not isinstance(evidence, dict):
                continue
            chunks.append(str(evidence.get("file", "")))
            chunks.extend(str(value) for value in evidence.get("markers", []))
    return " ".join(chunks).casefold()


def has_accessibility_evidence(records: list[dict[str, Any]]) -> bool:
    text = evidence_text(records)
    return "accessibility" in text or "axe" in text or "wcag" in text


def has_overflow_evidence(records: list[dict[str, Any]]) -> bool:
    text = evidence_text(records)
    states = " ".join(
        str(value).casefold()
        for record in records
        for value in record.get("states", [])
    )
    return "overflow" in text or "horizontal-scroll" in text or "overflow" in states


def is_generated_strictness_finding(finding: dict[str, Any]) -> bool:
    identifier = str(finding.get("id", ""))
    return identifier.startswith(GENERATED_PREFIXES)


def strict_surface_findings(records: list[dict[str, Any]]) -> tuple[list[dict[str, Any]], dict[str, set[str]]]:
    rendered_by_surface: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        if record.get("kind") == "rendered_screen" and record.get("surface_id"):
            rendered_by_surface[str(record["surface_id"])].append(record)

    findings: list[dict[str, Any]] = []
    finding_ids_by_surface: dict[str, set[str]] = defaultdict(set)
    for surface_id, surface_records in sorted(rendered_by_surface.items()):
        representative = surface_records[0]
        module = str(representative.get("module", "quality_e2e"))
        states = sorted({str(state) for row in surface_records for state in row.get("states", [])})
        categories = strict_state_categories(states)
        missing_state_categories = sorted(REQUIRED_STATE_CATEGORIES - categories)
        if missing_state_categories:
            identifier = AUDIT.finding_id("STATE", surface_id)
            finding_ids_by_surface[surface_id].add(identifier)
            findings.append(
                {
                    "id": identifier,
                    "severity": "MEDIUM",
                    "module": module,
                    "subject": surface_id,
                    "title": f"{surface_id} lacks explicit HTTP failure and recovery closure",
                    "evidence": {
                        "declared_states": states,
                        "state_categories": sorted(categories),
                        "missing_categories": missing_state_categories,
                    },
                    "impact": "Issue #326 requires explicit applicability or evidence for 404, 419, 429, server/dependency failure and recovery; validation errors and generic missing states cannot substitute for HTTP evidence.",
                    "disposition": "Declare and execute each missing category, or persist an owner-approved non-applicability rule with exact evidence.",
                }
            )

        locales = declared_locales(states)
        missing_locales = sorted(REQUIRED_LOCALES - locales)
        if missing_locales:
            identifier = AUDIT.finding_id("LOCALE", surface_id)
            finding_ids_by_surface[surface_id].add(identifier)
            findings.append(
                {
                    "id": identifier,
                    "severity": "MEDIUM",
                    "module": module,
                    "subject": surface_id,
                    "title": f"{surface_id} lacks explicit EN/PL parity evidence",
                    "evidence": {
                        "declared_states": states,
                        "declared_locales": sorted(locales),
                        "missing_locales": missing_locales,
                    },
                    "impact": "Locale completeness cannot be inferred from a generic localization marker or from another surface.",
                    "disposition": "Declare and execute exact-head English and Polish evidence, or persist an owner-approved locale non-applicability rule.",
                }
            )

        if not has_accessibility_evidence(surface_records):
            identifier = AUDIT.finding_id("ACCESSIBILITY", surface_id)
            finding_ids_by_surface[surface_id].add(identifier)
            findings.append(
                {
                    "id": identifier,
                    "severity": "MEDIUM",
                    "module": module,
                    "subject": surface_id,
                    "title": f"{surface_id} lacks explicit accessibility evidence",
                    "evidence": {"evidence_text": evidence_text(surface_records)[:1000]},
                    "impact": "Responsive or functional evidence does not establish accessibility or keyboard/semantic closure.",
                    "disposition": "Add bounded exact-head accessibility evidence or persist an owner-approved non-applicability rule.",
                }
            )

        if not has_overflow_evidence(surface_records):
            identifier = AUDIT.finding_id("OVERFLOW", surface_id)
            finding_ids_by_surface[surface_id].add(identifier)
            findings.append(
                {
                    "id": identifier,
                    "severity": "MEDIUM",
                    "module": module,
                    "subject": surface_id,
                    "title": f"{surface_id} lacks explicit horizontal-overflow evidence",
                    "evidence": {"evidence_text": evidence_text(surface_records)[:1000], "declared_states": states},
                    "impact": "Viewport declarations do not prove absence of clipped controls or horizontal scrolling.",
                    "disposition": "Add exact-head overflow assertions at applicable viewport and long-content boundaries.",
                }
            )
    return findings, finding_ids_by_surface


def apply_strictness(matrix: dict[str, Any]) -> dict[str, Any]:
    route_records = [row for row in matrix.get("route_records", []) if isinstance(row, dict)]
    retained_findings = [
        row for row in matrix.get("findings", [])
        if isinstance(row, dict) and not is_generated_strictness_finding(row)
    ]
    strict_findings, ids_by_surface = strict_surface_findings(route_records)

    for record in route_records:
        identifiers = {
            str(value) for value in record.get("finding_ids", [])
            if not str(value).startswith(GENERATED_PREFIXES)
        }
        identifiers.update(ids_by_surface.get(str(record.get("surface_id", "")), set()))
        record["finding_ids"] = sorted(identifiers)
        record["locales"] = sorted(declared_locales([str(value) for value in record.get("states", [])]))

    matrix["route_records"] = route_records
    matrix["findings"] = AUDIT.deduplicate_findings([*retained_findings, *strict_findings])
    matrix["counts"]["findings"] = len(matrix["findings"])
    matrix["module_summaries"] = RECONCILE.rebuild_module_summaries(matrix, matrix["findings"])
    return matrix


def validate_strictness(matrix: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    finding_ids = {str(row.get("id")) for row in matrix.get("findings", []) if isinstance(row, dict)}
    rendered_by_surface: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in matrix.get("route_records", []):
        if isinstance(record, dict) and record.get("kind") == "rendered_screen" and record.get("surface_id"):
            rendered_by_surface[str(record["surface_id"])].append(record)

    for surface_id, records in rendered_by_surface.items():
        states = sorted({str(state) for row in records for state in row.get("states", [])})
        categories = strict_state_categories(states)
        locales = declared_locales(states)
        required = {
            AUDIT.finding_id("STATE", surface_id): bool(REQUIRED_STATE_CATEGORIES - categories),
            AUDIT.finding_id("LOCALE", surface_id): bool(REQUIRED_LOCALES - locales),
            AUDIT.finding_id("ACCESSIBILITY", surface_id): not has_accessibility_evidence(records),
            AUDIT.finding_id("OVERFLOW", surface_id): not has_overflow_evidence(records),
        }
        for identifier, needed in required.items():
            if needed and identifier not in finding_ids:
                errors.append(f"{surface_id}: missing required finding {identifier}")
    return errors


def update_summary(output: Path, matrix: dict[str, Any]) -> None:
    severity_counts = Counter(str(row.get("severity", "UNKNOWN")) for row in matrix["findings"])
    payload = {
        "exact_sha": matrix["exact_sha"],
        "finding_count": len(matrix["findings"]),
        "severity_counts": dict(sorted(severity_counts.items())),
        "state_rule": "precise 404 419 429 server failure and recovery markers require explicit coverage or a finding",
        "locale_rule": "both en and pl require explicit coverage or a finding",
        "accessibility_rule": "accessibility requires an explicit evidence marker or a finding",
        "overflow_rule": "horizontal overflow requires an explicit assertion or a finding",
    }
    write_json(output / "portal-exhaustive-strictness.json", payload)
    (output / "portal-exhaustive-audit-summary.md").write_text(
        AUDIT.markdown_summary(matrix), encoding="utf-8"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    matrix_path = output / "portal-exhaustive-audit-matrix.json"
    matrix = apply_strictness(read_json(matrix_path))
    errors = validate_strictness(matrix)
    if errors:
        raise RuntimeError("Strictness validation failed: " + "; ".join(errors))
    write_json(matrix_path, matrix)
    write_json(output / "portal-exhaustive-audit-findings.json", matrix["findings"])
    RECONCILE.write_split_evidence(output, matrix)
    update_summary(output, matrix)
    print(json.dumps({"exact_sha": matrix["exact_sha"], "finding_count": len(matrix["findings"])}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
