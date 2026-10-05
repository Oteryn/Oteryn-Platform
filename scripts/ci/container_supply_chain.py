#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import re
from dataclasses import dataclass
from datetime import date, datetime
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_POLICY = ROOT / "docs/security/CONTAINER_SUPPLY_CHAIN_POLICY.json"
PINNED_IMAGE_RE = re.compile(r"^[^@\s]+:[^@\s]+@sha256:[0-9a-f]{64}$")
SHA40_RE = re.compile(r"^[0-9a-f]{40}$")
IMAGE_DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")


class SupplyChainError(RuntimeError):
    pass


@dataclass(frozen=True)
class ExternalReference:
    path: str
    kind: str
    reference: str


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SupplyChainError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(raw, dict):
        raise SupplyChainError(f"{path} must contain a JSON object")
    return raw


def dockerfile_references(path: Path) -> tuple[list[ExternalReference], str | None]:
    aliases: set[str] = set()
    references: list[ExternalReference] = []
    final_user: str | None = None
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        match = re.match(
            r"^FROM(?:\s+--platform=\S+)?\s+(\S+)(?:\s+AS\s+(\S+))?$",
            line,
            flags=re.IGNORECASE,
        )
        if match:
            image = match.group(1)
            alias = match.group(2)
            references.append(ExternalReference(str(path), "FROM", image))
            if alias:
                aliases.add(alias)
            continue
        match = re.search(r"--from=(\S+)", line)
        if match:
            source = match.group(1)
            if source not in aliases and not source.isdigit():
                references.append(ExternalReference(str(path), "COPY --from", source))
        match = re.match(r"^USER\s+(\S+)", line, flags=re.IGNORECASE)
        if match:
            final_user = match.group(1)
    return references, final_user


def validate_exception_registry(policy: dict[str, Any], root: Path = ROOT) -> None:
    vuln = policy.get("vulnerability_policy")
    if not isinstance(vuln, dict):
        raise SupplyChainError("policy.vulnerability_policy must be an object")
    exception_rel = vuln.get("exception_file")
    max_days = vuln.get("maximum_exception_days")
    if not isinstance(exception_rel, str) or not exception_rel:
        raise SupplyChainError("vulnerability_policy.exception_file is required")
    if not isinstance(max_days, int) or isinstance(max_days, bool) or max_days < 1:
        raise SupplyChainError("maximum_exception_days must be a positive integer")
    registry = load_json(root / exception_rel)
    exceptions = registry.get("exceptions")
    if not isinstance(exceptions, list):
        raise SupplyChainError("exception registry exceptions must be a list")
    today = date.today()
    for index, item in enumerate(exceptions):
        if not isinstance(item, dict):
            raise SupplyChainError(f"exception {index} must be an object")
        required = ("component", "vulnerability_id", "package_name", "expires_on", "evidence")
        missing = [name for name in required if not str(item.get(name, "")).strip()]
        if missing:
            raise SupplyChainError(f"exception {index} missing fields: {missing}")
        try:
            expires = date.fromisoformat(str(item["expires_on"]))
        except ValueError as exc:
            raise SupplyChainError(f"exception {index} has invalid expires_on") from exc
        if expires < today:
            raise SupplyChainError(
                f"exception {index} expired on {expires.isoformat()}: {item['vulnerability_id']}"
            )
        created_raw = item.get("created_on")
        if created_raw:
            try:
                created = date.fromisoformat(str(created_raw))
            except ValueError as exc:
                raise SupplyChainError(f"exception {index} has invalid created_on") from exc
            if (expires - created).days > max_days:
                raise SupplyChainError(
                    f"exception {index} exceeds maximum {max_days}-day lifetime"
                )


def validate_repository(policy_path: Path = DEFAULT_POLICY, root: Path = ROOT) -> dict[str, Any]:
    policy = load_json(policy_path)
    if policy.get("schema_version") != 1:
        raise SupplyChainError("unsupported policy schema_version")
    dockerfiles = policy.get("dockerfiles")
    if not isinstance(dockerfiles, list) or not dockerfiles:
        raise SupplyChainError("policy.dockerfiles must be a non-empty list")
    evidence: list[dict[str, Any]] = []
    for item in dockerfiles:
        if not isinstance(item, dict):
            raise SupplyChainError("dockerfile policy entry must be an object")
        rel = item.get("path")
        expected_user = item.get("expected_user")
        if not isinstance(rel, str) or not rel:
            raise SupplyChainError("dockerfile policy entry requires path")
        path = root / rel
        refs, final_user = dockerfile_references(path)
        if not refs:
            raise SupplyChainError(f"{rel} has no external image references")
        mutable = [ref.reference for ref in refs if not PINNED_IMAGE_RE.fullmatch(ref.reference)]
        if mutable:
            raise SupplyChainError(f"{rel} contains mutable external image references: {mutable}")
        if final_user != expected_user:
            raise SupplyChainError(
                f"{rel} final USER is {final_user!r}, expected {expected_user!r}"
            )
        evidence.append(
            {
                "component": item.get("component"),
                "path": rel,
                "final_user": final_user,
                "references": [ref.reference for ref in refs],
            }
        )
    validate_exception_registry(policy, root)
    return {"schema_version": 1, "dockerfiles": evidence}


