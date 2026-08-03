#!/usr/bin/env python3
"""Generate a fail-closed current-main portal completeness audit.

The generator reuses the repository's authoritative route/view/navigation,
coverage, capability, dimension, content-scale and media contracts. Product
incompleteness is reported as findings; only invalid audit inputs make the
command fail.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

AUDIT_SCHEMA_VERSION = 1
HISTORICAL_AUDIT_HEAD = "2ec4e35a116a051f5841930ef750119458268050"
HISTORICAL_TARGET = "b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608"
PARENT_ISSUE = 326

MODULES = (
    "identity",
    "accounts",
    "characters",
    "public_game_data",
    "cms_content",
    "editorial_media",
    "wiki",
    "support_moderation",
    "admin_rbac_audit",
    "wallet_marketplace",
    "game_catalog",
    "platform_api",
    "payments",
    "products_entitlements",
    "legal_privacy_commerce",
    "operations_observability",
    "public_edge",
    "quality_e2e",
)

MODULE_PREFIXES: tuple[tuple[str, str], ...] = (
    ("identity.character-profile", "characters"),
    ("account.character", "characters"),
    ("identity.", "identity"),
    ("account.", "accounts"),
    ("public.game-data", "public_game_data"),
    ("public.community", "public_game_data"),
    ("community.", "public_game_data"),
    ("wiki.", "wiki"),
    ("editorial-media", "editorial_media"),
    ("browser-supporting-media", "editorial_media"),
    ("support.moderation", "support_moderation"),
    ("support-legal", "legal_privacy_commerce"),
    ("admin.", "admin_rbac_audit"),
    ("marketplace.", "wallet_marketplace"),
    ("game-catalog.", "game_catalog"),
    ("downloads.", "cms_content"),
    ("events.", "cms_content"),
    ("announcements.", "cms_content"),
    ("public.news", "cms_content"),
    ("public.home", "cms_content"),
    ("public.localization", "cms_content"),
)

CAPABILITY_MODULE_PREFIXES: tuple[tuple[str, str], ...] = (
    ("account.", "accounts"),
    ("character.", "characters"),
    ("commerce.", "payments"),
    ("support.", "support_moderation"),
    ("public.", "public_game_data"),
    ("knowledge.", "game_catalog"),
)

REQUIRED_VIEWPORTS = {
    "desktop-1440x1000",
    "tablet-820x1180",
    "mobile-390x844",
}

STATE_CATEGORY_PATTERNS: dict[str, tuple[str, ...]] = {
    "empty": ("empty", "no-result", "no_result"),
    "validation": ("validation", "invalid"),
    "authorization": ("authorization", "permission", "denied", "forbidden"),
    "conflict": ("conflict", "duplicate", "replay", "quota"),
    "not_found": ("not-found", "not_found", "404", "missing"),
    "rate_limit": ("429", "rate", "throttle"),
    "server_failure": ("500", "failure", "error", "unavailable", "503"),
    "recovery": ("recovery", "restored", "retry", "recoverable"),
}

VALID_VERDICTS = {"PASS", "PARTIAL", "MISSING", "UNKNOWN", "NOT_APPLICABLE"}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False, sort_keys=False) + "\n", encoding="utf-8")


def load_fragments(root: Path, key: str = "surfaces") -> list[dict[str, Any]]:
    if not root.exists():
        return []
    records: list[dict[str, Any]] = []
    for file in sorted(root.glob("*.json")):
        value = read_json(file)
        entries = value if isinstance(value, list) else value.get(key, [])
        if isinstance(entries, list):
            records.extend(entry for entry in entries if isinstance(entry, dict))
    return records


def run_json_command(repo_root: Path, command: list[str]) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        cwd=repo_root,
        env=os.environ.copy(),
        text=True,
        capture_output=True,
        check=False,
    )
    try:
        report = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        return {
            "errors": [
                f"Command did not return JSON: {' '.join(command)}: {exc}; "
                f"stderr={completed.stderr.strip()[:500]}"
            ],
            "warnings": [],
            "command": command,
            "returncode": completed.returncode,
        }
    if not isinstance(report, dict):
        return {
            "errors": [f"Command returned non-object JSON: {' '.join(command)}"],
            "warnings": [],
            "command": command,
            "returncode": completed.returncode,
        }
    report["command"] = command
    report["returncode"] = completed.returncode
    if completed.returncode != 0 and not report.get("errors"):
        report["errors"] = [
            f"Command failed without structured errors: {' '.join(command)}; "
            f"stderr={completed.stderr.strip()[:500]}"
        ]
    return report


def stable_slug(value: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", value.casefold()).strip("-")
    digest = hashlib.sha256(value.encode("utf-8")).hexdigest()[:8]
    return f"{slug[:48]}-{digest}" if slug else digest


def finding_id(category: str, subject: str) -> str:
    return f"OTERYN-AUDIT-CURRENT-{category}-{stable_slug(subject).upper()}"


def module_for_surface(surface_id: str | None) -> str:
    value = surface_id or ""
    for prefix, module in MODULE_PREFIXES:
        if value.startswith(prefix):
            return module
    return "quality_e2e"


def module_for_capability(capability_id: str) -> str:
    if capability_id.startswith("commerce.provider"):
        return "payments"
    if capability_id.startswith("commerce.products") or capability_id.startswith("commerce.game-code"):
        return "products_entitlements"
    if capability_id.startswith("knowledge."):
        return "game_catalog"
    for prefix, module in CAPABILITY_MODULE_PREFIXES:
        if capability_id.startswith(prefix):
            return module
    return "quality_e2e"


def state_categories(states: Iterable[str]) -> list[str]:
    normalized = [str(state).casefold() for state in states]
    present: list[str] = []
    for category, patterns in STATE_CATEGORY_PATTERNS.items():
        if any(any(pattern in state for pattern in patterns) for state in normalized):
            present.append(category)
    return present


def final_classification(verdicts: dict[str, str]) -> str:
    applicable = [value for value in verdicts.values() if value != "NOT_APPLICABLE"]
    if any(value not in VALID_VERDICTS for value in verdicts.values()):
        raise ValueError(f"Unsupported verdicts: {verdicts}")
    if applicable and all(value == "PASS" for value in applicable):
        return "COMPLETE"
    if any(value == "MISSING" for value in applicable):
        return "MISSING"
    if any(value == "UNKNOWN" for value in applicable):
        return "BLOCKED"
    return "PARTIAL"


def evidence_markers(surface: dict[str, Any]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    for evidence in surface.get("evidence", []) if isinstance(surface.get("evidence"), list) else []:
        if not isinstance(evidence, dict):
            continue
        result.append(
            {
                "file": evidence.get("file"),
                "markers": evidence.get("markers", []),
            }
        )
    return result


def load_dimension_surfaces(repo_root: Path) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    base = read_json(repo_root / "scripts/acceptance/coverage/portal-evidence-dimensions.json")
    records: list[dict[str, Any]] = []
    for relative in base.get("surface_fragments", []):
        value = read_json(repo_root / "scripts/acceptance/coverage" / relative)
        entries = value if isinstance(value, list) else value.get("surfaces", [])
        if isinstance(entries, list):
            records.extend(entry for entry in entries if isinstance(entry, dict))
    return base, {str(record.get("id")): record for record in records if record.get("id")}


def load_surfaces(repo_root: Path) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    manifest = read_json(repo_root / "scripts/acceptance/coverage/portal-coverage-manifest.json")
    records = [entry for entry in manifest.get("surfaces", []) if isinstance(entry, dict)]
    records.extend(load_fragments(repo_root / "scripts/acceptance/coverage/surfaces"))
    return manifest, {str(record.get("id")): record for record in records if record.get("id")}


def content_scale_records(repo_root: Path) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    contract = read_json(repo_root / "docs/testing/PORTAL_CONTENT_SCALE_EVIDENCE.json")
    records = contract.get("surfaces", {})
    return contract, records if isinstance(records, dict) else {}


def media_records(repo_root: Path) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    contract = read_json(repo_root / "docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json")
    entries = contract.get("surfaces", [])
    records = {
        str(entry.get("id")): entry
        for entry in entries
        if isinstance(entry, dict) and entry.get("id")
    }
    return contract, records


def expected_inventory_status(repo_root: Path, module: str) -> tuple[str, str | None]:
    candidates = {
        "wiki": repo_root / "docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json",
        "game_catalog": repo_root / "docs/testing/GAME_CATALOG_EXPECTED_CONTENT_INVENTORY.json",
    }
    path = candidates.get(module)
    if path is None:
        return "NOT_APPLICABLE", None
    if not path.exists():
        return "MISSING", str(path.relative_to(repo_root))
    value = read_json(path)
    if value.get("status") == "complete" and value.get("expected_records"):
        return "PASS", str(path.relative_to(repo_root))
    return "PARTIAL", str(path.relative_to(repo_root))


def route_record(
    raw: dict[str, Any],
    surface: dict[str, Any] | None,
    dimension: dict[str, Any] | None,
    scale: dict[str, Any] | None,
    media: dict[str, Any] | None,
    repo_root: Path,
    exact_sha: str,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    findings: list[dict[str, Any]] = []
    surface_id = raw.get("surface_id")
    module = module_for_surface(surface_id)
    kind = raw.get("kind")
    roles = list(surface.get("roles", [])) if surface else []
    states = list(surface.get("states", [])) if surface else []
    declared_categories = state_categories(states)
    evidence_layers = set(surface.get("evidence_layers", [])) if surface else set()
    markers = evidence_markers(surface or {})

    viewport_names = set((dimension or {}).get("viewports", {}).keys())
    critical = bool((dimension or {}).get("critical"))
    missing_viewports = sorted(REQUIRED_VIEWPORTS - viewport_names) if kind == "rendered_screen" else []
    portability = (dimension or {}).get("portability", {})
    portability_status = portability.get("status") if isinstance(portability, dict) else None

    exists = "PASS"
    if kind in {"resource_supporting", "exception"}:
        functional = "PASS"
        content_complete = "NOT_APPLICABLE"
    elif kind == "redirect":
        functional = "PASS"
        content_complete = "NOT_APPLICABLE"
    else:
        functional = "PARTIAL" if "playwright" in evidence_layers and markers else "UNKNOWN"
        inventory_status, inventory_path = expected_inventory_status(repo_root, module)
        if module in {"wiki", "game_catalog"}:
            content_complete = inventory_status
            if inventory_status != "PASS":
                finding = {
                    "id": finding_id("CONTENT", module),
                    "severity": "HIGH",
                    "module": module,
                    "subject": module,
                    "title": f"{module} lacks a complete authoritative expected-content inventory",
                    "evidence": inventory_path or "expected inventory contract is not present",
                    "impact": "CONTENT_COMPLETE cannot pass from route presence or sample fixtures.",
                    "disposition": "Create and validate an authoritative versioned expected inventory and reconcile every actual record.",
                }
                findings.append(finding)
        elif scale and scale.get("classification") not in {None, "not_applicable", "supporting_endpoint"}:
            content_complete = "PARTIAL"
        else:
            content_complete = "UNKNOWN"

    production_complete = "PASS"
    if kind == "rendered_screen":
        production_complete = "PARTIAL"
        if missing_viewports:
            findings.append(
                {
                    "id": finding_id("VIEWPORT", str(surface_id)),
                    "severity": "MEDIUM" if critical else "LOW",
                    "module": module,
                    "subject": surface_id,
                    "title": f"{surface_id} lacks explicit evidence for required viewports",
                    "evidence": {"missing_viewports": missing_viewports, "declared_viewports": sorted(viewport_names)},
                    "impact": "PRODUCTION_COMPLETE cannot pass for all required responsive boundaries.",
                    "disposition": "Add zero-retry exact-head evidence for each missing viewport or a justified non-applicability rule.",
                }
            )
        if critical and portability_status == "excluded":
            findings.append(
                {
                    "id": finding_id("PORTABILITY", str(surface_id)),
                    "severity": "MEDIUM",
                    "module": module,
                    "subject": surface_id,
                    "title": f"{surface_id} has no Firefox/WebKit execution evidence",
                    "evidence": portability.get("rationale"),
                    "impact": "Risk-based portability is explicitly excluded for a critical surface.",
                    "disposition": "Retain exclusion only with accepted risk ownership or add bounded Firefox/WebKit evidence.",
                }
            )
        if not {"server_failure", "recovery"}.intersection(declared_categories):
            findings.append(
                {
                    "id": finding_id("STATE", str(surface_id)),
                    "severity": "MEDIUM" if critical else "LOW",
                    "module": module,
                    "subject": surface_id,
                    "title": f"{surface_id} does not explicitly declare failure and recovery coverage",
                    "evidence": {"declared_states": states, "state_categories": declared_categories},
                    "impact": "The Issue #326 error/dependency state matrix remains incomplete or implicit.",
                    "disposition": "Classify applicable 500/503/failure/restoration states explicitly and attach deterministic evidence.",
                }
            )

    verdicts = {
        "exists": exists,
        "functional": functional,
        "content_complete": content_complete,
        "production_complete": production_complete,
    }
    record = {
        "exact_sha": exact_sha,
        "historical_baseline_head": HISTORICAL_AUDIT_HEAD,
        "module": module,
        "surface_id": surface_id,
        "route_name": raw.get("route_name"),
        "kind": kind,
        "methods": raw.get("methods", []),
        "uri": raw.get("uri"),
        "action": raw.get("action"),
        "views": raw.get("views", []),
        "roles": roles,
        "states": states,
        "state_categories": declared_categories,
        "locales": [locale for locale in ("en", "pl") if any(locale in state.casefold() for state in states)],
        "viewports": sorted(viewport_names),
        "browsers": sorted((dimension or {}).get("browsers", {}).keys()),
        "portability_status": portability_status,
        "evidence_layers": sorted(evidence_layers),
        "evidence": markers,
        "content_scale": scale,
        "media_contract": media,
        "verdicts": verdicts,
        "final_classification": final_classification(verdicts),
        "finding_ids": sorted({finding["id"] for finding in findings}),
    }
    return record, findings


def capability_records(
    product: dict[str, Any],
    frontend: dict[str, Any],
    exact_sha: str,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    product_by_id = {
        str(entry.get("id")): entry
        for entry in product.get("capabilities", [])
        if isinstance(entry, dict) and entry.get("id")
    }
    records: list[dict[str, Any]] = []
    findings: list[dict[str, Any]] = []
    for entry in frontend.get("capabilities", []):
        if not isinstance(entry, dict) or not entry.get("id"):
            continue
        capability_id = str(entry["id"])
        product_entry = product_by_id.get(capability_id, {})
        delivery = product_entry.get("delivery_status")
        backend = entry.get("backend_status")
        frontend_status = entry.get("frontend_status")
        integration = entry.get("integration_status")
        user_facing = bool(entry.get("user_facing"))

        exists = "MISSING" if backend == "missing" else ("PARTIAL" if backend == "partial" else "PASS")
        if integration == "implemented":
            functional = "PARTIAL"
        elif integration == "partial":
            functional = "PARTIAL"
        elif integration == "not_applicable":
            functional = "NOT_APPLICABLE"
        else:
            functional = "MISSING"
        if delivery == "implemented":
            content = "PARTIAL"
        elif delivery == "partial":
            content = "PARTIAL"
        elif delivery == "not_applicable":
            content = "NOT_APPLICABLE"
        elif delivery == "missing" or backend == "missing":
            content = "MISSING"
        else:
            content = "UNKNOWN"
        production = "PARTIAL" if functional in {"PASS", "PARTIAL"} else functional

        verdicts = {
            "exists": exists,
            "functional": functional,
            "content_complete": content,
            "production_complete": production,
        }
        module = module_for_capability(capability_id)
        record = {
            "exact_sha": exact_sha,
            "module": module,
            "capability_id": capability_id,
            "user_facing": user_facing,
            "delivery_status": delivery,
            "backend_status": backend,
            "frontend_status": frontend_status,
            "integration_status": integration,
            "surface_ids": entry.get("surface_ids", []),
            "exception_reason": entry.get("exception_reason"),
            "verdicts": verdicts,
            "final_classification": final_classification(verdicts),
        }
        records.append(record)

        if record["final_classification"] in {"MISSING", "PARTIAL", "BLOCKED"} and delivery != "not_applicable":
            severity = "HIGH" if user_facing and record["final_classification"] == "MISSING" else "MEDIUM"
            findings.append(
                {
                    "id": finding_id("CAPABILITY", capability_id),
                    "severity": severity,
                    "module": module,
                    "subject": capability_id,
                    "title": f"Capability {capability_id} is not complete across backend, frontend and integration",
                    "evidence": {
                        "delivery_status": delivery,
                        "backend_status": backend,
                        "frontend_status": frontend_status,
                        "integration_status": integration,
                        "surface_ids": entry.get("surface_ids", []),
                    },
                    "impact": "The user-facing or supporting capability cannot be classified COMPLETE under Issue #326.",
                    "disposition": "Track implementation and exact integrated browser evidence in the owning product Issue.",
                }
            )
    return records, findings


def deduplicate_findings(findings: Iterable[dict[str, Any]]) -> list[dict[str, Any]]:
    by_id: dict[str, dict[str, Any]] = {}
    for finding in findings:
        identifier = str(finding["id"])
        current = by_id.get(identifier)
        if current is None:
            by_id[identifier] = finding
            continue
        evidence = current.get("evidence")
        if not isinstance(evidence, list):
            evidence = [evidence]
        evidence.append(finding.get("evidence"))
        current["evidence"] = evidence
    return [by_id[key] for key in sorted(by_id)]


def build_module_summaries(
    routes: list[dict[str, Any]],
    capabilities: list[dict[str, Any]],
    findings: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    route_by_module: dict[str, list[dict[str, Any]]] = defaultdict(list)
    capability_by_module: dict[str, list[dict[str, Any]]] = defaultdict(list)
    finding_by_module: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in routes:
        route_by_module[record["module"]].append(record)
    for record in capabilities:
        capability_by_module[record["module"]].append(record)
    for finding in findings:
        finding_by_module[str(finding.get("module", "quality_e2e"))].append(finding)

    summaries: list[dict[str, Any]] = []
    for module in MODULES:
        module_routes = route_by_module[module]
        module_capabilities = capability_by_module[module]
        classifications = Counter(
            [record["final_classification"] for record in module_routes]
            + [record["final_classification"] for record in module_capabilities]
        )
        if classifications and set(classifications) == {"COMPLETE"} and not finding_by_module[module]:
            final = "COMPLETE"
        elif classifications.get("MISSING", 0) > 0:
            final = "MISSING"
        elif classifications.get("BLOCKED", 0) > 0:
            final = "BLOCKED"
        else:
            final = "PARTIAL"
        summaries.append(
            {
                "module": module,
                "route_record_count": len(module_routes),
                "capability_record_count": len(module_capabilities),
                "classification_counts": dict(sorted(classifications.items())),
                "finding_count": len(finding_by_module[module]),
                "material_finding_count": sum(
                    1 for finding in finding_by_module[module] if finding.get("severity") in {"CRITICAL", "HIGH", "MEDIUM"}
                ),
                "final_classification": final,
            }
        )
    return summaries


def markdown_summary(matrix: dict[str, Any]) -> str:
    lines = [
        "# Current-main exhaustive portal audit",
        "",
        f"- Exact SHA: `{matrix['exact_sha']}`",
        f"- Historical baseline: PR #381 head `{HISTORICAL_AUDIT_HEAD}` on `{HISTORICAL_TARGET}`",
        f"- Parent Issue: #{PARENT_ISSUE}",
        f"- Route records: {len(matrix['route_records'])}",
        f"- Capability records: {len(matrix['capability_records'])}",
        f"- Findings: {len(matrix['findings'])}",
        "",
        "## Module verdicts",
        "",
        "| Module | Routes | Capabilities | Findings | Final |",
        "|---|---:|---:|---:|---|",
    ]
    for module in matrix["module_summaries"]:
        lines.append(
            f"| `{module['module']}` | {module['route_record_count']} | "
            f"{module['capability_record_count']} | {module['finding_count']} | "
            f"**{module['final_classification']}** |"
        )
    lines.extend(
        [
            "",
            "## Evidence boundary",
            "",
            "This audit is fail-closed. Repository declarations and stable Playwright markers do not by themselves prove that every mapped test executed successfully on the exact audit SHA. Fresh GitHub Actions run IDs are recorded separately during closeout. Missing authoritative Wiki or Game Catalog expected inventories remain findings rather than inferred completeness.",
            "",
        ]
    )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--sha", default=os.environ.get("GITHUB_SHA", "UNKNOWN"))
    args = parser.parse_args(argv)

    repo_root = args.repo_root.resolve()
    output = args.output.resolve()
    exact_sha = str(args.sha).strip()
    if not re.fullmatch(r"[0-9a-f]{40}", exact_sha):
        print(f"ERROR: --sha must be a full lowercase commit SHA, got {exact_sha!r}", file=sys.stderr)
        return 2

    validators = {
        "route_view_navigation": ["node", "scripts/acceptance/coverage/validate-route-view-navigation-final.mjs"],
        "portal_coverage": ["node", "scripts/acceptance/coverage/validate-portal-coverage.mjs", "--strict"],
        "product_completeness": ["node", "scripts/acceptance/coverage/validate-product-completeness.mjs"],
        "backend_frontend": ["node", "scripts/acceptance/coverage/validate-backend-frontend-completeness.mjs"],
        "dimension_evidence": ["node", "scripts/acceptance/coverage/validate-dimension-evidence.mjs"],
        "content_scale": ["node", "scripts/acceptance/coverage/validate-portal-content-scale-evidence.mjs"],
        "media_states": ["node", "scripts/acceptance/coverage/validate-portal-media-strict-closure.mjs"],
    }
    validation_reports = {
        name: run_json_command(repo_root, command) for name, command in validators.items()
    }

    infrastructure_errors: list[str] = []
    for name, report in validation_reports.items():
        for error in report.get("errors", []):
            infrastructure_errors.append(f"{name}: {error}")

    manifest, surfaces = load_surfaces(repo_root)
    dimension_contract, dimensions = load_dimension_surfaces(repo_root)
    scale_contract, scales = content_scale_records(repo_root)
    media_contract, media = media_records(repo_root)
    product = read_json(repo_root / "docs/testing/product-completeness-benchmark.json")
    frontend = read_json(repo_root / "docs/testing/product-backend-frontend-completeness.json")

    route_inventory = validation_reports["route_view_navigation"].get("inventory", [])
    route_records: list[dict[str, Any]] = []
    findings: list[dict[str, Any]] = []
    for raw in route_inventory:
        if not isinstance(raw, dict):
            continue
        surface_id = raw.get("surface_id")
        record, route_findings = route_record(
            raw,
            surfaces.get(str(surface_id)) if surface_id else None,
            dimensions.get(str(surface_id)) if surface_id else None,
            scales.get(str(surface_id)) if surface_id else None,
            media.get(str(surface_id)) if surface_id else None,
            repo_root,
            exact_sha,
        )
        route_records.append(record)
        findings.extend(route_findings)

    capability_rows, capability_findings = capability_records(product, frontend, exact_sha)
    findings.extend(capability_findings)
    findings.append(
        {
            "id": finding_id("PR", "historical-pr-381"),
            "severity": "LOW",
            "module": "quality_e2e",
            "subject": "PR #381",
            "title": "Historical audit PR #381 is stale and not mergeable against current main",
            "evidence": {
                "head": HISTORICAL_AUDIT_HEAD,
                "frozen_target": HISTORICAL_TARGET,
                "current_relationship": "historical-source-only",
            },
            "impact": "The old PR cannot serve as current exact-head evidence or terminal repository state.",
            "disposition": "Retain links and frozen evidence identity, then close PR #381 as superseded after this audit persists current-main evidence.",
        }
    )
    findings = deduplicate_findings(findings)
    module_summaries = build_module_summaries(route_records, capability_rows, findings)

    matrix = {
        "schema_version": AUDIT_SCHEMA_VERSION,
        "task_id": "OTERYN-20260803-portal-exhaustive-current-main-audit",
        "parent_issue": PARENT_ISSUE,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "exact_sha": exact_sha,
        "historical_baseline": {
            "pr": 381,
            "head": HISTORICAL_AUDIT_HEAD,
            "target": HISTORICAL_TARGET,
            "surface_groups": 27,
            "named_routes": 240,
            "rendered_screens": 126,
            "benchmark_capabilities": 43,
            "modules": 18,
            "relationship": "frozen historical evidence; not current exact-head proof",
        },
        "contracts": {
            "portal_manifest_schema": manifest.get("schema_version"),
            "dimension_schema": dimension_contract.get("schema_version"),
            "content_scale_schema": scale_contract.get("schema_version"),
            "media_schema": media_contract.get("schema_version"),
        },
        "validator_reports": validation_reports,
        "infrastructure_errors": infrastructure_errors,
        "route_records": route_records,
        "capability_records": capability_rows,
        "module_summaries": module_summaries,
        "findings": findings,
        "global_verdict": "AUDIT_INVALID" if infrastructure_errors else "AUDIT_COMPLETE_WITH_FINDINGS",
        "nonclaims": [
            "Repository evidence is not production deployment proof.",
            "Stable test markers are not substituted for exact-head workflow conclusions.",
            "Missing expected content inventories are not inferred from sample fixtures.",
        ],
    }

    output.mkdir(parents=True, exist_ok=True)
    write_json(output / "portal-exhaustive-audit-matrix.json", matrix)
    write_json(output / "portal-exhaustive-audit-findings.json", findings)
    (output / "portal-exhaustive-audit-summary.md").write_text(markdown_summary(matrix), encoding="utf-8")

    summary = {
        "exact_sha": exact_sha,
        "global_verdict": matrix["global_verdict"],
        "route_record_count": len(route_records),
        "capability_record_count": len(capability_rows),
        "finding_count": len(findings),
        "infrastructure_error_count": len(infrastructure_errors),
        "module_verdicts": {
            row["module"]: row["final_classification"] for row in module_summaries
        },
    }
    print(json.dumps(summary, indent=2))
    return 1 if infrastructure_errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
