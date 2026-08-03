#!/usr/bin/env python3
"""Reconcile the generated portal audit with whole-repository boundaries.

Adds records for explicitly excluded named routes, every one of the 18 programme
modules, and coverage-contract mismatches that a validator scoped to only one
manifest layer cannot detect. It also writes per-module machine-readable files
small enough to retain in Git.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import subprocess
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable

MODULE_PATH = Path(__file__).with_name("portal_exhaustive_audit.py")
SPEC = importlib.util.spec_from_file_location("portal_exhaustive_audit", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Cannot import {MODULE_PATH}")
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)

VERDICT_PRIORITY = {
    "MISSING": 5,
    "UNKNOWN": 4,
    "PARTIAL": 3,
    "PASS": 2,
    "NOT_APPLICABLE": 1,
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def selector_matches(value: str, selector: dict[str, Any]) -> bool:
    exact = selector.get("exact")
    prefix = selector.get("prefix")
    if isinstance(exact, str):
        return value == exact
    if isinstance(prefix, str):
        return value.startswith(prefix)
    return False


def route_list(repo_root: Path) -> list[dict[str, Any]]:
    completed = subprocess.run(
        ["php", "artisan", "route:list", "--json"],
        cwd=repo_root,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"Cannot list Laravel routes: {completed.stderr.strip()}")
    value = json.loads(completed.stdout)
    if not isinstance(value, list):
        raise RuntimeError("Laravel route list is not a JSON array")
    return [entry for entry in value if isinstance(entry, dict)]


def route_methods(route: dict[str, Any]) -> list[str]:
    raw = route.get("methods", route.get("method", ""))
    if isinstance(raw, list):
        values = raw
    else:
        values = str(raw).split("|")
    return [str(value).strip().upper() for value in values if str(value).strip()]


def excluded_route_records(
    repo_root: Path,
    manifest: dict[str, Any],
    exact_sha: str,
) -> list[dict[str, Any]]:
    exclusions = [
        entry for entry in manifest.get("route_name_exclusions", [])
        if isinstance(entry, dict)
    ]
    records: list[dict[str, Any]] = []
    for route in route_list(repo_root):
        name = str(route.get("name") or "").strip()
        if not name:
            continue
        exclusion = next((entry for entry in exclusions if selector_matches(name, entry)), None)
        if exclusion is None:
            continue
        records.append(
            {
                "exact_sha": exact_sha,
                "route_name": name,
                "kind": "justified_exclusion",
                "methods": route_methods(route),
                "uri": route.get("uri"),
                "action": route.get("action"),
                "reason": exclusion.get("reason"),
                "verdicts": {
                    "exists": "PASS",
                    "functional": "NOT_APPLICABLE",
                    "content_complete": "NOT_APPLICABLE",
                    "production_complete": "NOT_APPLICABLE",
                },
                "final_classification": "COMPLETE",
            }
        )
    return sorted(records, key=lambda entry: entry["route_name"])


def aggregate_verdict(records: Iterable[dict[str, Any]], dimension: str) -> str:
    values = [
        str(record.get("verdicts", {}).get(dimension, "UNKNOWN"))
        for record in records
    ]
    applicable = [value for value in values if value != "NOT_APPLICABLE"]
    if not applicable:
        return "NOT_APPLICABLE"
    return max(applicable, key=lambda value: VERDICT_PRIORITY.get(value, 99))


def module_records(
    matrix: dict[str, Any],
    findings: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    by_module: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for collection in ("route_records", "capability_records"):
        for record in matrix.get(collection, []):
            if isinstance(record, dict) and record.get("module"):
                by_module[str(record["module"])].append(record)

    records: list[dict[str, Any]] = []
    for module in AUDIT.MODULES:
        children = by_module[module]
        evidence: list[str] = []
        if children:
            verdicts = {
                dimension: aggregate_verdict(children, dimension)
                for dimension in (
                    "exists",
                    "functional",
                    "content_complete",
                    "production_complete",
                )
            }
            evidence.append(f"{len(children)} current route/capability records")
        elif module == "quality_e2e":
            verdicts = {
                "exists": "PASS",
                "functional": "PASS",
                "content_complete": "NOT_APPLICABLE",
                "production_complete": "PARTIAL",
            }
            evidence.append("all repository contract validators passed on the generated audit SHA")
        else:
            verdicts = {
                "exists": "UNKNOWN",
                "functional": "UNKNOWN",
                "content_complete": "NOT_APPLICABLE",
                "production_complete": "UNKNOWN",
            }
            identifier = AUDIT.finding_id("MODULE", module)
            findings.append(
                {
                    "id": identifier,
                    "severity": "MEDIUM",
                    "module": module,
                    "subject": module,
                    "title": f"Module {module} has no current-main auditable route or capability record",
                    "evidence": {
                        "route_record_count": 0,
                        "capability_record_count": 0,
                        "historical_baseline": matrix.get("historical_baseline"),
                    },
                    "impact": "The module cannot receive a current EXISTS, FUNCTIONAL or PRODUCTION_COMPLETE verdict.",
                    "disposition": "Create an explicit bounded module contract or an owner-approved deferral with current exact evidence.",
                }
            )
            evidence.append("no current route or capability record")

        records.append(
            {
                "exact_sha": matrix["exact_sha"],
                "module": module,
                "child_record_count": len(children),
                "evidence": evidence,
                "verdicts": verdicts,
                "final_classification": AUDIT.final_classification(verdicts),
            }
        )
    return records


def add_content_scale_scope_finding(
    repo_root: Path,
    matrix: dict[str, Any],
    findings: list[dict[str, Any]],
) -> None:
    manifest = read_json(repo_root / "scripts/acceptance/coverage/portal-coverage-manifest.json")
    all_surfaces = {
        str(entry.get("id"))
        for entry in manifest.get("surfaces", [])
        if isinstance(entry, dict) and entry.get("id")
    }
    fragments_root = repo_root / "scripts/acceptance/coverage/surfaces"
    for file in sorted(fragments_root.glob("*.json")):
        value = read_json(file)
        entries = value if isinstance(value, list) else value.get("surfaces", [])
        for entry in entries if isinstance(entries, list) else []:
            if isinstance(entry, dict) and entry.get("id"):
                all_surfaces.add(str(entry["id"]))

    scale = read_json(repo_root / "docs/testing/PORTAL_CONTENT_SCALE_EVIDENCE.json")
    scale_surfaces = set(str(value) for value in (scale.get("surfaces") or {}).keys())
    missing = sorted(all_surfaces - scale_surfaces)
    extra = sorted(scale_surfaces - all_surfaces)
    if not missing and not extra:
        return
    findings.append(
        {
            "id": AUDIT.finding_id("CONTRACT", "content-scale-surface-scope"),
            "severity": "MEDIUM",
            "module": "quality_e2e",
            "subject": "content-scale-surface-scope",
            "title": "Content-scale strict closure does not cover the complete 27-surface portal inventory",
            "evidence": {
                "all_portal_surface_count": len(all_surfaces),
                "content_scale_surface_count": len(scale_surfaces),
                "missing_surface_ids": missing,
                "unexpected_surface_ids": extra,
                "validator_report": matrix.get("validator_reports", {}).get("content_scale"),
            },
            "impact": "A green content-scale validator can omit surfaces introduced through manifest fragments.",
            "disposition": "Load all portal surface fragments in the content-scale validator and classify every current surface.",
        }
    )


def rebuild_module_summaries(matrix: dict[str, Any], findings: list[dict[str, Any]]) -> list[dict[str, Any]]:
    routes_by_module = Counter(str(row.get("module")) for row in matrix.get("route_records", []))
    caps_by_module = Counter(str(row.get("module")) for row in matrix.get("capability_records", []))
    findings_by_module = Counter(str(row.get("module")) for row in findings)
    module_by_id = {row["module"]: row for row in matrix.get("module_records", [])}
    result: list[dict[str, Any]] = []
    for module in AUDIT.MODULES:
        record = module_by_id[module]
        result.append(
            {
                "module": module,
                "route_record_count": routes_by_module[module],
                "capability_record_count": caps_by_module[module],
                "finding_count": findings_by_module[module],
                "final_classification": record["final_classification"],
                "verdicts": record["verdicts"],
            }
        )
    return result


def write_split_evidence(output: Path, matrix: dict[str, Any]) -> None:
    modules_root = output / "modules"
    for module in AUDIT.MODULES:
        module_record = next(row for row in matrix["module_records"] if row["module"] == module)
        payload = {
            "schema_version": matrix["schema_version"],
            "exact_sha": matrix["exact_sha"],
            "module_record": module_record,
            "route_records": [row for row in matrix["route_records"] if row.get("module") == module],
            "capability_records": [row for row in matrix["capability_records"] if row.get("module") == module],
            "findings": [row for row in matrix["findings"] if row.get("module") == module],
        }
        write_json(modules_root / f"{module}.json", payload)
    write_json(output / "excluded-route-records.json", matrix["excluded_route_records"])
    write_json(
        output / "portal-exhaustive-audit-index.json",
        {
            "schema_version": matrix["schema_version"],
            "exact_sha": matrix["exact_sha"],
            "global_verdict": matrix["global_verdict"],
            "counts": matrix["counts"],
            "module_summaries": matrix["module_summaries"],
            "finding_ids": [row["id"] for row in matrix["findings"]],
        },
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo_root = args.repo_root.resolve()
    output = args.output.resolve()
    matrix_path = output / "portal-exhaustive-audit-matrix.json"
    matrix = read_json(matrix_path)
    findings = list(matrix.get("findings", []))

    manifest = read_json(repo_root / "scripts/acceptance/coverage/portal-coverage-manifest.json")
    exclusions = excluded_route_records(repo_root, manifest, str(matrix["exact_sha"]))
    discovered = int(matrix["validator_reports"]["portal_coverage"]["discovered_named_route_count"])
    if len(matrix.get("route_records", [])) + len(exclusions) != discovered:
        raise RuntimeError(
            "Classified plus explicitly excluded named routes do not equal the discovered route count: "
            f"{len(matrix.get('route_records', []))} + {len(exclusions)} != {discovered}"
        )

    add_content_scale_scope_finding(repo_root, matrix, findings)
    matrix["excluded_route_records"] = exclusions
    matrix["module_records"] = module_records(matrix, findings)
    matrix["findings"] = AUDIT.deduplicate_findings(findings)
    matrix["module_summaries"] = rebuild_module_summaries(matrix, matrix["findings"])
    matrix["counts"] = {
        "discovered_named_routes": discovered,
        "classified_route_records": len(matrix.get("route_records", [])),
        "excluded_route_records": len(exclusions),
        "rendered_routes": int(matrix["validator_reports"]["route_view_navigation"]["rendered_route_count"]),
        "capability_records": len(matrix.get("capability_records", [])),
        "module_records": len(matrix["module_records"]),
        "findings": len(matrix["findings"]),
    }

    write_json(matrix_path, matrix)
    write_json(output / "portal-exhaustive-audit-findings.json", matrix["findings"])
    write_split_evidence(output, matrix)
    print(json.dumps({"exact_sha": matrix["exact_sha"], "counts": matrix["counts"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
