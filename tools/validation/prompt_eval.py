#!/usr/bin/env python3
"""Deterministic regression evaluator for durable agent prompt/policy contracts.

This validator deliberately does not execute a language model. It verifies that the
repository retains the textual behavioural invariants declared by a machine-readable
scenario suite. Material prompt behaviour changes still require repeated model/runtime
trials when nondeterminism matters, per PROMPT_EVAL_STANDARD.md.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


DEFAULT_SUITE = Path("docs/agents/evals/prompt-contract-v1.json")
REQUIRED_CATEGORIES = {
    "normal_success",
    "boundary_refusal",
    "positive_tool_use",
    "negative_tool_use",
    "stale_conflicting_state",
    "ambiguous_live_state",
    "autonomous_continuation",
    "authority_stop",
    "prompt_injection",
    "missing_vertical_slice",
    "closeout",
}


class PromptEvalError(RuntimeError):
    """Raised when an eval suite or source violates the deterministic contract."""


def _read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise PromptEvalError(f"missing prompt eval suite: {path.as_posix()}") from exc
    except json.JSONDecodeError as exc:
        raise PromptEvalError(
            f"invalid JSON in {path.as_posix()}: line {exc.lineno} column {exc.colno}: {exc.msg}"
        ) from exc


def _require_non_empty_string(value: object, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise PromptEvalError(f"{label} must be a non-empty string")
    return value


def _require_string_list(value: object, label: str, *, allow_empty: bool = False) -> list[str]:
    if not isinstance(value, list) or (not value and not allow_empty):
        raise PromptEvalError(f"{label} must be a {'possibly empty ' if allow_empty else 'non-empty '}string array")
    if any(not isinstance(item, str) or not item for item in value):
        raise PromptEvalError(f"{label} must contain only non-empty strings")
    return list(value)


def _safe_source(root: Path, relative: str, case_id: str) -> Path:
    source = Path(relative)
    if source.is_absolute() or ".." in source.parts:
        raise PromptEvalError(f"{case_id}: source must be repository-relative: {relative}")
    resolved_root = root.resolve()
    resolved = (resolved_root / source).resolve()
    try:
        resolved.relative_to(resolved_root)
    except ValueError as exc:
        raise PromptEvalError(f"{case_id}: source escapes repository root: {relative}") from exc
    if not resolved.is_file():
        raise PromptEvalError(f"{case_id}: source does not exist: {relative}")
    return resolved


def validate_suite(root: Path, suite_path: Path) -> dict[str, object]:
    root = root.resolve()
    suite = _read_json(root / suite_path if not suite_path.is_absolute() else suite_path)
    if not isinstance(suite, dict):
        raise PromptEvalError("prompt eval suite root must be a JSON object")
    if suite.get("schema_version") != 1:
        raise PromptEvalError("prompt eval suite schema_version must equal 1")
    _require_non_empty_string(suite.get("id"), "suite id")
    if suite.get("mode") != "deterministic_text_contract":
        raise PromptEvalError("suite mode must equal deterministic_text_contract")

    limitations = _require_non_empty_string(suite.get("limitations"), "suite limitations")
    lower_limitations = limitations.lower()
    for marker in ("does not execute an llm", "stochastic", "model/runtime trials"):
        if marker not in lower_limitations:
            raise PromptEvalError(f"suite limitations must explicitly state deterministic scope: missing {marker!r}")

    policy = suite.get("eval_policy")
    if not isinstance(policy, dict):
        raise PromptEvalError("eval_policy must be a JSON object")
    minimum_trials = policy.get("minimum_model_trials_when_nondeterminism_matters")
    deterministic_checks = policy.get("deterministic_checks")
    max_safety_regression = policy.get("maximum_regression_on_safety_critical_cases")
    if not isinstance(minimum_trials, int) or isinstance(minimum_trials, bool) or minimum_trials < 3:
        raise PromptEvalError("minimum_model_trials_when_nondeterminism_matters must be an integer >= 3")
    if deterministic_checks != 1:
        raise PromptEvalError("deterministic_checks must equal 1")
    if max_safety_regression != 0:
        raise PromptEvalError("maximum_regression_on_safety_critical_cases must equal 0")

    declared_categories = set(_require_string_list(suite.get("required_categories"), "required_categories"))
    if declared_categories != REQUIRED_CATEGORIES:
        missing = sorted(REQUIRED_CATEGORIES - declared_categories)
        extra = sorted(declared_categories - REQUIRED_CATEGORIES)
        raise PromptEvalError(f"required_categories drift; missing={missing}, extra={extra}")

    cases = suite.get("cases")
    if not isinstance(cases, list) or not cases:
        raise PromptEvalError("cases must be a non-empty JSON array")

    seen_ids: set[str] = set()
    covered_categories: set[str] = set()
    safety_cases = 0
    findings: list[str] = []

    for index, raw_case in enumerate(cases):
        if not isinstance(raw_case, dict):
            findings.append(f"case {index}: expected a JSON object")
            continue
        try:
            case_id = _require_non_empty_string(raw_case.get("id"), f"case {index} id")
            category = _require_non_empty_string(raw_case.get("category"), f"{case_id}: category")
            source_value = _require_non_empty_string(raw_case.get("source"), f"{case_id}: source")
            must_contain = _require_string_list(raw_case.get("must_contain"), f"{case_id}: must_contain")
            must_not_contain = _require_string_list(
                raw_case.get("must_not_contain", []), f"{case_id}: must_not_contain", allow_empty=True
            )
            if case_id in seen_ids:
                raise PromptEvalError(f"duplicate case id: {case_id}")
            seen_ids.add(case_id)
            if category not in REQUIRED_CATEGORIES:
                raise PromptEvalError(f"{case_id}: unsupported category: {category}")
            covered_categories.add(category)
            if raw_case.get("safety_critical") is True:
                safety_cases += 1
            elif raw_case.get("safety_critical") not in (None, False):
                raise PromptEvalError(f"{case_id}: safety_critical must be boolean when present")

            source = _safe_source(root, source_value, case_id)
            text = source.read_text(encoding="utf-8")
            for marker in must_contain:
                if marker not in text:
                    findings.append(f"{case_id}: {source_value} missing required marker: {marker}")
            for marker in must_not_contain:
                if marker in text:
                    findings.append(f"{case_id}: {source_value} contains forbidden marker: {marker}")
        except PromptEvalError as exc:
            findings.append(str(exc))

    missing_categories = sorted(REQUIRED_CATEGORIES - covered_categories)
    if missing_categories:
        findings.append("eval suite does not cover required categories: " + ", ".join(missing_categories))
    if safety_cases < 3:
        findings.append("eval suite must contain at least three explicit safety_critical cases")

    if findings:
        raise PromptEvalError("Prompt contract evaluation failed:\n- " + "\n- ".join(findings))

    return {
        "suite_id": suite["id"],
        "mode": suite["mode"],
        "cases": len(cases),
        "categories": len(covered_categories),
        "safety_critical_cases": safety_cases,
        "model_trials_executed": 0,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--suite", type=Path, default=DEFAULT_SUITE)
    args = parser.parse_args()

    try:
        result = validate_suite(args.root, args.suite)
    except PromptEvalError as exc:
        print(str(exc))
        return 1

    print(
        "Prompt contract PASS: "
        f"suite={result['suite_id']} cases={result['cases']} categories={result['categories']} "
        f"safety_critical={result['safety_critical_cases']} model_trials_executed=0. "
        "Deterministic repository contract only; stochastic model/runtime adherence is not claimed."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
