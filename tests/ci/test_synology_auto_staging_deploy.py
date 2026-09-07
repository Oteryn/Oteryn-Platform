#!/usr/bin/env python3

from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
BUILD_WORKFLOW = ROOT / ".github/workflows/build-synology-staging-images.yml"
DEPLOY_WORKFLOW = ROOT / ".github/workflows/deploy-synology-staging.yml"


class SynologyAutoStagingDeployContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.build_workflow = BUILD_WORKFLOW.read_text(encoding="utf-8")
        cls.deploy_workflow = DEPLOY_WORKFLOW.read_text(encoding="utf-8")

    def test_runtime_and_synology_main_changes_trigger_exact_image_build(self) -> None:
        push_block = self.build_workflow.split("  push:\n", 1)[1].split("  workflow_dispatch:\n", 1)[0]
        self.assertIn("      - deploy/synology/**", push_block)
        self.assertIn("      - .github/workflows/build-synology-staging-images.yml", push_block)
        self.assertIn("      - .github/workflows/deploy-synology-staging.yml", push_block)
        self.assertIn("      - app/**", push_block)
        self.assertIn("      - public/**", push_block)
        self.assertIn("      - resources/**", push_block)

    def test_main_push_waits_for_build_before_dispatching_staging(self) -> None:
        self.assertIn("  deploy-staging:\n", self.build_workflow)
        self.assertIn("github.event_name == 'push' && github.ref == 'refs/heads/main'", self.build_workflow)
        self.assertIn("      - validate-deployment\n      - build", self.build_workflow)
        self.assertIn("actions: write", self.build_workflow)
        self.assertIn("actions/workflows/deploy-synology-staging.yml/dispatches", self.build_workflow)
        self.assertIn("gh run watch", self.build_workflow)

    def test_automatic_deploy_is_exact_sha_and_preserves_approved_staging_identity(self) -> None:
        expected = (
            "ghcr.io/blakinio/canary@sha256:"
            "784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f"
        )
        self.assertIn("inputs[release_sha]=${GITHUB_SHA}", self.build_workflow)
        self.assertIn(expected, self.build_workflow)
        self.assertIn("inputs[canary_game_bind_address]=192.168.1.2", self.build_workflow)
        self.assertIn("inputs[game_world_id]=1", self.build_workflow)
        self.assertIn("inputs[game_world_slug]=oteryn-staging", self.build_workflow)
        self.assertIn("inputs[game_world_name]=Oteryn Staging", self.build_workflow)
        self.assertIn("inputs[game_world_region]=LAN", self.build_workflow)

    def test_superseded_main_build_does_not_dispatch_an_old_release(self) -> None:
        self.assertIn('git/ref/heads/main', self.build_workflow)
        self.assertIn('if [[ "$current_main" != "$GITHUB_SHA" ]]', self.build_workflow)
        self.assertIn("Skipping superseded main deployment", self.build_workflow)
        self.assertIn("steps.dispatch.outputs.dispatched == 'true'", self.build_workflow)

    def test_pull_requests_never_enter_the_staging_dispatch_job(self) -> None:
        self.assertNotIn("pull_request && github.ref == 'refs/heads/main'", self.build_workflow)
        self.assertIn("if: github.event_name == 'push' && github.ref == 'refs/heads/main'", self.build_workflow)

    def test_privileged_deploy_runner_remains_manual_only(self) -> None:
        self.assertIn(
            "github.event_name == 'workflow_dispatch' || (github.event_name == 'push' && matrix.name != 'deploy-runner')",
            self.build_workflow,
        )

    def test_manual_deploy_and_rollback_remain_available(self) -> None:
        self.assertIn("  workflow_dispatch:\n", self.deploy_workflow)
        self.assertIn("          - deploy\n          - rollback", self.deploy_workflow)
        self.assertIn("release_sha:", self.deploy_workflow)
        self.assertIn("if: inputs.action == 'rollback'", self.deploy_workflow)


if __name__ == "__main__":
    unittest.main(verbosity=2)
