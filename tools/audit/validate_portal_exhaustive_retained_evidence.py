#!/usr/bin/env python3
"""Fail-closed validation for repository-retained portal audit provenance."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any

SHA_RE = re.compile(r"^[0-9a-f]{40}$")
DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
EVIDENCE_RELATIVE = Path("docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit")


def _collect_source_shas(value: Any, found: set[str]) -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if key in {"exact_sha", "source_sha"} and isinstance(child, str):
                found.add(child)
            _collect_source_shas(child, found)
    elif isinstance(value, list):
        for child in value:
            _collect_source_shas(child, found)


def _valid_positive_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value > 0


def validate_retained_evidence(manifest: dict[str, Any], evidence_documents: list[tuple[str, Any]]) -> dict[str, Any]:
    errors: list[str] = []

    if manifest.get("schema_version") != 3:
        errors.append("retained portal audit manifest schema_version must be 3")

    provenance = manifest.get("provenance")
    if not isinstance(provenance, dict):
        errors.append("retained portal audit manifest must define provenance")
        provenance = {}

    stages: dict[str, dict[str, Any]] = {}
    for stage_id in ("base_generation", "strict_source", "final_pr_head", "merge"):
        stage = provenance.get(stage_id)
        if not isinstance(stage, dict):
            errors.append(f"provenance stage {stage_id} is missing")
            stage = {}
        stages[stage_id] = stage
        sha = stage.get("sha")
        if not isinstance(sha, str) or SHA_RE.fullmatch(sha) is None:
            errors.append(f"provenance stage {stage_id} must record a full SHA")

    stage_shas = [stage.get("sha") for stage in stages.values() if isinstance(stage.get("sha"), str)]
    if len(stage_shas) == 4 and len(set(stage_shas)) != 4:
        errors.append("base generation, strict source, final PR head and merge SHAs must remain distinct")

    strict_stage = stages["strict_source"]
    final_stage = stages["final_pr_head"]
    merge_stage = stages["merge"]

    if manifest.get("source_sha") != strict_stage.get("sha"):
        errors.append("manifest source_sha must equal provenance.strict_source.sha")

    workflow = manifest.get("workflow")
    if not isinstance(workflow, dict):
        errors.append("retained portal audit manifest must define workflow")
        workflow = {}
    for key in ("run_id", "artifact_id"):
        if not _valid_positive_int(workflow.get(key)):
            errors.append(f"workflow {key} must be a positive integer")
        if workflow.get(key) != strict_stage.get(key):
            errors.append(f"workflow {key} must equal provenance.strict_source.{key}")
    if not isinstance(workflow.get("artifact_digest"), str) or DIGEST_RE.fullmatch(workflow["artifact_digest"]) is None:
        errors.append("workflow artifact_digest must be a sha256 digest")
    if workflow.get("artifact_digest") != strict_stage.get("artifact_digest"):
        errors.append("workflow artifact_digest must equal provenance.strict_source.artifact_digest")
    if workflow.get("conclusion") != "success" or strict_stage.get("conclusion") != "success":
        errors.append("strict source workflow must record success")

    for stage_id, stage in (("strict_source", strict_stage), ("final_pr_head", final_stage)):
        if not _valid_positive_int(stage.get("run_id")):
            errors.append(f"provenance stage {stage_id} run_id must be a positive integer")
        if not _valid_positive_int(stage.get("artifact_id")):
            errors.append(f"provenance stage {stage_id} artifact_id must be a positive integer")
        digest = stage.get("artifact_digest")
        if not isinstance(digest, str) or DIGEST_RE.fullmatch(digest) is None:
            errors.append(f"provenance stage {stage_id} artifact_digest must be a sha256 digest")
        if stage.get("conclusion") != "success":
            errors.append(f"provenance stage {stage_id} must record success")

    if not _valid_positive_int(merge_stage.get("pr_number")):
        errors.append("provenance stage merge pr_number must be a positive integer")

    allowed = provenance.get("allowed_embedded_source_shas")
    expected_allowed = {stages["base_generation"].get("sha"), strict_stage.get("sha")}
    if not isinstance(allowed, list) or not allowed or any(not isinstance(value, str) or SHA_RE.fullmatch(value) is None for value in allowed):
        errors.append("provenance allowed_embedded_source_shas must be a non-empty list of full SHAs")
        allowed_set: set[str] = set()
    else:
        allowed_set = set(allowed)
        if len(allowed_set) != len(allowed):
            errors.append("provenance allowed_embedded_source_shas must not contain duplicates")
        if allowed_set != expected_allowed:
            errors.append("allowed embedded source SHAs must exactly declare base_generation and strict_source")

    observed: set[str] = set()
    for _name, document in evidence_documents:
        _collect_source_shas(document, observed)

    invalid_observed = sorted(value for value in observed if SHA_RE.fullmatch(value) is None)
    for value in invalid_observed:
        errors.append(f"retained evidence contains malformed embedded source SHA: {value}")

    unexplained = sorted(observed - allowed_set)
    for value in unexplained:
        errors.append(f"retained evidence contains unexplained embedded source SHA: {value}")

    missing_declared = sorted(allowed_set - observed)
    for value in missing_declared:
        errors.append(f"declared retained evidence source SHA is not present in evidence: {value}")

    final_reference = provenance.get("durable_final_exact_head_reference")
    if not isinstance(final_reference, dict):
        errors.append("provenance must define durable_final_exact_head_reference")
    else:
        for key in ("sha", "run_id", "artifact_id", "artifact_digest"):
            if final_reference.get(key) != final_stage.get(key):
                errors.append(f"durable final exact-head reference {key} must equal provenance.final_pr_head.{key}")

    return {
        "schema_version": manifest.get("schema_version"),
        "retained_document_count": len(evidence_documents),
        "observed_embedded_source_shas": sorted(observed),
        "declared_embedded_source_shas": sorted(allowed_set),
        "final_pr_head_sha": final_stage.get("sha"),
        "merge_sha": merge_stage.get("sha"),
        "errors": errors,
    }


def load_repository_inputs(repo_root: Path) -> tuple[dict[str, Any], list[tuple[str, Any]]]:
    evidence_root = repo_root / EVIDENCE_RELATIVE
    manifest_path = evidence_root / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    documents: list[tuple[str, Any]] = []
    for path in sorted(evidence_root.glob("*.json")):
        if path.name == "manifest.json":
            continue
        documents.append((path.name, json.loads(path.read_text(encoding="utf-8"))))
    return manifest, documents


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    try:
        manifest, documents = load_repository_inputs(repo_root)
        report = validate_retained_evidence(manifest, documents)
    except Exception as exc:  # fail closed on malformed/missing retained evidence
        report = {
            "schema_version": None,
            "retained_document_count": 0,
            "observed_embedded_source_shas": [],
            "declared_embedded_source_shas": [],
            "final_pr_head_sha": None,
            "merge_sha": None,
            "errors": [f"Cannot validate retained portal audit evidence: {exc}"],
        }
    print(json.dumps(report, indent=2, sort_keys=True))
    return 1 if report["errors"] else 0


if __name__ == "__main__":
    sys.exit(main())
