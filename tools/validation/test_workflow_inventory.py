#!/usr/bin/env python3

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from workflow_inventory import WorkflowInventoryError, classify_workflow, validate_inventory


BASE = """name: Test\non:\n{events}\npermissions:\n  contents: read\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: true\n"""


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
            "manual-audit.yml": ("  workflow_dispatch:\n", "manual_validation"),
        }
        for filename, (events, expected) in cases.items():
            with self.subTest(filename=filename):
                text = BASE.format(events=events.rstrip())
                self.assertEqual(expected, classify_workflow(Path(filename), text))

    def test_unknown_trigger_shape_fails_closed(self) -> None:
        text = BASE.format(events="  repository_dispatch:")
        with self.assertRaisesRegex(WorkflowInventoryError, "unclassified workflow"):
            classify_workflow(Path("mystery.yml"), text)

    def test_inventory_requires_top_level_permissions(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / ".github/workflows/example.yml"
            path.parent.mkdir(parents=True)
            path.write_text(
                "name: Example\non:\n  pull_request:\njobs:\n  test:\n    permissions:\n      contents: read\n    runs-on: ubuntu-latest\n",
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
                    f"name: Example\non:\n  pull_request:\n{permissions}\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: true\n",
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


if __name__ == "__main__":
    unittest.main(verbosity=2)
