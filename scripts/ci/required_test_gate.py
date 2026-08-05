#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class GateDecision:
    passed: bool
    outcome: str
    runtime_tests_required: bool | None
    message: str


def _parse_required(value: str) -> bool:
    normalized = value.strip().casefold()
    if normalized == "true":
        return True
    if normalized == "false":
        return False
    raise ValueError(f"unsupported ci-required value: {value!r}")


def evaluate_gate(
    *,
    classification_result: str,
    ci_required: str,
    runtime_tests_result: str,
) -> GateDecision:
    classification = classification_result.strip().casefold()
    runtime_result = runtime_tests_result.strip().casefold()

    if classification != "success":
        return GateDecision(
            passed=False,
            outcome="failed",
            runtime_tests_required=None,
            message=(
                "Change classification did not succeed; the required test gate is "
                f"failing closed (classification={classification or 'missing'})."
            ),
        )

    try:
        required = _parse_required(ci_required)
    except ValueError as exc:
        return GateDecision(
            passed=False,
            outcome="failed",
            runtime_tests_required=None,
            message=f"Change classification produced an invalid gate decision: {exc}",
        )

    if required:
        if runtime_result != "success":
            return GateDecision(
                passed=False,
                outcome="failed",
                runtime_tests_required=True,
                message=(
                    "Runtime tests were required but did not succeed "
                    f"(runtime-tests={runtime_result or 'missing'})."
                ),
            )
        return GateDecision(
            passed=True,
            outcome="runtime-tests-passed",
            runtime_tests_required=True,
            message="Runtime tests were required and completed successfully.",
        )

    if runtime_result != "skipped":
        return GateDecision(
            passed=False,
            outcome="failed",
            runtime_tests_required=False,
            message=(
                "Runtime tests were classified NOT_APPLICABLE but the conditional job "
                f"did not report skipped (runtime-tests={runtime_result or 'missing'})."
            ),
        )

    return GateDecision(
        passed=True,
        outcome="not-applicable",
        runtime_tests_required=False,
        message=(
            "Runtime tests are NOT_APPLICABLE for this classified change set; "
            "the required test context passed without executing the runtime suite."
        ),
    )


def write_summary(path: Path, decision: GateDecision) -> None:
    required = (
        "UNKNOWN"
        if decision.runtime_tests_required is None
        else "YES" if decision.runtime_tests_required else "NO"
    )
    lines = [
        "### Required test gate",
        "",
        f"- result: `{'PASS' if decision.passed else 'FAIL'}`",
        f"- outcome: `{decision.outcome}`",
        f"- runtime tests required: `{required}`",
        f"- evidence: {decision.message}",
        "",
    ]
    with path.open("a", encoding="utf-8") as handle:
        handle.write("\n".join(lines))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Evaluate the stable protected test context from CI routing evidence."
    )
    parser.add_argument("--classification-result", required=True)
    parser.add_argument("--ci-required", required=True)
    parser.add_argument("--runtime-tests-result", required=True)
    parser.add_argument("--summary", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    decision = evaluate_gate(
        classification_result=args.classification_result,
        ci_required=args.ci_required,
        runtime_tests_result=args.runtime_tests_result,
    )
    if args.summary:
        write_summary(args.summary, decision)
    print(decision.message)
    return 0 if decision.passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
