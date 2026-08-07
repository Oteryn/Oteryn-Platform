#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
GATE_PATH = REPOSITORY_ROOT / "scripts/ci/required_test_gate.py"
WORKFLOW_PATH = REPOSITORY_ROOT / ".github/workflows/ci.yml"

spec = importlib.util.spec_from_file_location("required_test_gate", GATE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"Unable to load required test gate from {GATE_PATH}")
gate = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = gate
spec.loader.exec_module(gate)


class RequiredTestGateTest(unittest.TestCase):
    def evaluate(
        self,
        *,
        classification: str = "success",
        ci_required: str = "true",
        runtime_tests: str = "success",
    ):
        return gate.evaluate_gate(
            classification_result=classification,
            ci_required=ci_required,
            runtime_tests_result=runtime_tests,
        )

    def test_documentation_only_change_passes_with_explicit_not_applicable(self) -> None:
        decision = self.evaluate(ci_required="false", runtime_tests="skipped")

        self.assertTrue(decision.passed)
        self.assertEqual("not-applicable", decision.outcome)
        self.assertFalse(decision.runtime_tests_required)
        self.assertIn("NOT_APPLICABLE", decision.message)

    def test_runtime_change_passes_only_after_runtime_tests_succeed(self) -> None:
        decision = self.evaluate(ci_required="true", runtime_tests="success")

        self.assertTrue(decision.passed)
        self.assertEqual("runtime-tests-passed", decision.outcome)
        self.assertTrue(decision.runtime_tests_required)

    def test_classifier_failure_fails_closed(self) -> None:
        decision = self.evaluate(
            classification="failure", ci_required="", runtime_tests="skipped"
        )

        self.assertFalse(decision.passed)
        self.assertIsNone(decision.runtime_tests_required)
        self.assertIn("failing closed", decision.message)

    def test_malformed_classifier_output_fails_closed(self) -> None:
        decision = self.evaluate(ci_required="maybe", runtime_tests="skipped")

        self.assertFalse(decision.passed)
        self.assertIn("invalid gate decision", decision.message)

    def test_required_runtime_tests_cannot_be_skipped(self) -> None:
        decision = self.evaluate(ci_required="true", runtime_tests="skipped")

        self.assertFalse(decision.passed)
        self.assertTrue(decision.runtime_tests_required)
        self.assertIn("were required", decision.message)

    def test_failed_runtime_tests_fail_the_required_context(self) -> None:
        decision = self.evaluate(ci_required="true", runtime_tests="failure")

        self.assertFalse(decision.passed)
        self.assertIn("did not succeed", decision.message)

    def test_not_applicable_path_requires_the_conditional_job_to_be_skipped(self) -> None:
        decision = self.evaluate(ci_required="false", runtime_tests="success")

        self.assertFalse(decision.passed)
        self.assertFalse(decision.runtime_tests_required)
        self.assertIn("did not report skipped", decision.message)

    def test_summary_records_pass_and_not_applicable_evidence(self) -> None:
        decision = self.evaluate(ci_required="false", runtime_tests="skipped")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "summary"
            gate.write_summary(path, decision)
            summary = path.read_text(encoding="utf-8")

        self.assertIn("result: `PASS`", summary)
        self.assertIn("outcome: `not-applicable`", summary)
        self.assertIn("runtime tests required: `NO`", summary)


class RequiredTestWorkflowContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW_PATH.read_text(encoding="utf-8")

    def test_classifier_validates_both_ci_routing_contracts(self) -> None:
        self.assertIn("python tests/ci/test_classify_changes.py", self.workflow)
        self.assertIn("python tests/ci/test_required_test_gate.py", self.workflow)

    def test_protected_classifier_validates_active_task_checkpoints(self) -> None:
        classifier = self.workflow.split("\n  runtime_tests:\n", 1)[0]

        self.assertIn("name: classify-changes", classifier)
        self.assertIn(
            "python tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint",
            classifier,
        )

    def test_runtime_suite_is_conditional_on_successful_runtime_classification(self) -> None:
        self.assertIn("  runtime_tests:\n    name: runtime-tests", self.workflow)
        self.assertIn(
            "if: ${{ needs.classify_changes.result == 'success' && needs.classify_changes.outputs.ci == 'true' }}",
            self.workflow,
        )
        self.assertIn("image: mariadb:11.8", self.workflow)
        self.assertIn("php artisan test --log-junit=artifacts/ci/phpunit-junit.xml", self.workflow)

    def test_protected_test_context_is_always_emitted_as_aggregate_gate(self) -> None:
        self.assertIn(
            "  test:\n    needs:\n      - classify_changes\n      - runtime_tests\n    if: ${{ always() }}",
            self.workflow,
        )
        self.assertIn("python scripts/ci/required_test_gate.py", self.workflow)
        self.assertIn(
            "RUNTIME_TESTS_RESULT: ${{ needs.runtime_tests.result }}", self.workflow
        )


if __name__ == "__main__":
    unittest.main()
