#!/usr/bin/env python3
"""Apply final fail-closed state, locale, accessibility and overflow rules."""

from __future__ import annotations

import argparse
import importlib.util
import json
import subprocess
from collections import Counter, defaultdict
from functools import lru_cache
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parents[1]
STRICTNESS_EVIDENCE_PATH = Path("docs/testing/PORTAL_STRICTNESS_EVIDENCE.json")


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


def load_strictness_evidence(repo_root: Path) -> dict[str, Any]:
    path = repo_root / STRICTNESS_EVIDENCE_PATH
    if not path.exists():
        return {"schema_version": 1, "surfaces": {}}
    value = read_json(path)
    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise RuntimeError(f"{STRICTNESS_EVIDENCE_PATH} must use schema_version 1")
    surfaces = value.get("surfaces")
    if not isinstance(surfaces, dict):
        raise RuntimeError(f"{STRICTNESS_EVIDENCE_PATH} must define a surfaces object")
    return value


def validate_evidence_reference(repo_root: Path, owner: str, evidence: Any) -> None:
    if not isinstance(evidence, dict):
        raise RuntimeError(f"{owner} must define an evidence object")
    relative = evidence.get("file")
    markers = evidence.get("markers")
    if not isinstance(relative, str) or not relative.strip() or not isinstance(markers, list) or not markers:
        raise RuntimeError(f"{owner} must define a repository file and non-empty markers")
    absolute = (repo_root / relative).resolve()
    try:
        absolute.relative_to(repo_root.resolve())
    except ValueError as exc:
        raise RuntimeError(f"{owner} evidence escapes repository root: {relative}") from exc
    if not absolute.is_file():
        raise RuntimeError(f"{owner} references missing evidence file {relative}")
    source = absolute.read_text(encoding="utf-8")
    for marker in markers:
        if not isinstance(marker, str) or not marker.strip():
            raise RuntimeError(f"{owner} contains an empty evidence marker")
        if marker not in source:
            raise RuntimeError(f"{owner} evidence marker is missing from {relative}: {marker}")


@lru_cache(maxsize=4)
def runtime_routes(repo_root_text: str) -> dict[str, dict[str, Any]]:
    repo_root = Path(repo_root_text)
    completed = subprocess.run(
        ["php", "artisan", "route:list", "--json"],
        cwd=repo_root,
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            "Cannot resolve runtime routes for strictness applicability: "
            + (completed.stderr.strip() or completed.stdout.strip())
        )
    value = json.loads(completed.stdout)
    if not isinstance(value, list):
        raise RuntimeError("Laravel route:list --json did not return an array")
    return {
        str(route.get("name")): route
        for route in value
        if isinstance(route, dict) and route.get("name")
    }


def middleware_values(route: dict[str, Any]) -> list[str]:
    value = route.get("middleware", [])
    if isinstance(value, list):
        return [str(item) for item in value]
    if isinstance(value, str):
        return [item.strip() for item in value.split(",") if item.strip()]
    return []


