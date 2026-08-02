#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
CLASSIFIER_PATH = REPOSITORY_ROOT / "scripts/ci/classify_changes.py"
FIXTURES_PATH = Path(__file__).parent / "fixtures/change-routing-cases.json"

spec = importlib.util.spec_from_file_location("classify_changes", CLASSIFIER_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"Unable to load classifier from {CLASSIFIER_PATH}")
classifier = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = classifier
spec.loader.exec_module(classifier)


class ChangeRoutingTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture = json.loads(FIXTURES_PATH.read_text(encoding="utf-8"))

    def test_declared_change_classes(self) -> None:
        observed = set()
        for case in self.fixture["cases"][:13]:
            result = classifier.classify_paths(case["paths"])
            observed.update(result["classes"])
        self.assertEqual(
            {
                "docs_only",
                "agent_governance",
                "backend",
                "frontend",
                "dependency",
                "database",
                "auth_security",
                "payment",
                "go_gateway",
                "deployment",
                "edge",
                "shared",
                "workflow",
            },
            observed,
        )

    def test_fixture_routing(self) -> None:
        for case in self.fixture["cases"]:
            with self.subTest(case=case["name"]):
                result = classifier.classify_paths(case["paths"])
                self.assertEqual(case["classes"], result["classes"])
                expected_gates = {
                    gate: gate in case["gates"] for gate in classifier.GATES
                }
                self.assertEqual(expected_gates, result["gates"])

    def test_empty_change_set_fails_closed(self) -> None:
        result = classifier.classify_paths([])
        self.assertEqual(["shared"], result["classes"])
        self.assertTrue(all(result["gates"].values()))

    def test_force_all_fails_closed(self) -> None:
        result = classifier.classify_paths(["docs/agents/example.md"], force_all=True)
        self.assertEqual(["shared"], result["classes"])
        self.assertTrue(all(result["gates"].values()))

    def test_github_output_and_summary_are_explicit(self) -> None:
        result = classifier.classify_paths(["docs/agents/example.md"])
        with tempfile.TemporaryDirectory() as directory:
            output_path = Path(directory) / "output"
            summary_path = Path(directory) / "summary"
            classifier.write_github_output(output_path, result)
            classifier.write_summary(summary_path, result)
            output = output_path.read_text(encoding="utf-8")
            summary = summary_path.read_text(encoding="utf-8")

        self.assertIn("ci=false", output)
        self.assertIn("phase7=false", output)
        self.assertIn("classes=agent_governance", output)
        self.assertIn("routing evidence only", summary)
        self.assertIn("| `ci` | `SKIP` |", summary)


if __name__ == "__main__":
    unittest.main()
