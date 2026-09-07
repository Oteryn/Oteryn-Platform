#!/usr/bin/env python3

from __future__ import annotations

import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
BUILD_WORKFLOW = ROOT / ".github/workflows/build-synology-staging-images.yml"
DEPLOY_WORKFLOW = ROOT / ".github/workflows/deploy-synology-staging.yml"
DEPLOY_SCRIPT = ROOT / "deploy/synology/scripts/deploy.sh"
IPV4_HELPER = ROOT / "deploy/synology/scripts/validate-ipv4.sh"


class SynologyAutoStagingDeployContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.build_workflow = BUILD_WORKFLOW.read_text(encoding="utf-8")
        cls.deploy_workflow = DEPLOY_WORKFLOW.read_text(encoding="utf-8")
        cls.deploy_script = DEPLOY_SCRIPT.read_text(encoding="utf-8")

    def test_runtime_and_synology_main_changes_trigger_exact_image_build(self) -> None:
        push_block = self.build_workflow.split("  push:\n", 1)[1].split("  workflow_dispatch:\n", 1)[0]
        self.assertIn("      - deploy/synology/**", push_block)
        self.assertIn("      - .github/workflows/build-synology-staging-images.yml", push_block)
        self.assertIn("      - .github/workflows/deploy-synology-staging.yml", push_block)
        self.assertIn("      - app/**", push_block)
        self.assertIn("      - public/**", push_block)
        self.assertIn("      - resources/**", push_block)

    def test_main_push_waits_for_build_then_dispatches_without_waiting_for_remote_deploy(self) -> None:
        self.assertIn("  deploy-staging:\n", self.build_workflow)
        self.assertIn("github.event_name == 'push' && github.ref == 'refs/heads/main'", self.build_workflow)
        self.assertIn("      - validate-deployment\n      - build", self.build_workflow)
        self.assertIn("actions: write", self.build_workflow)
        self.assertIn("actions/workflows/deploy-synology-staging.yml/dispatches", self.build_workflow)
        self.assertIn("Dispatch exact main SHA to Synology staging", self.build_workflow)
        self.assertNotIn("gh run watch", self.build_workflow)
        self.assertNotIn("Locate and monitor Synology deployment run", self.build_workflow)
        self.assertIn("Deployment continues independently in Deploy Synology Staging", self.build_workflow)

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

    def test_deploy_uses_repository_canonical_origin_without_environment_drift(self) -> None:
        self.assertIn(
            "CANONICAL_PUBLIC_APP_URL: https://oteryn.molehill.cloud",
            self.deploy_workflow,
        )
        self.assertNotIn("vars.OTERYN_STAGING_APP_URL", self.deploy_workflow)
        self.assertNotIn("APP_URL_INPUT", self.deploy_workflow)
        self.assertIn('app_url="$CANONICAL_PUBLIC_APP_URL"', self.deploy_workflow)
        self.assertIn("APP_URL=$app_url", self.deploy_workflow)

    def run_ipv4_helper(self, address: str, policy: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["bash", str(IPV4_HELPER), address, policy],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    def test_synology_deploy_workflow_no_longer_requires_python(self) -> None:
        self.assertNotIn("python3 --version", self.deploy_workflow)
        self.assertNotIn("python3 -", self.deploy_workflow)
        self.assertIn(
            'bash deploy/synology/scripts/validate-ipv4.sh "$CANARY_GAME_BIND_ADDRESS_INPUT" private-or-loopback',
            self.deploy_workflow,
        )

    def test_synology_deploy_script_no_longer_requires_python(self) -> None:
        self.assertNotIn("command -v python3", self.deploy_script)
        self.assertNotIn("python3 -", self.deploy_script)
        self.assertIn("validate-ipv4.sh", self.deploy_script)

    def test_shell_ipv4_helper_accepts_only_loopback_or_rfc1918_for_game_bind(self) -> None:
        accepted = (
            "127.0.0.1", "127.20.1.2", "10.0.0.1", "172.16.0.1",
            "172.31.255.254", "192.168.1.2",
        )
        rejected = (
            "0.0.0.0", "8.8.8.8", "169.254.1.1", "172.15.0.1",
            "172.32.0.1", "192.167.1.1", "224.0.0.1", "256.1.1.1",
            "192.168.001.2", "192.168.1", "not-an-ip",
        )
        for address in accepted:
            with self.subTest(address=address):
                self.assertEqual(self.run_ipv4_helper(address, "private-or-loopback").returncode, 0)
        for address in rejected:
            with self.subTest(address=address):
                self.assertNotEqual(self.run_ipv4_helper(address, "private-or-loopback").returncode, 0)

    def test_shell_ipv4_helper_keeps_service_binds_on_exact_loopback(self) -> None:
        self.assertEqual(self.run_ipv4_helper("127.0.0.1", "loopback").returncode, 0)
        for address in ("127.0.0.2", "10.0.0.1", "192.168.1.2"):
            with self.subTest(address=address):
                self.assertNotEqual(self.run_ipv4_helper(address, "loopback").returncode, 0)

    def test_runtime_deploy_avoids_duplicate_full_compose_pull(self) -> None:
        self.assertNotIn('"${compose[@]}" pull', self.deploy_script)
        self.assertIn('docker image inspect "$runtime_image"', self.deploy_script)

    def test_runtime_health_names_gateway_dependency_failures_before_ready(self) -> None:
        health = (ROOT / "deploy/synology/scripts/health-check.sh").read_text(encoding="utf-8")
        self.assertIn('Canary session issuer /health', health)
        self.assertIn('Gateway -> Platform', health)
        self.assertIn('Gateway -> Canary session issuer', health)
        self.assertIn('Gateway aggregate readiness failed after both internal dependencies passed.', health)

    def test_new_main_can_finalize_a_proven_previous_candidate_before_transition(self) -> None:
        self.assertIn("finalize_previous_candidate_if_healthy()", self.deploy_script)
        finalizer_call = self.deploy_script.index("\nfinalize_previous_candidate_if_healthy\n")
        baseline = self.deploy_script.index('bash "$SCRIPT_DIR/prepare-fresh-schema-baseline.sh"', finalizer_call)
        self.assertLess(finalizer_call, baseline)
        self.assertIn('OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"', self.deploy_script)
        self.assertIn('Finalized previously migrated candidate', self.deploy_script)


if __name__ == "__main__":
    unittest.main(verbosity=2)