def validate_non_applicability(
    surface_id: str,
    rule: str,
    surface_records: list[dict[str, Any]],
    all_surface_records: list[dict[str, Any]],
    repo_root: Path,
) -> None:
    if not all_surface_records:
        raise RuntimeError(f"{surface_id}: non-applicability cannot be proven without exact route records")

    if rule == "read_only_surface":
        allowed = {"GET", "HEAD", "OPTIONS"}
        methods = {
            str(method).upper()
            for record in all_surface_records
            for method in record.get("methods", [])
        }
        if not methods or not methods.issubset(allowed):
            raise RuntimeError(f"{surface_id}: read_only_surface is false for methods {sorted(methods)}")
        return

    if rule == "no_throttle_middleware":
        routes = runtime_routes(str(repo_root.resolve()))
        for record in all_surface_records:
            route_name = str(record.get("route_name", ""))
            route = routes.get(route_name)
            if route is None:
                raise RuntimeError(f"{surface_id}: runtime route is missing for {route_name}")
            middleware = [value.casefold() for value in middleware_values(route)]
            if any("throttle" in value for value in middleware):
                raise RuntimeError(f"{surface_id}: no_throttle_middleware is false for {route_name}")
        return

    if rule == "operator_surface_without_locale_route":
        if not surface_records:
            raise RuntimeError(f"{surface_id}: operator locale non-applicability requires rendered routes")
        if any(not str(record.get("route_name", "")).startswith("admin.") for record in surface_records):
            raise RuntimeError(f"{surface_id}: operator locale rule requires admin.* rendered routes")
        if any("{locale}" in str(record.get("uri", "")) for record in surface_records):
            raise RuntimeError(f"{surface_id}: operator locale rule is false because a rendered route has a locale segment")
        return

    raise RuntimeError(f"{surface_id}: unsupported strictness non-applicability rule {rule!r}")


def apply_surface_contract(
    surface_id: str,
    surface_records: list[dict[str, Any]],
    all_surface_records: list[dict[str, Any]],
    states: list[str],
    evidence_contract: dict[str, Any] | None,
    repo_root: Path,
) -> tuple[set[str], set[str], bool, bool, dict[str, Any]]:
    categories = strict_state_categories(states)
    locales = declared_locales(states)
    accessibility = has_accessibility_evidence(surface_records)
    overflow = has_overflow_evidence(surface_records)
    summary: dict[str, Any] = {
        "covered_state_categories": [],
        "not_applicable_state_categories": [],
        "locale_status": "declared" if REQUIRED_LOCALES.issubset(locales) else "incomplete",
        "accessibility_status": "declared" if accessibility else "incomplete",
        "overflow_status": "declared" if overflow else "incomplete",
    }

    surfaces = evidence_contract.get("surfaces", {}) if isinstance(evidence_contract, dict) else {}
    entry = surfaces.get(surface_id) if isinstance(surfaces, dict) else None
    if not isinstance(entry, dict):
        return categories, locales, accessibility, overflow, summary

    state_entries = entry.get("state_categories", {})
    if not isinstance(state_entries, dict):
        raise RuntimeError(f"{surface_id}: state_categories must be an object")
    for category, disposition in state_entries.items():
        if category not in REQUIRED_STATE_CATEGORIES or not isinstance(disposition, dict):
            raise RuntimeError(f"{surface_id}: invalid strictness state category {category!r}")
        status = disposition.get("status")
        if status == "covered":
            validate_evidence_reference(repo_root, f"{surface_id} {category}", disposition.get("evidence"))
            categories.add(category)
            summary["covered_state_categories"].append(category)
        elif status == "not_applicable":
            reason = disposition.get("reason")
            rule = disposition.get("rule")
            if not isinstance(reason, str) or len(reason.strip()) < 80 or not isinstance(rule, str):
                raise RuntimeError(f"{surface_id} {category}: non-applicability requires a bounded reason and rule")
            validate_non_applicability(surface_id, rule, surface_records, all_surface_records, repo_root)
            categories.add(category)
            summary["not_applicable_state_categories"].append(category)
        else:
            raise RuntimeError(f"{surface_id} {category}: status must be covered or not_applicable")

    locale_entry = entry.get("locale")
    if isinstance(locale_entry, dict):
        status = locale_entry.get("status")
        if status == "covered":
            validate_evidence_reference(repo_root, f"{surface_id} locale", locale_entry.get("evidence"))
            locales.update(REQUIRED_LOCALES)
            summary["locale_status"] = "covered"
        elif status == "not_applicable":
            reason = locale_entry.get("reason")
            rule = locale_entry.get("rule")
            if not isinstance(reason, str) or len(reason.strip()) < 80 or not isinstance(rule, str):
                raise RuntimeError(f"{surface_id} locale: non-applicability requires a bounded reason and rule")
            validate_non_applicability(surface_id, rule, surface_records, all_surface_records, repo_root)
            locales.update(REQUIRED_LOCALES)
            summary["locale_status"] = "not_applicable"
        else:
            raise RuntimeError(f"{surface_id} locale: status must be covered or not_applicable")

    for key in ("accessibility", "overflow"):
        proof = entry.get(key)
        if not isinstance(proof, dict):
            continue
        status = proof.get("status")
        if status != "covered":
            raise RuntimeError(f"{surface_id} {key}: only covered evidence is supported")
        validate_evidence_reference(repo_root, f"{surface_id} {key}", proof.get("evidence"))
        if key == "accessibility":
            accessibility = True
            summary["accessibility_status"] = "covered"
        else:
            overflow = True
            summary["overflow_status"] = "covered"

    summary["covered_state_categories"] = sorted(set(summary["covered_state_categories"]))
    summary["not_applicable_state_categories"] = sorted(set(summary["not_applicable_state_categories"]))
    return categories, locales, accessibility, overflow, summary


