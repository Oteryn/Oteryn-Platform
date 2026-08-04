#!/usr/bin/env python3
"""Fail-closed compiler for Oteryn deep-system validation evidence."""

from __future__ import annotations

import argparse
import json
import re
import sys
import xml.etree.ElementTree as ET
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
REQUIRED_LANES = {
    "python-validator-tests",
    "composer-validate",
    "composer-audit",
    "php-format",
    "php-analysis",
    "php-tests",
    "php-game-auth-concurrency",
    "npm-audit",
    "coverage-contract-strict",
    "content-scale-contract",
    "browser-full-chromium",
    "account-lifecycle",
    "community-data",
    "downloads",
    "downloads-portability",
    "portability",
    "responsive",
    "resilience",
    "accessibility",
    "soak",
    "visual-exploratory",
}
REQUIRED_JUNIT_LANES = {
    "php-tests",
    "php-game-auth-concurrency",
    "browser-full-chromium",
    "account-lifecycle",
    "community-data",
    "content-scale-contract",
    "downloads",
    "downloads-portability",
    "portability",
    "responsive",
    "resilience",
    "accessibility",
    "soak",
}
EXPECTED_PROJECTS_BY_LANE = {
    "browser-full-chromium": {"chromium-primary"},
    "account-lifecycle": {"chromium-primary"},
    "community-data": {
        "community-data-chromium-desktop",
        "community-data-chromium-tablet",
        "community-data-chromium-mobile",
    },
    "content-scale-contract": {
        "content-scale-chromium-desktop",
        "content-scale-chromium-tablet",
        "content-scale-chromium-mobile",
    },
    "downloads": {"chromium-primary"},
    "downloads-portability": {
        "downloads-portability-firefox",
        "downloads-portability-webkit",
    },
    "portability": {
        "portability-chromium",
        "portability-firefox",
        "portability-webkit",
    },
    "responsive": {
        "responsive-desktop",
        "responsive-tablet",
        "responsive-mobile",
    },
    "resilience": {
        "resilience-chromium",
        "error-states-chromium-desktop",
        "error-states-chromium-tablet",
        "error-states-chromium-mobile",
    },
    "accessibility": {"accessibility-chromium"},
    "soak": {"soak-chromium"},
}
VISUAL_PROBLEM_KEYS = {
    "statusMismatch",
    "horizontalOverflow",
    "unlabeledControls",
    "lowContrast",
    "focusNotObserved",
    "rawTechnicalMessages",
    "browserErrors",
}
ALLOWED_STATUSES = {"PASS", "FAIL", "BLOCKED", "NOT_APPLICABLE"}
PROJECT_PREFIX = re.compile(r"^\[([^\]]+)\]\s")


class ValidationError(ValueError):
    """Raised when evidence is incomplete or internally inconsistent."""


@dataclass(frozen=True)
class JUnitSummary:
    files: int = 0
    tests: int = 0
    failures: int = 0
    errors: int = 0
    skipped: int = 0
    projects: tuple[str, ...] = ()

    def plus(self, other: "JUnitSummary") -> "JUnitSummary":
        return JUnitSummary(
            files=self.files + other.files,
            tests=self.tests + other.tests,
            failures=self.failures + other.failures,
            errors=self.errors + other.errors,
            skipped=self.skipped + other.skipped,
            projects=tuple(sorted(set(self.projects) | set(other.projects))),
        )


def _int_attr(node: ET.Element, name: str) -> int:
    raw = node.attrib.get(name, "0")
    try:
        return int(float(raw))
    except ValueError as exc:
        raise ValidationError(f"invalid JUnit {name} value: {raw!r}") from exc


