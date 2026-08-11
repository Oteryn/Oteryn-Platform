#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from verify_integration_test_registration import RegistrationError, validate_repository


TEST_PATH = "tests/Integration/Example/ExampleIntegrationTest.php"
WORKFLOW_PATH = ".github/workflows/example-integration.yml"
TRIGGER = "tests/Integration/Example/**"
ENVIRONMENT = ["EXAMPLE_BASELINE_PATH", "EXAMPLE_CANDIDATE_PATH"]


class IntegrationTestRegistrationTest(unittest.TestCase):
    def make_repository(
        self,
        *,
        include_test: bool = True,
        registration_path: str = TEST_PATH,
        invocation_marker: str = TEST_PATH,
        trigger_marker: str = TRIGGER,
        workflow_invocation: str = TEST_PATH,
        workflow_trigger: str = TRIGGER,
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
            "  pull_request:",
            "    paths:",
            f"      - '{workflow_trigger}'",
            "jobs:",
            "  integration:",
            "    runs-on: ubuntu-latest",
            "    env:",
        ]
        if include_environment:
            workflow_lines.extend(f"      {name}: fixture" for name in ENVIRONMENT)
        workflow_lines.extend(
            [
                "    steps:",
                f"      - run: vendor/bin/phpunit {workflow_invocation}",
            ]
        )
        self.write(root / WORKFLOW_PATH, "\n".join(workflow_lines) + "\n")

        record = {
            "path": registration_path,
            "workflow": WORKFLOW_PATH,
            "invocation_marker": invocation_marker,
            "trigger_marker": trigger_marker,
            "required_environment": ENVIRONMENT,
        }
        records = [record, dict(record)] if duplicate else [record]
        registry = {"schema_version": 1, "tests": records}
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

    def test_workflow_must_contain_exact_test_invocation(self) -> None:
        root = self.make_repository(workflow_invocation="tests/Integration/Example")
        self.assert_registration_error(root, "does not explicitly invoke")

    def test_workflow_must_contain_trigger_marker(self) -> None:
        root = self.make_repository(workflow_trigger="tests/Integration/Other/**")
        self.assert_registration_error(root, "does not contain trigger marker")

    def test_workflow_must_contain_required_environment_markers(self) -> None:
        root = self.make_repository(include_environment=False)
        self.assert_registration_error(root, "missing required environment markers")


if __name__ == "__main__":
    unittest.main(verbosity=2)
