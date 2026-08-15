#!/usr/bin/env python3
"""Evaluate a PHPUnit Clover report against the repository coverage policy."""

from __future__ import annotations

import argparse
import json
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any


class CoveragePolicyError(RuntimeError):
    pass


def _load_policy(path: Path) -> dict[str, Any]:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise CoveragePolicyError(f"missing coverage policy: {path}") from exc
    except json.JSONDecodeError as exc:
        raise CoveragePolicyError(
            f"invalid coverage policy JSON at line {exc.lineno}: {exc.msg}"
        ) from exc

    if not isinstance(raw, dict) or raw.get("schema_version") != 1:
        raise CoveragePolicyError("coverage policy must use schema_version 1")
    mode = raw.get("mode")
    if mode not in {"report_only", "enforce"}:
        raise CoveragePolicyError("coverage policy mode must be report_only or enforce")
    minimum = raw.get("minimum_statement_percent")
    if mode == "enforce":
        if not isinstance(minimum, (int, float)) or isinstance(minimum, bool):
            raise CoveragePolicyError(
                "enforce mode requires numeric minimum_statement_percent"
            )
        if minimum < 0 or minimum > 100:
            raise CoveragePolicyError(
                "minimum_statement_percent must be between 0 and 100"
            )
    elif minimum is not None and (
        not isinstance(minimum, (int, float))
        or isinstance(minimum, bool)
        or minimum < 0
        or minimum > 100
    ):
        raise CoveragePolicyError(
            "report_only minimum_statement_percent must be null or 0..100"
        )
    return raw


def _metric_percent(covered: int, total: int) -> float | None:
    if total <= 0:
        return None
    return round((covered / total) * 100.0, 2)


def parse_clover(path: Path) -> dict[str, object]:
    try:
        root = ET.parse(path).getroot()
    except FileNotFoundError as exc:
        raise CoveragePolicyError(f"missing Clover report: {path}") from exc
    except ET.ParseError as exc:
        raise CoveragePolicyError(f"invalid Clover XML: {exc}") from exc

    project = root.find("project")
    if project is None:
        raise CoveragePolicyError("Clover report is missing project element")
    metrics = project.find("metrics")
    if metrics is None:
        raise CoveragePolicyError("Clover report is missing project metrics")

    def integer(name: str) -> int:
        value = metrics.get(name)
        if value is None:
            raise CoveragePolicyError(f"Clover project metrics missing {name}")
        try:
            parsed = int(value)
        except ValueError as exc:
            raise CoveragePolicyError(
                f"Clover project metric {name} is not an integer"
            ) from exc
        if parsed < 0:
            raise CoveragePolicyError(
                f"Clover project metric {name} must not be negative"
            )
        return parsed

    statements = integer("statements")
    covered_statements = integer("coveredstatements")
    methods = integer("methods")
    covered_methods = integer("coveredmethods")

    if covered_statements > statements:
        raise CoveragePolicyError("covered statements exceed total statements")
    if covered_methods > methods:
        raise CoveragePolicyError("covered methods exceed total methods")

    return {
        "statements": statements,
        "covered_statements": covered_statements,
        "statement_percent": _metric_percent(covered_statements, statements),
        "methods": methods,
        "covered_methods": covered_methods,
        "method_percent": _metric_percent(covered_methods, methods),
    }


def evaluate(policy: dict[str, Any], metrics: dict[str, object]) -> dict[str, object]:
    statement_percent = metrics["statement_percent"]
    if statement_percent is None:
        raise CoveragePolicyError(
            "coverage report has no executable statements in the configured source scope"
        )

    mode = policy["mode"]
    minimum = policy.get("minimum_statement_percent")
    passed = True
    reason = "report-only baseline collection"
    if mode == "enforce":
        assert isinstance(minimum, (int, float))
        passed = float(statement_percent) >= float(minimum)
        reason = (
            "statement coverage meets enforced floor"
            if passed
            else "statement coverage is below enforced floor"
        )

    return {
        "mode": mode,
        "scope": policy.get("scope", "app/**"),
        "statement_percent": statement_percent,
        "method_percent": metrics["method_percent"],
        "statements": metrics["statements"],
        "covered_statements": metrics["covered_statements"],
        "methods": metrics["methods"],
        "covered_methods": metrics["covered_methods"],
        "minimum_statement_percent": minimum,
        "passed": passed,
        "reason": reason,
    }


def write_summary(path: Path, result: dict[str, object]) -> None:
    lines = [
        "### PHP code coverage",
        "",
        f"- mode: `{result['mode']}`",
        f"- scope: `{result['scope']}`",
        f"- statement coverage: `{result['statement_percent']}%`",
        f"- method coverage: `{result['method_percent']}%`",
        f"- enforced floor: `{result['minimum_statement_percent']}`",
        f"- result: `{'PASS' if result['passed'] else 'FAIL'}`",
        f"- policy: {result['reason']}",
        "",
        "Coverage is supporting evidence; it does not replace risk-specific integration, security, concurrency, contract or browser proof.",
        "",
    ]
    with path.open("a", encoding="utf-8") as handle:
        handle.write("\n".join(lines))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--clover", type=Path)
    parser.add_argument("--validate-policy", action="store_true")
    parser.add_argument(
        "--policy",
        type=Path,
        default=Path("docs/agents/CI_COVERAGE_POLICY.json"),
    )
    parser.add_argument("--json-output", type=Path)
    parser.add_argument("--summary", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        policy = _load_policy(args.policy)
        if args.validate_policy:
            print(
                json.dumps(
                    {
                        "schema_version": policy["schema_version"],
                        "mode": policy["mode"],
                        "minimum_statement_percent": policy.get(
                            "minimum_statement_percent"
                        ),
                        "scope": policy.get("scope", "app/**"),
                    },
                    sort_keys=True,
                )
            )
            return 0
        if args.clover is None:
            raise CoveragePolicyError(
                "--clover is required unless --validate-policy is used"
            )
        metrics = parse_clover(args.clover)
        result = evaluate(policy, metrics)
    except CoveragePolicyError as exc:
        print(f"coverage policy error: {exc}")
        return 2

    rendered = json.dumps(result, indent=2, sort_keys=True)
    print(rendered)
    if args.json_output:
        args.json_output.parent.mkdir(parents=True, exist_ok=True)
        args.json_output.write_text(rendered + "\n", encoding="utf-8")
    if args.summary:
        write_summary(args.summary, result)
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
