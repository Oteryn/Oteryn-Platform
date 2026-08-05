#!/usr/bin/env python3
"""Regression contract for the protected CI `test` context."""

from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_PATH = ROOT / ".github" / "workflows" / "ci.yml"


def job_block(workflow: str, job_id: str) -> str:
    match = re.search(
        rf"(?ms)^  {re.escape(job_id)}:\n(?P<body>.*?)(?=^  [A-Za-z0-9_]+:\n|\Z)",
        workflow,
    )
    if match is None:
        raise AssertionError(f"Missing workflow job: {job_id}")

    return match.group("body")


def terminal_gate_passes(
    classification_result: str,
    ci_required: bool,
    full_suite_result: str,
) -> bool:
    if classification_result != "success":
        return False
    if ci_required and full_suite_result != "success":
        return False
    return True


class RequiredTestGateWorkflowTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        cls.classifier = job_block(cls.workflow, "classify_changes")
        cls.full_suite = job_block(cls.workflow, "test_suite")
        cls.required_gate = job_block(cls.workflow, "test")

    def test_classifier_validates_the_gate_contract(self) -> None:
        self.assertIn("python tests/ci/test_classify_changes.py", self.classifier)
        self.assertIn("python tests/ci/test_ci_required_gate.py", self.classifier)

    def test_expensive_suite_is_internal_and_conditional(self) -> None:
        self.assertIn("name: full-test-suite", self.full_suite)
        self.assertIn("needs: classify_changes", self.full_suite)
        self.assertIn(
            "if: ${{ needs.classify_changes.result == 'success' && "
            "needs.classify_changes.outputs.ci == 'true' }}",
            self.full_suite,
        )
        self.assertIn("services:", self.full_suite)
        self.assertIn("mariadb:", self.full_suite)

    def test_required_context_always_materializes_without_services(self) -> None:
        self.assertIn("- classify_changes", self.required_gate)
        self.assertIn("- test_suite", self.required_gate)
        self.assertIn("if: ${{ always() }}", self.required_gate)
        self.assertNotIn("services:", self.required_gate)
        self.assertIsNone(
            re.search(r"(?m)^    name:", self.required_gate),
            "The protected context must remain the job id `test`.",
        )

    def test_required_context_fails_closed(self) -> None:
        self.assertIn("needs.classify_changes.result != 'success'", self.required_gate)
        self.assertIn("needs.classify_changes.outputs.ci == 'true'", self.required_gate)
        self.assertIn("needs.test_suite.result != 'success'", self.required_gate)

    def test_terminal_state_matrix(self) -> None:
        cases = {
            ("failure", False, "skipped"): False,
            ("cancelled", False, "skipped"): False,
            ("success", False, "skipped"): True,
            ("success", True, "failure"): False,
            ("success", True, "cancelled"): False,
            ("success", True, "success"): True,
        }

        for inputs, expected in cases.items():
            with self.subTest(inputs=inputs):
                self.assertEqual(terminal_gate_passes(*inputs), expected)


if __name__ == "__main__":
    unittest.main()
