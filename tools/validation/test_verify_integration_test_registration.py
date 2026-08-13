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
        workflow_trigger: str = TRIGGER,
        workflow_condition: str = CONDITION,
        workflow_condition_comment: str | None = None,
        duplicate: bool = False,
        dispatch_input: str | None = None,
        dispatch_input_as_comment: bool = False,
        dispatch_input_as_inline_comment: bool = False,
        invocation_mode: str = "run",
        environment_mode: str = "job_env",
    ) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)

        if include_test:
            self.write(root / TEST_PATH, "<?php\n")

        workflow_lines = ["name: Example Integration", "on:", f"  {event}:"]
        if event == "workflow_dispatch":
            workflow_lines.append("    inputs:")
            if dispatch_input is not None:
                if dispatch_input_as_comment:
                    workflow_lines.append(f"      # {dispatch_input}:")
                elif dispatch_input_as_inline_comment:
                    workflow_lines.append(f"      unrelated: # {dispatch_input}:")
                    workflow_lines.extend(["        required: false", "        type: boolean"])
                else:
                    workflow_lines.append(f"      {dispatch_input}:")
                    workflow_lines.extend(["        required: true", "        type: boolean"])
        else:
            workflow_lines.extend(["    paths:", f"      - '{workflow_trigger}'"])
        workflow_lines.extend([
            "permissions:",
            "  contents: read",
            "jobs:",
            f"  {JOB}:",
            f"    if: {workflow_condition}",
        ])
        if workflow_condition_comment is not None:
            workflow_lines.append(f"    # {workflow_condition_comment}")
        workflow_lines.append("    runs-on: ubuntu-latest")

        if environment_mode == "job_env":
            workflow_lines.extend(["    env:", *[f"      {name}: fixture" for name in ENVIRONMENT]])

        workflow_lines.append("    steps:")
        if environment_mode == "job_env":
            pass
        elif environment_mode == "github_env":
            workflow_lines.extend([
                "      - name: Prepare environment",
                "        run: |",
                f"          echo \"{ENVIRONMENT[0]}=/tmp/base\" >> \"$GITHUB_ENV\"",
                f"          echo \"{ENVIRONMENT[1]}=/tmp/candidate\" >> \"${{GITHUB_ENV}}\"",
            ])
        elif environment_mode == "github_env_alias":
            workflow_lines.extend([
                "      - name: Write lookalike environment values",
                "        run: |",
                f"          echo \"OTHER={ENVIRONMENT[0]}=/tmp/base\" >> \"$GITHUB_ENV\"",
                f"          echo \"ANOTHER={ENVIRONMENT[1]}=/tmp/candidate\" >> \"$GITHUB_ENV\"",
            ])
        elif environment_mode == "comment":
            workflow_lines.extend([f"      # {ENVIRONMENT[0]}", f"      # {ENVIRONMENT[1]}"])
        elif environment_mode == "step_name":
            workflow_lines.extend([
                f"      - name: {ENVIRONMENT[0]} {ENVIRONMENT[1]}",
                "        run: true",
            ])
        elif environment_mode == "echo_only":
            workflow_lines.extend([
                "      - name: Mention environment only",
                "        run: |",
                f"          echo {ENVIRONMENT[0]}",
                f"          echo {ENVIRONMENT[1]}",
            ])
        elif environment_mode != "none":
            raise ValueError(f"unsupported environment_mode: {environment_mode}")

        if invocation_mode == "run":
            workflow_lines.extend([
                "      - name: Execute integration test",
                "        run: |",
                "          set -o pipefail",
                "          vendor/bin/phpunit \\",
                f"            {TEST_PATH} \\",
                "            --testdox",
            ])
        elif invocation_mode == "comment":
            workflow_lines.extend([
                f"      # vendor/bin/phpunit {TEST_PATH}",
                "      - run: vendor/bin/phpunit tests/Integration/Example",
            ])
        elif invocation_mode == "inline_comment":
            workflow_lines.append(f"      - run: echo ok # vendor/bin/phpunit {TEST_PATH}")
        elif invocation_mode == "step_name":
            workflow_lines.extend([
                f"      - name: vendor/bin/phpunit {TEST_PATH}",
                "        run: echo ok",
            ])
        elif invocation_mode == "echo":
            workflow_lines.append(f"      - run: echo vendor/bin/phpunit {TEST_PATH}")
        elif invocation_mode == "directory":
            workflow_lines.append("      - run: vendor/bin/phpunit tests/Integration/Example")
        elif invocation_mode == "or_true":
            workflow_lines.append(f"      - run: vendor/bin/phpunit {TEST_PATH} || true")
        elif invocation_mode == "unreachable":
            workflow_lines.extend([
                "      - run: |",
                "          exit 0",
                f"          vendor/bin/phpunit {TEST_PATH}",
            ])
        elif invocation_mode == "pipeline_without_pipefail":
            workflow_lines.append(
                f"      - run: vendor/bin/phpunit {TEST_PATH} | tee /tmp/integration.log"
            )
        elif invocation_mode == "pipeline_with_pipefail":
            workflow_lines.extend([
                "      - run: |",
                "          set -o pipefail",
                f"          vendor/bin/phpunit {TEST_PATH} | tee /tmp/integration.log",
            ])
        else:
            raise ValueError(f"unsupported invocation_mode: {invocation_mode}")

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

    def test_valid_workflow_dispatch_input_registration_passes(self) -> None:
        condition = "github.event_name == 'workflow_dispatch'"
        root = self.make_repository(
            event="workflow_dispatch",
            registration_event="workflow_dispatch",
            trigger_marker="run_proof:",
            job_condition_marker=condition,
            workflow_condition=condition,
            dispatch_input="run_proof",
        )
        self.assertEqual([TEST_PATH], validate_repository(root))

    def test_github_env_exports_before_proving_step_are_accepted(self) -> None:
        root = self.make_repository(environment_mode="github_env")
        self.assertEqual([TEST_PATH], validate_repository(root))

    def test_proving_step_env_is_accepted(self) -> None:
        root = self.make_repository(environment_mode="none")
        path = root / WORKFLOW_PATH
        text = path.read_text(encoding="utf-8")
        marker = "      - name: Execute integration test\n        run: |"
        replacement = (
            "      - name: Execute integration test\n"
            "        env:\n"
            f"          {ENVIRONMENT[0]}: fixture\n"
            f"          {ENVIRONMENT[1]}: fixture\n"
            "        run: |"
        )
        path.write_text(text.replace(marker, replacement), encoding="utf-8")
        self.assertEqual([TEST_PATH], validate_repository(root))

    def test_pipefail_pipeline_is_accepted(self) -> None:
        root = self.make_repository(invocation_mode="pipeline_with_pipefail")
        self.assertEqual([TEST_PATH], validate_repository(root))

    def test_prior_step_env_does_not_leak_into_proving_step(self) -> None:
        root = self.make_repository(environment_mode="none")
        path = root / WORKFLOW_PATH
        text = path.read_text(encoding="utf-8")
        marker = "    steps:\n"
        replacement = (
            "    steps:\n"
            "      - name: Prior scoped env\n"
            "        env:\n"
            f"          {ENVIRONMENT[0]}: fixture\n"
            f"          {ENVIRONMENT[1]}: fixture\n"
            "        run: true\n"
        )
        path.write_text(text.replace(marker, replacement, 1), encoding="utf-8")
        self.assert_registration_error(root, "missing executable required environment provisioning")

    def test_same_step_github_env_export_does_not_retroactively_apply(self) -> None:
        root = self.make_repository(environment_mode="none")
        path = root / WORKFLOW_PATH
        text = path.read_text(encoding="utf-8")
        marker = "          set -o pipefail\n"
        replacement = (
            f"          echo \"{ENVIRONMENT[0]}=/tmp/base\" >> \"$GITHUB_ENV\"\n"
            f"          echo \"{ENVIRONMENT[1]}=/tmp/candidate\" >> \"$GITHUB_ENV\"\n"
            "          set -o pipefail\n"
        )
        path.write_text(text.replace(marker, replacement, 1), encoding="utf-8")
        self.assert_registration_error(root, "missing executable required environment provisioning")

    def test_lookalike_github_env_assignment_does_not_satisfy_contract(self) -> None:
        root = self.make_repository(environment_mode="github_env_alias")
        self.assert_registration_error(root, "missing executable required environment provisioning")

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

    def test_directory_only_phpunit_execution_fails(self) -> None:
        root = self.make_repository(invocation_mode="directory")
        self.assert_registration_error(root, "does not executably invoke")

    def test_comment_does_not_count_as_executable_invocation(self) -> None:
        root = self.make_repository(invocation_mode="comment")
        self.assert_registration_error(root, "does not executably invoke")

    def test_inline_comment_does_not_count_as_executable_invocation(self) -> None:
        root = self.make_repository(invocation_mode="inline_comment")
        self.assert_registration_error(root, "does not executably invoke")

    def test_step_name_does_not_count_as_executable_invocation(self) -> None:
        root = self.make_repository(invocation_mode="step_name")
        self.assert_registration_error(root, "does not executably invoke")

    def test_echo_does_not_count_as_executable_invocation(self) -> None:
        root = self.make_repository(invocation_mode="echo")
        self.assert_registration_error(root, "does not executably invoke")

    def test_masked_failure_does_not_count_as_proving_invocation(self) -> None:
        root = self.make_repository(invocation_mode="or_true")
        self.assert_registration_error(root, "does not executably invoke")

    def test_unreachable_invocation_does_not_count_as_proving_invocation(self) -> None:
        root = self.make_repository(invocation_mode="unreachable")
        self.assert_registration_error(root, "does not executably invoke")

    def test_pipeline_without_pipefail_does_not_count_as_proving_invocation(self) -> None:
        root = self.make_repository(invocation_mode="pipeline_without_pipefail")
        self.assert_registration_error(root, "does not executably invoke")

    def test_trigger_marker_must_be_in_declared_top_level_event(self) -> None:
        root = self.make_repository(registration_event="workflow_dispatch")
        self.assert_registration_error(root, "has no top-level on.workflow_dispatch trigger")

    def test_commented_workflow_dispatch_input_cannot_satisfy_trigger_contract(self) -> None:
        condition = "github.event_name == 'workflow_dispatch'"
        root = self.make_repository(
            event="workflow_dispatch",
            registration_event="workflow_dispatch",
            trigger_marker="run_proof:",
            job_condition_marker=condition,
            workflow_condition=condition,
            dispatch_input="run_proof",
            dispatch_input_as_comment=True,
        )
        self.assert_registration_error(root, "does not contain executable marker")

    def test_inline_comment_cannot_spoof_workflow_dispatch_input(self) -> None:
        condition = "github.event_name == 'workflow_dispatch'"
        root = self.make_repository(
            event="workflow_dispatch",
            registration_event="workflow_dispatch",
            trigger_marker="run_proof:",
            job_condition_marker=condition,
            workflow_condition=condition,
            dispatch_input="run_proof",
            dispatch_input_as_inline_comment=True,
        )
        self.assert_registration_error(root, "does not contain executable marker")

    def test_proving_job_must_match_registry_job(self) -> None:
        root = self.make_repository(registration_job="other-job")
        self.assert_registration_error(root, "has no jobs.other-job proving job")

    def test_proving_job_condition_must_match_declared_event_path(self) -> None:
        root = self.make_repository(
            job_condition_marker="github.event_name == 'pull_request'",
            workflow_condition="github.event_name == 'workflow_dispatch'",
        )
        self.assert_registration_error(root, "does not contain required condition marker")

    def test_commented_condition_cannot_satisfy_proving_job_contract(self) -> None:
        root = self.make_repository(
            registration_event="pull_request",
            workflow_condition="github.event_name == 'workflow_dispatch'",
            workflow_condition_comment="github.event_name == 'pull_request'",
            job_condition_marker="github.event_name == 'pull_request'",
        )
        self.assert_registration_error(root, "does not contain required condition marker")

    def test_inline_commented_condition_cannot_satisfy_proving_job_contract(self) -> None:
        root = self.make_repository(
            registration_event="pull_request",
            workflow_condition="github.event_name == 'workflow_dispatch' # github.event_name == 'pull_request'",
            job_condition_marker="github.event_name == 'pull_request'",
        )
        self.assert_registration_error(root, "does not contain required condition marker")

    def test_environment_names_in_comments_do_not_satisfy_contract(self) -> None:
        root = self.make_repository(environment_mode="comment")
        self.assert_registration_error(root, "missing executable required environment provisioning")

    def test_environment_names_in_step_name_do_not_satisfy_contract(self) -> None:
        root = self.make_repository(environment_mode="step_name")
        self.assert_registration_error(root, "missing executable required environment provisioning")

    def test_echoing_environment_names_without_github_env_does_not_satisfy_contract(self) -> None:
        root = self.make_repository(environment_mode="echo_only")
        self.assert_registration_error(root, "missing executable required environment provisioning")

    def test_missing_required_environment_fails(self) -> None:
        root = self.make_repository(environment_mode="none")
        self.assert_registration_error(root, "missing executable required environment provisioning")


if __name__ == "__main__":
    unittest.main(verbosity=2)
