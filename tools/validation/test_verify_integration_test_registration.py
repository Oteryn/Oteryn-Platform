#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from verify_integration_test_registration import RegistrationError, validate_repository


TEST_PATH = "tests/Integration/Example/ExampleIntegrationTest.php"
WORKFLOW_PATH = ".github/workflows/example-integration.yml"
EVENT = "pull_request"
TRIGGER = "tests/Integration/Example/**"
JOB = "integration"
CONDITION = "github.event_name == 'pull_request'"
ENVIRONMENT = ["EXAMPLE_BASELINE_PATH", "EXAMPLE_CANDIDATE_PATH"]


class IntegrationTestRegistrationTest(unittest.TestCase):
    def make_repository(
        self,
        *,
        include_test: bool = True,
        registration_path: str = TEST_PATH,
        event: str = EVENT,
        registration_event: str = EVENT,
        registration_job: str = JOB,
        invocation_marker: str = TEST_PATH,
        trigger_marker: str = TRIGGER,
        job_condition_marker: str = CONDITION,
        workflow_invocation: str = TEST_PATH,
        workflow_trigger: str = TRIGGER,
        workflow_condition: str = CONDITION,
        invocation_as_comment: bool = False,
        duplicate: bool = False,
        include_environment: bool = True,
    ) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)

        if include_test:
            self.write(root / TEST_PATH, "<?php\n")

        workflow_lines = [
            "name: Example Integration",
            "on:",
            f"  {event}:",
            "    paths:",
            f"      - '{workflow_trigger}'",
            "permissions:",
            "  contents: read",
            "jobs:",
            f"  {JOB}:",
            f"    if: {workflow_condition}",
            "    runs-on: ubuntu-latest",
            "    env:",
        ]
        if include_environment:
            workflow_lines.extend(f"      {name}: fixture" for name in ENVIRONMENT)
        workflow_lines.extend(["    steps:"])
        if invocation_as_comment:
            workflow_lines.append(f"      # vendor/bin/phpunit {TEST_PATH}")
            workflow_lines.append("      - run: vendor/bin/phpunit tests/Integration/Example")
        else:
            workflow_lines.append(f"      - run: vendor/bin/phpunit {workflow_invocation}")
        self.write(root / WORKFLOW_PATH, "\n".join(workflow_lines) + "\n")

        record = {
            "path": registration_path,
            "workflow": WORKFLOW_PATH,
            "event": registration_event,
            "job": registration_job,
            "invocation_marker": invocation_marker,
            "trigger_marker": trigger_marker,
            "job_condition_marker": job_condition_marker,
            "required_environment": ENVIRONMENT,
        }
        records = [record, dict(record)] if duplicate else [record]
        registry = {"schema_version": 2, "tests": records}
        self.write(root / "tests/Integration/REGISTRY.json", json.dumps(registry, indent=2) + "\n")
        return root

    @staticmethod
    def write(path: Path, content: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def assert_registration_error(self, root: Path, marker: str) -> None:
        with self.assertRaises(RegistrationError) as caught:
            validate_repository(root)
        self.assertIn(marker, str(caught.exception))

    def test_valid_registration_passes(self) -> None:
        root = self.make_repository()
        self.assertEqual([TEST_PATH], validate_repository(root))

    def test_unregistered_test_fails(self) -> None:
        root = self.make_repository(registration_path="tests/Integration/Example/OtherTest.php")
        self.assert_registration_error(root, "unregistered Integration test")

    def test_stale_registration_fails(self) -> None:
        root = self.make_repository(include_test=False)
        self.assert_registration_error(root, "stale Integration registration")

    def test_duplicate_registration_fails(self) -> None:
        root = self.make_repository(duplicate=True)
        self.assert_registration_error(root, "duplicate integration-test registration")

    def test_directory_only_invocation_marker_fails(self) -> None:
        root = self.make_repository(invocation_marker="tests/Integration/Example")
        self.assert_registration_error(root, "invocation_marker must be the exact test path")

    def test_workflow_must_executably_invoke_exact_test(self) -> None:
        root = self.make_repository(workflow_invocation="tests/Integration/Example")
        self.assert_registration_error(root, "does not executably invoke")

    def test_comment_does_not_count_as_executable_invocation(self) -> None:
        root = self.make_repository(invocation_as_comment=True)
        self.assert_registration_error(root, "does not executably invoke")

    def test_trigger_marker_must_be_in_declared_top_level_event(self) -> None:
        root = self.make_repository(registration_event="workflow_dispatch")
        self.assert_registration_error(root, "has no top-level on.workflow_dispatch trigger")

    def test_proving_job_must_match_registry_job(self) -> None:
        root = self.make_repository(registration_job="other-job")
        self.assert_registration_error(root, "has no jobs.other-job proving job")

    def test_proving_job_condition_must_match_declared_event_path(self) -> None:
        root = self.make_repository(
            job_condition_marker="github.event_name == 'pull_request'",
            workflow_condition="github.event_name == 'workflow_dispatch'",
        )
        self.assert_registration_error(root, "does not contain required condition marker")

    def test_unrelated_trigger_text_cannot_satisfy_proving_job_contract(self) -> None:
        root = self.make_repository(
            registration_event="pull_request",
            workflow_condition="github.event_name == 'workflow_dispatch'",
            job_condition_marker="github.event_name == 'pull_request'",
        )
        self.assert_registration_error(root, "does not contain required condition marker")

    def test_workflow_must_contain_required_environment_markers_in_proving_job(self) -> None:
        root = self.make_repository(include_environment=False)
        self.assert_registration_error(root, "is missing required environment markers")


if __name__ == "__main__":
    unittest.main(verbosity=2)
