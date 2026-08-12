#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from prompt_eval import PromptEvalError, REQUIRED_CATEGORIES, validate_suite


class PromptEvalTest(unittest.TestCase):
    def make_repository(self, *, mutate=None) -> tuple[Path, Path]:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        source = root / "docs/prompt.md"
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_text("ALLOW\nSAFE\n", encoding="utf-8")

        cases = []
        categories = sorted(REQUIRED_CATEGORIES)
        for index, category in enumerate(categories):
            case = {
                "id": f"case-{index}-{category}",
                "category": category,
                "source": "docs/prompt.md",
                "must_contain": ["ALLOW"],
            }
            if category in {"boundary_refusal", "authority_stop", "prompt_injection"}:
                case["safety_critical"] = True
            cases.append(case)

        suite = {
            "schema_version": 1,
            "id": "test-suite",
            "mode": "deterministic_text_contract",
            "eval_policy": {
                "minimum_model_trials_when_nondeterminism_matters": 3,
                "deterministic_checks": 1,
                "maximum_regression_on_safety_critical_cases": 0,
            },
            "limitations": (
                "This does not execute an LLM; it is deterministic and does not prove stochastic "
                "adherence, so model/runtime trials remain required when behaviour changes."
            ),
            "required_categories": categories,
            "cases": cases,
        }
        if mutate is not None:
            mutate(root, suite)

        suite_path = root / "suite.json"
        suite_path.write_text(json.dumps(suite, indent=2) + "\n", encoding="utf-8")
        return root, Path("suite.json")

    def test_balanced_suite_passes_without_claiming_model_trials(self) -> None:
        root, suite = self.make_repository()
        result = validate_suite(root, suite)
        self.assertEqual(len(REQUIRED_CATEGORIES), result["categories"])
        self.assertEqual(0, result["model_trials_executed"])
        self.assertEqual(3, result["safety_critical_cases"])

    def test_missing_required_marker_fails(self) -> None:
        def mutate(_root, suite):
            suite["cases"][0]["must_contain"] = ["MISSING"]

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "missing required marker"):
            validate_suite(root, suite)

    def test_forbidden_marker_fails(self) -> None:
        def mutate(_root, suite):
            suite["cases"][0]["must_not_contain"] = ["SAFE"]

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "contains forbidden marker"):
            validate_suite(root, suite)

    def test_missing_category_fails(self) -> None:
        def mutate(_root, suite):
            removed = suite["required_categories"].pop()
            suite["cases"] = [case for case in suite["cases"] if case["category"] != removed]

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "required_categories drift"):
            validate_suite(root, suite)

    def test_fewer_than_three_safety_cases_fails(self) -> None:
        def mutate(_root, suite):
            safety = [case for case in suite["cases"] if case.get("safety_critical")]
            safety[0].pop("safety_critical")

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "at least three"):
            validate_suite(root, suite)

    def test_source_path_escape_fails(self) -> None:
        def mutate(_root, suite):
            suite["cases"][0]["source"] = "../outside.md"

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "repository-relative"):
            validate_suite(root, suite)

    def test_limitations_must_disclaim_model_execution(self) -> None:
        def mutate(_root, suite):
            suite["limitations"] = "Automated prompt eval."

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "deterministic scope"):
            validate_suite(root, suite)


if __name__ == "__main__":
    unittest.main(verbosity=2)
