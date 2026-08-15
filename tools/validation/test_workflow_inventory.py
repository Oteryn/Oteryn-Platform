#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from workflow_inventory import (
    WorkflowInventoryError,
    classify_workflow,
    validate_inventory,
    validate_lifecycle_policy,
)


BASE = """name: Test\non:\n{events}\npermissions:\n  contents: read\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: true\n"""


def write_policy(
    root: Path,
    *,
    registered: list[str],
    retired: list[str] | None = None,
    manual_only: list[str] | None = None,
    budget: int | None = None,
) -> None:
    policy = {
        "schema_version": 1,
        "workflow_budget": budget if budget is not None else max(1, len(registered)),
        "registered_workflows": registered,
        "retired_workflows": retired or [],
        "manual_only_workflows": manual_only or [],
    }
    path = root / "docs/agents/CI_WORKFLOW_LIFECYCLE.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(policy), encoding="utf-8")


class WorkflowInventoryTest(unittest.TestCase):
    def test_semantic_classes_are_stable(self) -> None:
        cases = {
            "ci.yml": ("  pull_request:\n", "required_core"),
            "agent-governance.yml": ("  pull_request:\n", "governance"),
            "build-images.yml": ("  push:\n", "build"),
            "deploy-staging.yml": ("  workflow_dispatch:\n", "deployment_operation"),
            "shared-validation.yml": ("  workflow_call:\n", "reusable_validation"),
            "weekly-health.yml": ("  schedule:\n    - cron: '0 0 * * 1'\n", "scheduled_validation"),
            "feature-contract.yml": ("  pull_request:\n", "domain_validation"),
            "trusted-pr-validation.yml": ("  pull_request_target:\n", "domain_validation"),
            "manual-audit.yml": ("  workflow_dispatch:\n", "manual_validation"),
            "owner-comment-control.yml": ("  issue_comment:\n", "manual_validation"),
        }
        for filename, (events, expected) in cases.items():
            with self.subTest(filename=filename):
                text = BASE.format(events=events.rstrip())
                self.assertEqual(expected, classify_workflow(Path(filename), text))

    def test_quoted_supported_event_is_classified(self) -> None:
        text = BASE.format(events='  "pull_request":')
        self.assertEqual("domain_validation", classify_workflow(Path("feature-contract.yml"), text))

    def test_unknown_trigger_shape_fails_closed(self) -> None:
        text = BASE.format(events="  repository_dispatch:")
        with self.assertRaisesRegex(WorkflowInventoryError, "unclassified workflow"):
            classify_workflow(Path("mystery.yml"), text)

    def test_filename_classification_cannot_bypass_unsupported_trigger(self) -> None:
        text = BASE.format(events="  repository_dispatch:")
        for filename in ("ci.yml", "build-malicious.yml", "deploy-malicious.yml"):
            with self.subTest(filename=filename):
                with self.assertRaisesRegex(WorkflowInventoryError, "unsupported top-level event"):
                    classify_workflow(Path(filename), text)

    def test_supported_trigger_cannot_hide_an_unsupported_trigger(self) -> None:
        text = BASE.format(events="  pull_request:\n  repository_dispatch:")
        with self.assertRaisesRegex(WorkflowInventoryError, "unsupported top-level event"):
            classify_workflow(Path("feature-contract.yml"), text)

    def test_quoted_unsupported_trigger_cannot_hide_behind_supported_trigger(self) -> None:
        text = BASE.format(events='  pull_request:\n  "repository_dispatch":')
        for filename in ("ci.yml", "build-malicious.yml", "deploy-malicious.yml"):
            with self.subTest(filename=filename):
                with self.assertRaisesRegex(WorkflowInventoryError, "unsupported top-level event"):
                    classify_workflow(Path(filename), text)

    def test_unparseable_direct_event_key_fails_closed(self) -> None:
        text = BASE.format(events="  pull_request:\n  repository.dispatch:")
        with self.assertRaisesRegex(WorkflowInventoryError, "unparseable top-level workflow event key"):
            classify_workflow(Path("build-malicious.yml"), text)

    def test_job_name_cannot_masquerade_as_supported_event(self) -> None:
        text = """name: Test
on:
  repository_dispatch:
permissions:
  contents: read
jobs:
  pull_request:
    runs-on: ubuntu-latest
    steps:
      - run: true
"""
        with self.assertRaisesRegex(WorkflowInventoryError, "unclassified workflow"):
            classify_workflow(Path("mystery.yml"), text)

    def test_nested_step_key_cannot_masquerade_as_supported_event(self) -> None:
        text = """name: Test
on:
  repository_dispatch:
permissions:
  contents: read
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: pull request text
        env:
          pull_request: yes
        run: true
"""
        with self.assertRaisesRegex(WorkflowInventoryError, "unclassified workflow"):
            classify_workflow(Path("mystery.yml"), text)

    def test_inventory_requires_top_level_permissions(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / ".github/workflows/example.yml"
            path.parent.mkdir(parents=True)
            path.write_text(
                "name: Example\non:\n  pull_request:\njobs:\n  test:\n"
                "    permissions:\n      contents: read\n    runs-on: ubuntu-latest\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(WorkflowInventoryError, "missing required top-level workflow marker permissions:"):
                validate_inventory(root)

    def test_inventory_accepts_supported_top_level_permissions_forms(self) -> None:
        forms = ("permissions: {}", "permissions: read-all", "permissions: write-all")
        for permissions in forms:
            with self.subTest(permissions=permissions), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                path = root / ".github/workflows/example.yml"
                path.parent.mkdir(parents=True)
                path.write_text(
                    f"name: Example\non:\n  pull_request:\n{permissions}\n"
                    "jobs:\n  test:\n    runs-on: ubuntu-latest\n"
                    "    steps:\n      - run: true\n",
                    encoding="utf-8",
                )
                result = validate_inventory(root)
                self.assertEqual("domain_validation", result[".github/workflows/example.yml"])

    def test_inventory_classifies_every_workflow_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workflow_root = root / ".github/workflows"
            workflow_root.mkdir(parents=True)
            (workflow_root / "ci.yml").write_text(
                BASE.format(events="  pull_request:"), encoding="utf-8"
            )
            (workflow_root / "weekly.yml").write_text(
                BASE.format(events="  schedule:\n    - cron: '0 0 * * 1'"), encoding="utf-8"
            )
            result = validate_inventory(root)
            self.assertEqual(2, len(result))
            self.assertEqual("required_core", result[".github/workflows/ci.yml"])
            self.assertEqual("scheduled_validation", result[".github/workflows/weekly.yml"])

    def test_lifecycle_rejects_unregistered_workflow(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workflow_root = root / ".github/workflows"
            workflow_root.mkdir(parents=True)
            (workflow_root / "ci.yml").write_text(
                BASE.format(events="  pull_request:"), encoding="utf-8"
            )
            (workflow_root / "surprise.yml").write_text(
                BASE.format(events="  workflow_dispatch:"), encoding="utf-8"
            )
            write_policy(root, registered=["ci.yml"], budget=1)
            with self.assertRaisesRegex(WorkflowInventoryError, "unregistered workflow files"):
                validate_lifecycle_policy(root)

    def test_lifecycle_rejects_reintroduced_retired_workflow(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workflow_root = root / ".github/workflows"
            workflow_root.mkdir(parents=True)
            (workflow_root / "old.yml").write_text(
                BASE.format(events="  workflow_dispatch:"), encoding="utf-8"
            )
            write_policy(root, registered=[], retired=["old.yml"], budget=1)
            with self.assertRaisesRegex(WorkflowInventoryError, "retired workflows were reintroduced"):
                validate_lifecycle_policy(root)

    def test_lifecycle_requires_every_registered_workflow_to_exist(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / ".github/workflows").mkdir(parents=True)
            write_policy(root, registered=["missing.yml"], budget=1)
            with self.assertRaisesRegex(WorkflowInventoryError, "registered workflows missing"):
                validate_lifecycle_policy(root)

    def test_lifecycle_enforces_manual_only_trigger(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workflow_root = root / ".github/workflows"
            workflow_root.mkdir(parents=True)
            (workflow_root / "deep.yml").write_text(
                BASE.format(events="  pull_request:\n  workflow_dispatch:"), encoding="utf-8"
            )
            write_policy(root, registered=["deep.yml"], manual_only=["deep.yml"], budget=1)
            with self.assertRaisesRegex(WorkflowInventoryError, "manual-only workflow"):
                validate_lifecycle_policy(root)

    def test_lifecycle_accepts_explicit_registered_manual_inventory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workflow_root = root / ".github/workflows"
            workflow_root.mkdir(parents=True)
            (workflow_root / "ci.yml").write_text(
                BASE.format(events="  pull_request:"), encoding="utf-8"
            )
            (workflow_root / "deep.yml").write_text(
                BASE.format(events="  workflow_dispatch:"), encoding="utf-8"
            )
            write_policy(
                root,
                registered=["ci.yml", "deep.yml"],
                retired=["old.yml"],
                manual_only=["deep.yml"],
                budget=2,
            )
            result = validate_lifecycle_policy(root)
            self.assertEqual(2, result["actual"])
            self.assertEqual(1, result["retired"])
            self.assertEqual(1, result["manual_only"])

    def test_lifecycle_enforces_workflow_budget(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workflow_root = root / ".github/workflows"
            workflow_root.mkdir(parents=True)
            for name in ("a.yml", "b.yml"):
                (workflow_root / name).write_text(
                    BASE.format(events="  workflow_dispatch:"), encoding="utf-8"
                )
            write_policy(root, registered=["a.yml", "b.yml"], budget=1)
            with self.assertRaisesRegex(WorkflowInventoryError, "workflow budget exceeded"):
                validate_lifecycle_policy(root)


if __name__ == "__main__":
    unittest.main(verbosity=2)