def parse_junit(path: Path) -> JUnitSummary:
    if not path.is_file():
        raise ValidationError(f"JUnit file is missing: {path}")
    try:
        root = ET.parse(path).getroot()
    except ET.ParseError as exc:
        raise ValidationError(f"invalid JUnit XML {path}: {exc}") from exc

    if root.tag == "testsuite":
        suites = [root]
    elif root.tag == "testsuites":
        suites = list(root.findall("testsuite")) or [root]
    else:
        raise ValidationError(f"unsupported JUnit root {root.tag!r} in {path}")

    total = JUnitSummary(files=1)
    for suite in suites:
        total = total.plus(
            JUnitSummary(
                tests=_int_attr(suite, "tests"),
                failures=_int_attr(suite, "failures"),
                errors=_int_attr(suite, "errors"),
                skipped=_int_attr(suite, "skipped"),
            )
        )

    testcases = root.findall(".//testcase")
    if total.tests != len(testcases):
        raise ValidationError(
            f"JUnit file {path} declared {total.tests} tests but contains "
            f"{len(testcases)} testcase elements"
        )

    projects = set()
    for testcase in testcases:
        match = PROJECT_PREFIX.match(testcase.attrib.get("name", ""))
        if match:
            projects.add(match.group(1))

    return JUnitSummary(
        files=total.files,
        tests=total.tests,
        failures=total.failures,
        errors=total.errors,
        skipped=total.skipped,
        projects=tuple(sorted(projects)),
    )


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ValidationError(f"evidence file is missing: {path}") from exc
    except json.JSONDecodeError as exc:
        raise ValidationError(f"invalid JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise ValidationError(f"top-level JSON must be an object: {path}")
    return value


def _resolve_evidence_path(base_dir: Path, relative: Any, lane: str) -> Path:
    if not isinstance(relative, str) or not relative:
        raise ValidationError(f"lane {lane} has invalid JUnit path")
    candidate = Path(relative)
    if candidate.is_absolute():
        raise ValidationError(f"lane {lane} has invalid JUnit path")
    resolved_base = base_dir.resolve()
    resolved = (resolved_base / candidate).resolve()
    if not resolved.is_relative_to(resolved_base):
        raise ValidationError(f"lane {lane} JUnit path escapes the evidence base directory")
    return resolved


def validate_visual_evidence(base_dir: Path, exact_sha: str) -> dict[str, Any]:
    evidence = load_json(base_dir / "artifacts/deep/visual/visual-acceptance-results.json")
    if evidence.get("validationSha") != exact_sha:
        raise ValidationError("visual evidence SHA does not match exact validation SHA")
    if evidence.get("classification") != "VISUAL_UX_EVIDENCE_COLLECTED":
        raise ValidationError("visual evidence has an unexpected classification")
    screenshot_count = evidence.get("screenshotCount")
    if not isinstance(screenshot_count, int) or screenshot_count <= 0:
        raise ValidationError("visual evidence contains no screenshots")
    problematic = evidence.get("problematic")
    if not isinstance(problematic, dict):
        raise ValidationError("visual evidence is missing the problematic summary")
    missing = sorted(VISUAL_PROBLEM_KEYS - set(problematic))
    if missing:
        raise ValidationError("visual evidence is missing problem keys: " + ", ".join(missing))
    findings: dict[str, int] = {}
    for key in sorted(VISUAL_PROBLEM_KEYS):
        records = problematic.get(key)
        if not isinstance(records, list):
            raise ValidationError(f"visual problem category {key} must be an array")
        findings[key] = len(records)
    active = {key: count for key, count in findings.items() if count}
    if active:
        rendered = ", ".join(f"{key}={count}" for key, count in active.items())
        raise ValidationError(f"visual evidence contains blocking findings: {rendered}")
    return {"screenshot_count": screenshot_count, "problem_counts": findings}


def validate_soak_evidence(base_dir: Path, exact_sha: str) -> dict[str, Any]:
    evidence = load_json(base_dir / "artifacts/deep/soak-runtime-metrics.json")
    if evidence.get("exact_tested_sha") != exact_sha:
        raise ValidationError("soak evidence SHA does not match exact validation SHA")
    target = evidence.get("target_duration_seconds")
    measured = evidence.get("measured_duration_seconds")
    if not isinstance(target, int) or target < 300:
        raise ValidationError("soak target must be at least 300 seconds")
    if not isinstance(measured, int) or measured < target:
        raise ValidationError(
            f"soak duration is incomplete: measured={measured!r} target={target!r}"
        )
    for key in (
        "server_rss_start_kb",
        "server_rss_end_kb",
        "server_rss_max_kb",
        "redis_keys_before",
        "redis_keys_after",
    ):
        value = evidence.get(key)
        if not isinstance(value, int) or value < 0:
            raise ValidationError(f"soak metric {key} must be a non-negative integer")
    try:
        samples = [
            line
            for line in (base_dir / "artifacts/deep/soak-rss-samples.tsv")
            .read_text(encoding="utf-8")
            .splitlines()
            if line
        ]
    except FileNotFoundError as exc:
        raise ValidationError("soak RSS samples are missing") from exc
    if len(samples) < 2:
        raise ValidationError("soak RSS evidence contains fewer than two samples")
    return {**evidence, "rss_sample_count": len(samples)}


def _validate_lane_projects(name: str, lane: dict[str, Any], summary: JUnitSummary) -> None:
    expected = EXPECTED_PROJECTS_BY_LANE.get(name)
    declared = lane.get("projects")
    if expected is None:
        if declared is not None:
            raise ValidationError(f"lane {name} declares unexpected browser projects")
        return

    if declared is not None:
        if (
            not isinstance(declared, list)
            or not declared
            or any(not isinstance(project, str) or not project for project in declared)
            or len(declared) != len(set(declared))
        ):
            raise ValidationError(f"lane {name} projects must be unique non-empty strings")
        declared_set = set(declared)
        if declared_set != expected:
            missing = sorted(expected - declared_set)
            unexpected = sorted(declared_set - expected)
            raise ValidationError(
                f"lane {name} project contract mismatch: missing={missing} unexpected={unexpected}"
            )

    executed = set(summary.projects)
    if executed != expected:
        missing = sorted(expected - executed)
        unexpected = sorted(executed - expected)
        raise ValidationError(
            f"lane {name} JUnit project mismatch: missing={missing} unexpected={unexpected}"
        )


def validate_contract(
    contract: dict[str, Any], exact_sha: str, base_dir: Path
) -> dict[str, Any]:
    if contract.get("schema_version") != SCHEMA_VERSION:
        raise ValidationError("unsupported lane contract schema_version")
    if contract.get("exact_sha") != exact_sha:
        raise ValidationError(
            f"lane contract SHA {contract.get('exact_sha')!r} does not match {exact_sha!r}"
        )
    if contract.get("retries") != 0:
        raise ValidationError("validation retries must be exactly zero")

    lanes = contract.get("lanes")
    if not isinstance(lanes, list) or not lanes:
        raise ValidationError("lane contract must contain a non-empty lanes array")

    by_name: dict[str, dict[str, Any]] = {}
    compiled: list[dict[str, Any]] = []
    global_junit = JUnitSummary()
    junit_owners: dict[Path, str] = {}
    blockers: list[dict[str, Any]] = []

    for lane in lanes:
        if not isinstance(lane, dict):
            raise ValidationError("each lane must be an object")
        name = lane.get("name")
        if not isinstance(name, str) or not name:
            raise ValidationError("each lane requires a non-empty name")
        if name in by_name:
            raise ValidationError(f"duplicate lane name: {name}")
        by_name[name] = lane

        status = lane.get("status")
        if status not in ALLOWED_STATUSES:
            raise ValidationError(f"lane {name} has invalid status {status!r}")
        if status == "FAIL":
            raise ValidationError(f"lane {name} reported FAIL")

        required = lane.get("required", False)
        if not isinstance(required, bool):
            raise ValidationError(f"lane {name} required must be a boolean")
        if required and status != "PASS":
            raise ValidationError(f"required lane {name} is not PASS: {status}")

        kind = lane.get("kind")
        if kind != "external" and status != "PASS":
            raise ValidationError(f"non-external lane {name} cannot report status {status}")
        if name in REQUIRED_JUNIT_LANES and kind != "junit":
            raise ValidationError(f"required JUnit lane {name} has unexpected kind {kind!r}")
        if name in REQUIRED_LANES - REQUIRED_JUNIT_LANES and kind != "command":
            raise ValidationError(f"required command lane {name} has unexpected kind {kind!r}")

        lane_summary = JUnitSummary()
        if kind == "command":
            if lane.get("exit_code") != 0:
                raise ValidationError(f"PASS command lane {name} must have exit_code 0")
        elif kind == "junit":
            junit_files = lane.get("junit_files")
            if not isinstance(junit_files, list) or not junit_files:
                raise ValidationError(f"JUnit lane {name} requires junit_files")
            for relative in junit_files:
                junit_path = _resolve_evidence_path(base_dir, relative, name)
                previous_owner = junit_owners.get(junit_path)
                if previous_owner is not None:
                    raise ValidationError(
                        f"JUnit evidence {relative!r} is reused by lanes {previous_owner} and {name}"
                    )
                junit_owners[junit_path] = name
                lane_summary = lane_summary.plus(parse_junit(junit_path))
            if lane_summary.tests <= 0:
                raise ValidationError(f"JUnit lane {name} executed zero tests")
            if lane_summary.failures or lane_summary.errors or lane_summary.skipped:
                raise ValidationError(
                    f"JUnit lane {name} is not clean: failures={lane_summary.failures} "
                    f"errors={lane_summary.errors} skipped={lane_summary.skipped}"
                )
            _validate_lane_projects(name, lane, lane_summary)
            global_junit = global_junit.plus(lane_summary)
        elif kind == "external":
            if status in {"BLOCKED", "NOT_APPLICABLE"}:
                reason = lane.get("reason")
                owner_issue = lane.get("owner_issue")
                if not isinstance(reason, str) or not reason.strip():
                    raise ValidationError(f"external lane {name} requires a non-empty reason")
                if (
                    not isinstance(owner_issue, int)
                    or isinstance(owner_issue, bool)
                    or owner_issue <= 0
                ):
                    raise ValidationError(f"external lane {name} requires a positive owner_issue")
                if status == "BLOCKED":
                    blockers.append(
                        {"name": name, "reason": reason, "owner_issue": owner_issue}
                    )
            elif status == "PASS":
                identity = lane.get("evidence_identity")
                if not isinstance(identity, str) or not identity.strip():
                    raise ValidationError(f"external PASS lane {name} requires evidence_identity")
        else:
            raise ValidationError(f"lane {name} has unsupported kind {kind!r}")

        compiled_lane = dict(lane)
        if kind == "junit":
            compiled_lane["junit_summary"] = asdict(lane_summary)
        compiled.append(compiled_lane)

    missing = sorted(REQUIRED_LANES - set(by_name))
    if missing:
        raise ValidationError(f"required lanes are missing: {', '.join(missing)}")
    optional_required = sorted(
        name for name in REQUIRED_LANES if by_name[name].get("required") is not True
    )
    if optional_required:
        raise ValidationError(
            "required lanes are not marked required: " + ", ".join(optional_required)
        )

    visual_summary = validate_visual_evidence(base_dir, exact_sha)
    soak_metrics = validate_soak_evidence(base_dir, exact_sha)
    nonclaims = contract.get("nonclaims")
    if (
        not isinstance(nonclaims, list)
        or not nonclaims
        or any(not isinstance(item, str) or not item.strip() for item in nonclaims)
    ):
        raise ValidationError("lane contract requires non-empty string nonclaims")

    return {
        "schema_version": SCHEMA_VERSION,
        "task_id": "OTERYN-20260803-deep-system-validation",
        "exact_sha": exact_sha,
        "global_verdict": (
            "DEEP_VALIDATION_PASS_WITH_EXTERNAL_BLOCKERS"
            if blockers
            else "DEEP_VALIDATION_PASS"
        ),
        "retries": 0,
        "required_lanes": sorted(REQUIRED_LANES),
        "lane_count": len(compiled),
        "junit_totals": asdict(global_junit),
        "visual_summary": visual_summary,
        "soak_metrics": soak_metrics,
        "external_blocker_count": len(blockers),
        "external_blockers": blockers,
        "lanes": compiled,
        "nonclaims": nonclaims,
    }


def render_markdown(manifest: dict[str, Any]) -> str:
    totals = manifest["junit_totals"]
    lines = [
        "# OTERYN deep system validation",
        "",
        f"- Exact tested SHA: `{manifest['exact_sha']}`",
        f"- Verdict: **{manifest['global_verdict']}**",
        f"- Validation lanes: {manifest['lane_count']}",
        f"- JUnit tests: {totals['tests']}",
        f"- Failures/errors/skips/retries: {totals['failures']}/{totals['errors']}/{totals['skipped']}/{manifest['retries']}",
        f"- Executed browser projects: {len(totals['projects'])}",
        f"- Visual screenshots: {manifest['visual_summary']['screenshot_count']}",
        f"- Soak duration: {manifest['soak_metrics']['measured_duration_seconds']} seconds",
        f"- External blockers: {manifest['external_blocker_count']}",
        "",
        "## Lanes",
        "",
        "| Lane | Kind | Status | Tests | Projects |",
        "|---|---|---:|---:|---|",
    ]
    for lane in manifest["lanes"]:
        summary = lane.get("junit_summary", {})
        tests = summary.get("tests", "—")
        projects = ", ".join(summary.get("projects", [])) or "—"
        lines.append(
            f"| `{lane['name']}` | {lane['kind']} | {lane['status']} | {tests} | {projects} |"
        )

    if manifest["external_blockers"]:
        lines.extend(["", "## External blockers", ""])
        for blocker in manifest["external_blockers"]:
            lines.append(
                f"- `{blocker['name']}` — {blocker['reason']} Owner: #{blocker['owner_issue']}."
            )

    lines.extend(["", "## Nonclaims", ""])
    lines.extend(f"- {item}" for item in manifest["nonclaims"])
    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--exact-sha", required=True)
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--base-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args(argv)

    try:
        manifest = validate_contract(load_json(args.contract), args.exact_sha, args.base_dir)
    except ValidationError as exc:
        print(json.dumps({"result": "FAIL", "error": str(exc)}, sort_keys=True))
        return 1

    args.output_dir.mkdir(parents=True, exist_ok=True)
    (args.output_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (args.output_dir / "report.md").write_text(render_markdown(manifest), encoding="utf-8")
    print(
        json.dumps(
            {
                "result": "PASS",
                "verdict": manifest["global_verdict"],
                "exact_sha": manifest["exact_sha"],
                "lanes": manifest["lane_count"],
                "tests": manifest["junit_totals"]["tests"],
                "projects": len(manifest["junit_totals"]["projects"]),
                "screenshots": manifest["visual_summary"]["screenshot_count"],
                "soak_seconds": manifest["soak_metrics"]["measured_duration_seconds"],
                "external_blockers": manifest["external_blocker_count"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