def active_exceptions(
    policy: dict[str, Any], component: str, root: Path = ROOT, today: date | None = None
) -> list[dict[str, Any]]:
    vuln = policy["vulnerability_policy"]
    registry = load_json(root / vuln["exception_file"])
    current = today or date.today()
    result = []
    for item in registry.get("exceptions", []):
        if item.get("component") != component:
            continue
        try:
            expires = date.fromisoformat(str(item.get("expires_on", "")))
        except ValueError:
            continue
        if expires >= current:
            result.append(item)
    return result


def evaluate_trivy(
    report_path: Path,
    policy_path: Path,
    component: str,
    root: Path = ROOT,
    today: date | None = None,
) -> dict[str, Any]:
    policy = load_json(policy_path)
    validate_exception_registry(policy, root)
    vuln_policy = policy["vulnerability_policy"]
    severities = {str(item).upper() for item in vuln_policy.get("severities", [])}
    if not severities:
        raise SupplyChainError("vulnerability_policy.severities must be non-empty")
    ignore_unfixed = bool(vuln_policy.get("ignore_unfixed", False))
    exceptions = active_exceptions(policy, component, root, today)
    report = load_json(report_path)
    blocked: list[dict[str, str]] = []
    excepted: list[dict[str, str]] = []
    for result in report.get("Results") or []:
        if not isinstance(result, dict):
            continue
        for finding in result.get("Vulnerabilities") or []:
            if not isinstance(finding, dict):
                continue
            severity = str(finding.get("Severity", "")).upper()
            if severity not in severities:
                continue
            fixed = str(finding.get("FixedVersion", "")).strip()
            if ignore_unfixed and not fixed:
                continue
            vuln_id = str(finding.get("VulnerabilityID", "")).strip()
            package = str(finding.get("PkgName", "")).strip()
            match = next(
                (
                    item
                    for item in exceptions
                    if item.get("vulnerability_id") == vuln_id
                    and item.get("package_name") == package
                ),
                None,
            )
            record = {
                "vulnerability_id": vuln_id,
                "package_name": package,
                "severity": severity,
                "fixed_version": fixed,
            }
            if match:
                record["exception_expires_on"] = str(match["expires_on"])
                excepted.append(record)
            else:
                blocked.append(record)
    if blocked:
        raise SupplyChainError(
            "container vulnerability policy blocked findings: "
            + json.dumps(blocked, sort_keys=True)
        )
    return {
        "component": component,
        "blocked": blocked,
        "excepted": excepted,
        "status": "PASS",
    }


def provenance_manifest(
    *,
    source_sha: str,
    dockerfile: Path,
    dockerignore: Path,
    image: str,
    image_digest: str,
    sbom: Path,
    trivy: Path,
    policy: Path,
) -> dict[str, Any]:
    if SHA40_RE.fullmatch(source_sha) is None:
        raise SupplyChainError("source SHA must be an exact lowercase 40-character commit SHA")
    if IMAGE_DIGEST_RE.fullmatch(image_digest) is None:
        raise SupplyChainError("image digest must be sha256:<64 lowercase hex>")
    refs, _ = dockerfile_references(dockerfile)
    manifest = {
        "schema": "oteryn-container-provenance-v1",
        "source_sha": source_sha,
        "image": image,
        "image_digest": image_digest,
        "image_ref": f"{image}@{image_digest}",
        "dockerfile": {
            "path": str(dockerfile),
            "sha256": sha256_file(dockerfile),
            "external_images": [item.reference for item in refs],
        },
        "dockerignore": {
            "path": str(dockerignore),
            "sha256": sha256_file(dockerignore),
        },
        "sbom": {"path": str(sbom), "sha256": sha256_file(sbom)},
        "vulnerability_report": {"path": str(trivy), "sha256": sha256_file(trivy)},
        "policy": {"path": str(policy), "sha256": sha256_file(policy)},
    }
    canonical = json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode()
    manifest["provenance_digest"] = "sha256:" + hashlib.sha256(canonical).hexdigest()
    return manifest


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    validate = sub.add_parser("validate-repository")
    validate.add_argument("--policy", type=Path, default=DEFAULT_POLICY)

    scan = sub.add_parser("evaluate-trivy")
    scan.add_argument("--report", type=Path, required=True)
    scan.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    scan.add_argument("--component", required=True)
    scan.add_argument("--json-output", type=Path)

    provenance = sub.add_parser("provenance")
    provenance.add_argument("--source-sha", required=True)
    provenance.add_argument("--dockerfile", type=Path, required=True)
    provenance.add_argument("--dockerignore", type=Path, required=True)
    provenance.add_argument("--image", required=True)
    provenance.add_argument("--image-digest", required=True)
    provenance.add_argument("--sbom", type=Path, required=True)
    provenance.add_argument("--trivy", type=Path, required=True)
    provenance.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    provenance.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.command == "validate-repository":
            result = validate_repository(args.policy)
        elif args.command == "evaluate-trivy":
            result = evaluate_trivy(args.report, args.policy, args.component)
            if args.json_output:
                args.json_output.parent.mkdir(parents=True, exist_ok=True)
                args.json_output.write_text(
                    json.dumps(result, indent=2, sort_keys=True) + "\n",
                    encoding="utf-8",
                )
        else:
            result = provenance_manifest(
                source_sha=args.source_sha,
                dockerfile=args.dockerfile,
                dockerignore=args.dockerignore,
                image=args.image,
                image_digest=args.image_digest,
                sbom=args.sbom,
                trivy=args.trivy,
                policy=args.policy,
            )
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(
                json.dumps(result, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    except SupplyChainError as exc:
        print(f"ERROR: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
