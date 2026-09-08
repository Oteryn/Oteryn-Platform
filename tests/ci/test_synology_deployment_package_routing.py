#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CLASSIFIER = ROOT / "scripts/ci/classify_synology_builds.py"
WORKFLOW = ROOT / ".github/workflows/build-synology-staging-images.yml"
HEAD_SHA = "a" * 40


class SynologyDeploymentPackageRoutingTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")

    def classify(self, event: str, path: str) -> dict[str, object]:
        result = subprocess.run(
            [
                sys.executable,
                str(CLASSIFIER),
                "--event",
                event,
                "--head",
                HEAD_SHA,
                "--paths-json",
                json.dumps([path]),
            ],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    @staticmethod
    def matrix_names(result: dict[str, object]) -> list[str]:
        matrix = result["matrix"]
        assert isinstance(matrix, dict)
        include = matrix["include"]
        assert isinstance(include, list)
        return [str(entry["name"]) for entry in include if isinstance(entry, dict)]

    def test_docs_inside_synology_tree_are_not_deployment_inputs(self) -> None:
        for path in (
            "deploy/synology/README.md",
            "deploy/synology/PUBLIC_ENDPOINTS.md",
            "deploy/synology/.gitignore",
        ):
            pr = self.classify("pull_request", path)
            main = self.classify("push", path)
            for result in (pr, main):
                self.assertFalse(result["platform_changed"], path)
                self.assertFalse(result["gateway_changed"], path)
                self.assertFalse(result["deploy_runner_changed"], path)
                self.assertFalse(result["deployment_package_changed"], path)
                self.assertFalse(result["runtime_deployment_changed"], path)
                self.assertFalse(result["deployment_validation_changed"], path)
                self.assertFalse(result["has_image_builds"], path)
                self.assertEqual(self.matrix_names(result), ["noop"], path)
            self.assertFalse(main["release_relevant"], path)
            self.assertNotIn(f"      - {path}\n", self.workflow, path)

    def test_runtime_deployment_inputs_reconcile_without_image_builds(self) -> None:
        for path in (
            "deploy/synology/.env.example",
            "deploy/synology/compose.yml",
            "deploy/synology/compose.marketplace.yml",
            "deploy/synology/nginx/internal.conf",
            "deploy/synology/tls/init.sh",
            "deploy/synology/mariadb/init/10-platform-database.sh",
            "deploy/synology/scripts/deploy.sh",
            "deploy/synology/scripts/health-check.sh",
            "deploy/synology/scripts/prepare-fresh-schema-baseline.sh",
            "deploy/synology/scripts/validate-ipv4.sh",
        ):
            pr = self.classify("pull_request", path)
            main = self.classify("push", path)
            for result in (pr, main):
                self.assertTrue(result["deployment_package_changed"], path)
                self.assertTrue(result["runtime_deployment_changed"], path)
                self.assertFalse(result["has_image_builds"], path)
                self.assertEqual(self.matrix_names(result), ["noop"], path)
            self.assertTrue(main["release_relevant"], path)

    def test_operator_inputs_validate_but_do_not_reconcile(self) -> None:
        for path in (
            "deploy/synology/runner/.env.example",
            "deploy/synology/runner/compose.yml",
            "deploy/synology/scripts/marketplace-staging.sh",
        ):
            pr = self.classify("pull_request", path)
            main = self.classify("push", path)
            for result in (pr, main):
                self.assertTrue(result["deployment_package_changed"], path)
                self.assertFalse(result["runtime_deployment_changed"], path)
                self.assertFalse(result["has_image_builds"], path)
            self.assertFalse(main["release_relevant"], path)

    def test_validation_only_inputs_never_reconcile(self) -> None:
        for path in (
            "deploy/synology/tests/test_health_check_path.py",
            "deploy/synology/scripts/production-target-preflight.sh",
        ):
            main = self.classify("push", path)
            self.assertTrue(main["deployment_validation_changed"], path)
            self.assertFalse(main["deployment_package_changed"], path)
            self.assertFalse(main["runtime_deployment_changed"], path)
            self.assertFalse(main["has_image_builds"], path)
            self.assertFalse(main["release_relevant"], path)

    def test_build_workflow_does_not_use_broad_synology_glob(self) -> None:
        self.assertNotIn("      - deploy/synology/**\n", self.workflow)
        for required in (
            "      - deploy/synology/.env.example\n",
            "      - deploy/synology/compose.yml\n",
            "      - deploy/synology/nginx/**\n",
            "      - deploy/synology/tls/**\n",
            "      - deploy/synology/mariadb/init/**\n",
            "      - deploy/synology/tests/**\n",
        ):
            self.assertIn(required, self.workflow, required)


if __name__ == "__main__":
    unittest.main(verbosity=2)