def strict_surface_findings(
    records: list[dict[str, Any]],
    evidence_contract: dict[str, Any] | None = None,
    repo_root: Path = REPO_ROOT,
) -> tuple[list[dict[str, Any]], dict[str, set[str]], dict[str, dict[str, Any]]]:
    rendered_by_surface: dict[str, list[dict[str, Any]]] = defaultdict(list)
    all_by_surface: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        if record.get("surface_id"):
            all_by_surface[str(record["surface_id"])].append(record)
        if record.get("kind") == "rendered_screen" and record.get("surface_id"):
            rendered_by_surface[str(record["surface_id"])].append(record)

    findings: list[dict[str, Any]] = []
    finding_ids_by_surface: dict[str, set[str]] = defaultdict(set)
    summaries: dict[str, dict[str, Any]] = {}
    for surface_id, surface_records in sorted(rendered_by_surface.items()):
        representative = surface_records[0]
        module = str(representative.get("module", "quality_e2e"))
        states = sorted({str(state) for row in surface_records for state in row.get("states", [])})
        categories, locales, accessibility, overflow, summary = apply_surface_contract(
            surface_id,
            surface_records,
            all_by_surface.get(surface_id, []),
            states,
            evidence_contract,
            repo_root,
        )
        summaries[surface_id] = summary

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
                        "strictness_contract": summary,
                    },
                    "impact": "Issue #326 requires explicit applicability or evidence for 404, 419, 429, server/dependency failure and recovery; validation errors and generic missing states cannot substitute for HTTP evidence.",
                    "disposition": "Declare and execute each missing category, or persist an owner-approved non-applicability rule with exact evidence.",
                }
            )

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
                        "strictness_contract": summary,
                    },
                    "impact": "Locale completeness cannot be inferred from a generic localization marker or from another surface.",
                    "disposition": "Declare and execute exact-head English and Polish evidence, or persist an owner-approved locale non-applicability rule.",
                }
            )

        if not accessibility:
            identifier = AUDIT.finding_id("ACCESSIBILITY", surface_id)
            finding_ids_by_surface[surface_id].add(identifier)
            findings.append(
                {
                    "id": identifier,
                    "severity": "MEDIUM",
                    "module": module,
                    "subject": surface_id,
                    "title": f"{surface_id} lacks explicit accessibility evidence",
                    "evidence": {"evidence_text": evidence_text(surface_records)[:1000], "strictness_contract": summary},
                    "impact": "Responsive or functional evidence does not establish accessibility or keyboard/semantic closure.",
                    "disposition": "Add bounded exact-head accessibility evidence or persist an owner-approved non-applicability rule.",
                }
            )

        if not overflow:
            identifier = AUDIT.finding_id("OVERFLOW", surface_id)
            finding_ids_by_surface[surface_id].add(identifier)
            findings.append(
                {
                    "id": identifier,
                    "severity": "MEDIUM",
                    "module": module,
                    "subject": surface_id,
                    "title": f"{surface_id} lacks explicit horizontal-overflow evidence",
                    "evidence": {"evidence_text": evidence_text(surface_records)[:1000], "declared_states": states, "strictness_contract": summary},
                    "impact": "Viewport declarations do not prove absence of clipped controls or horizontal scrolling.",
                    "disposition": "Add exact-head overflow assertions at applicable viewport and long-content boundaries.",
                }
            )
    return findings, finding_ids_by_surface, summaries


