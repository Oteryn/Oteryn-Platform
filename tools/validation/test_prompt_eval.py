#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from prompt_eval import PromptEvalError, validate_suite


class PromptEvalTest(unittest.TestCase):
    def make_repository(self, *, mutate=None) -> tuple[Path, Path]:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        source = root / "docs/prompt.md"
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_text("ALLOW\nSAFE\n", encoding="utf-8")
        categories = ["binding", "authority", "evidence"]
        cases = [
            {
                "id": f"case-{category}",
                "category": category,
                "source": "docs/prompt.md",
                "must_contain": ["ALLOW"],
                "safety_critical": True,
            }
            for category in categories
        ]
        suite = {
            "schema_version": 2,
            "id": "test-suite",
            "mode": "deterministic_contract_only",
            "evidence_class": "structural_static",
            "limitations": (
                "This does not execute an LLM; model_trials_executed=0. "
                "It does not prove provider delivery or runtime behavior."
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
        self.assertEqual("structural_static", result["evidence_class"])
        self.assertEqual(0, result["model_trials_executed"])
        self.assertEqual(3, result["safety_critical_cases"])

    def test_missing_required_marker_fails(self) -> None:
        root, suite = self.make_repository(
            mutate=lambda _root, data: data["cases"][0].update(must_contain=["MISSING"])
        )
        with self.assertRaisesRegex(PromptEvalError, "missing required marker"):
            validate_suite(root, suite)

    def test_forbidden_marker_fails(self) -> None:
        root, suite = self.make_repository(
            mutate=lambda _root, data: data["cases"][0].update(must_not_contain=["SAFE"])
        )
        with self.assertRaisesRegex(PromptEvalError, "contains forbidden marker"):
            validate_suite(root, suite)

    def test_undeclared_category_fails(self) -> None:
        root, suite = self.make_repository(
            mutate=lambda _root, data: data["cases"][0].update(category="undeclared")
        )
        with self.assertRaisesRegex(PromptEvalError, "undeclared category"):
            validate_suite(root, suite)

    def test_uncovered_declared_category_fails(self) -> None:
        def mutate(_root, data):
            data["required_categories"].append("missing")

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "category coverage drift"):
            validate_suite(root, suite)

    def test_fewer_than_three_safety_cases_fails(self) -> None:
        def mutate(_root, data):
            data["cases"][0]["safety_critical"] = False

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "at least three"):
            validate_suite(root, suite)

    def test_source_path_escape_fails(self) -> None:
        root, suite = self.make_repository(
            mutate=lambda _root, data: data["cases"][0].update(source="../outside.md")
        )
        with self.assertRaisesRegex(PromptEvalError, "repository-relative"):
            validate_suite(root, suite)

    def test_limitations_must_disclaim_all_three_evidence_classes(self) -> None:
        root, suite = self.make_repository(
            mutate=lambda _root, data: data.update(limitations="Automated prompt eval.")
        )
        with self.assertRaisesRegex(PromptEvalError, "limitations missing"):
            validate_suite(root, suite)

    def test_evidence_class_must_be_structural_static(self) -> None:
        root, suite = self.make_repository(
            mutate=lambda _root, data: data.update(evidence_class="provider_adoption")
        )
        with self.assertRaisesRegex(PromptEvalError, "evidence_class"):
            validate_suite(root, suite)

    def test_boolean_is_not_a_valid_safety_label_string(self) -> None:
        root, suite = self.make_repository(
            mutate=lambda _root, data: data["cases"][0].update(safety_critical="yes")
        )
        with self.assertRaisesRegex(PromptEvalError, "must be boolean"):
            validate_suite(root, suite)


if __name__ == "__main__":
    unittest.main()