def apply_strictness(matrix: dict[str, Any], repo_root: Path = REPO_ROOT) -> dict[str, Any]:
    route_records = [row for row in matrix.get("route_records", []) if isinstance(row, dict)]
    retained_findings = [
        row for row in matrix.get("findings", [])
        if isinstance(row, dict) and not is_generated_strictness_finding(row)
    ]
    evidence_contract = load_strictness_evidence(repo_root)
    strict_findings, ids_by_surface, summaries = strict_surface_findings(route_records, evidence_contract, repo_root)

    for record in route_records:
        identifiers = {
            str(value) for value in record.get("finding_ids", [])
            if not str(value).startswith(GENERATED_PREFIXES)
        }
        surface_id = str(record.get("surface_id", ""))
        identifiers.update(ids_by_surface.get(surface_id, set()))
        record["finding_ids"] = sorted(identifiers)
        record["locales"] = sorted(declared_locales([str(value) for value in record.get("states", [])]))
        if surface_id in summaries:
            record["strictness_evidence"] = summaries[surface_id]

    matrix["route_records"] = route_records
    matrix["findings"] = AUDIT.deduplicate_findings([*retained_findings, *strict_findings])
    matrix["counts"]["findings"] = len(matrix["findings"])
    matrix["contracts"]["strictness_evidence_schema"] = evidence_contract.get("schema_version")
    matrix["module_summaries"] = RECONCILE.rebuild_module_summaries(matrix, matrix["findings"])
    return matrix


def validate_strictness(matrix: dict[str, Any], repo_root: Path = REPO_ROOT) -> list[str]:
    route_records = [row for row in matrix.get("route_records", []) if isinstance(row, dict)]
    expected, _, _ = strict_surface_findings(route_records, load_strictness_evidence(repo_root), repo_root)
    expected_ids = {str(row.get("id")) for row in expected}
    actual_ids = {
        str(row.get("id"))
        for row in matrix.get("findings", [])
        if isinstance(row, dict) and is_generated_strictness_finding(row)
    }
    errors: list[str] = []
    for identifier in sorted(expected_ids - actual_ids):
        errors.append(f"missing required strictness finding {identifier}")
    for identifier in sorted(actual_ids - expected_ids):
        errors.append(f"unexpected stale strictness finding {identifier}")
    return errors


def update_summary(output: Path, matrix: dict[str, Any]) -> None:
    severity_counts = Counter(str(row.get("severity", "UNKNOWN")) for row in matrix["findings"])
    payload = {
        "exact_sha": matrix["exact_sha"],
        "finding_count": len(matrix["findings"]),
        "severity_counts": dict(sorted(severity_counts.items())),
        "state_rule": "precise 404 419 429 server failure and recovery markers require explicit coverage or machine-verified non-applicability",
        "locale_rule": "both en and pl require explicit coverage or machine-verified operator-surface non-applicability",
        "accessibility_rule": "accessibility requires an exact source evidence marker or a finding",
        "overflow_rule": "horizontal overflow requires an exact source assertion or a finding",
        "strictness_evidence_contract": str(STRICTNESS_EVIDENCE_PATH),
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
    matrix = apply_strictness(read_json(matrix_path), REPO_ROOT)
    errors = validate_strictness(matrix, REPO_ROOT)
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
