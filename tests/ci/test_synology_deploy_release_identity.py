#!/usr/bin/env python3

from __future__ import annotations

import os
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
BUILD_WORKFLOW = ROOT / ".github/workflows/build-synology-staging-images.yml"
DEPLOY_WORKFLOW = ROOT / ".github/workflows/deploy-synology-staging.yml"
CHARACTER_WORKFLOW = ROOT / ".github/workflows/character-bazaar-staging-control.yml"
LIQUID20_WORKFLOW = ROOT / ".github/workflows/liquid20-synology-control.yml"
PREFLIGHT_WORKFLOW = ROOT / ".github/workflows/synology-production-target-preflight.yml"
PREFLIGHT = ROOT / "deploy/synology/scripts/production-target-preflight.sh"
GHCR_HELPER = ROOT / "deploy/synology/scripts/repository-ghcr-image.sh"
RUNTIME_ENV = ROOT / "deploy/synology/.env.example"
RUNNER_ENV = ROOT / "deploy/synology/runner/.env.example"
RUNNER_COMPOSE = ROOT / "deploy/synology/runner/compose.yml"
RUNNER_ENTRYPOINT = ROOT / "deploy/synology/runner/entrypoint.sh"


class SynologyDeployReleaseIdentityContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.build_workflow = BUILD_WORKFLOW.read_text(encoding="utf-8")
        cls.workflow = DEPLOY_WORKFLOW.read_text(encoding="utf-8")
        cls.character_workflow = CHARACTER_WORKFLOW.read_text(encoding="utf-8")
        cls.liquid20_workflow = LIQUID20_WORKFLOW.read_text(encoding="utf-8")
        cls.preflight_workflow = PREFLIGHT_WORKFLOW.read_text(encoding="utf-8")
        cls.preflight = PREFLIGHT.read_text(encoding="utf-8")
        cls.helper = GHCR_HELPER.read_text(encoding="utf-8")
        cls.runtime_env = RUNTIME_ENV.read_text(encoding="utf-8")
        cls.runner_env = RUNNER_ENV.read_text(encoding="utf-8")
        cls.runner_compose = RUNNER_COMPOSE.read_text(encoding="utf-8")
        cls.runner_entrypoint = RUNNER_ENTRYPOINT.read_text(encoding="utf-8")

    def run_helper(self, owner: str, package: str = "oteryn-platform") -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env.pop("OTERYN_GHCR_OWNER", None)
        env["GITHUB_REPOSITORY_OWNER"] = owner
        return subprocess.run(
            ["bash", str(GHCR_HELPER), package],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    def test_repository_ghcr_helper_lowercases_current_owner(self) -> None:
        result = self.run_helper("Oteryn")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "ghcr.io/oteryn/oteryn-platform")

    def test_repository_ghcr_helper_rejects_invalid_owner_or_package(self) -> None:
        invalid_owner = self.run_helper("bad_owner")
        self.assertNotEqual(invalid_owner.returncode, 0)
        invalid_package = self.run_helper("Oteryn", "Bad/Package")
        self.assertNotEqual(invalid_package.returncode, 0)

    def test_deploy_requires_exact_commit_sha_instead_of_mutable_release_tag(self) -> None:
        self.assertIn("release_sha:", self.workflow)
        self.assertIn("^[0-9a-f]{40}$", self.workflow)
        self.assertNotIn("release_tag:", self.workflow)
        self.assertNotIn("default: main", self.workflow)

    def test_platform_and_gateway_source_revision_must_match_release_sha(self) -> None:
        self.assertIn("org.opencontainers.image.revision", self.workflow)
        self.assertIn('platform_revision" != "$RELEASE_SHA"', self.workflow)
        self.assertIn('gateway_revision" != "$RELEASE_SHA"', self.workflow)

    def test_runtime_images_are_resolved_to_digest_references(self) -> None:
        self.assertIn("Resolve immutable runtime image digests", self.workflow)
        self.assertIn("@sha256:", self.workflow)
        self.assertIn("platform_ref=", self.workflow)
        self.assertIn("gateway_ref=", self.workflow)
        self.assertIn("canary_ref=", self.workflow)
        self.assertIn("PLATFORM_IMAGE=$platform_image", self.workflow)
        self.assertIn("GATEWAY_IMAGE=$gateway_image", self.workflow)
        self.assertIn("CANARY_IMAGE=$canary_image", self.workflow)

    def test_digest_resolution_occurs_before_environment_is_written(self) -> None:
        self.assertLess(
            self.workflow.index("Resolve immutable runtime image digests"),
            self.workflow.index("Write ephemeral staging environment"),
        )

    def test_build_and_deploy_resolve_platform_images_from_current_repository_owner(self) -> None:
        marker = "deploy/synology/scripts/repository-ghcr-image.sh"
        self.assertIn(marker, self.build_workflow)
        self.assertIn(marker, self.workflow)
        self.assertIn("package: oteryn-platform", self.build_workflow)
        self.assertIn("package: oteryn-game-gateway", self.build_workflow)
        self.assertIn("package: oteryn-deploy-runner", self.build_workflow)
        self.assertIn('platform_repo="$(bash deploy/synology/scripts/repository-ghcr-image.sh oteryn-platform)"', self.workflow)
        self.assertIn('gateway_repo="$(bash deploy/synology/scripts/repository-ghcr-image.sh oteryn-game-gateway)"', self.workflow)

    def test_character_bazaar_uses_current_owner_for_platform_images_but_preserves_canary_pin(self) -> None:
        self.assertIn("repository-ghcr-image.sh oteryn-platform", self.character_workflow)
        self.assertIn("repository-ghcr-image.sh oteryn-game-gateway", self.character_workflow)
        self.assertIn(
            "ghcr.io/blakinio/canary@sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f",
            self.character_workflow,
        )
        self.assertIn("contains(github.event.head_commit.message, '[character-bazaar-staging]')", self.character_workflow)

    def test_liquid20_package_uses_current_repository_owner_while_source_pin_remains_external(self) -> None:
        self.assertIn("repository-ghcr-image.sh liquid20-collector", self.liquid20_workflow)
        self.assertNotIn("ghcr.io/blakinio/liquid20-collector", self.liquid20_workflow)
        self.assertIn("repository: blakinio/freqtrade", self.liquid20_workflow)
        self.assertIn("c00a091c5adc67cf75c46db5805e358ffc72fad7", self.liquid20_workflow)

    def test_repository_only_hardening_does_not_auto_publish_or_bootstrap_on_main_push(self) -> None:
        build_push = self.build_workflow.split("  push:\n", 1)[1].split("  workflow_dispatch:\n", 1)[0]
        self.assertIn("deploy/synology/docker/**", build_push)
        self.assertNotIn(".github/workflows/", build_push)
        self.assertNotIn("deploy/synology/runner/", build_push)
        self.assertNotIn("repository-ghcr-image.sh", build_push)
        self.assertIn(
            "github.event_name == 'workflow_dispatch' || (github.event_name == 'push' && matrix.name != 'deploy-runner')",
            self.build_workflow,
        )

        liquid20_push = self.liquid20_workflow.split("  push:\n", 1)[1].split("  schedule:\n", 1)[0]
        self.assertIn("deploy/liquid20/**", liquid20_push)
        self.assertNotIn(".github/workflows/", liquid20_push)
        self.assertNotIn("repository-ghcr-image.sh", liquid20_push)

    def test_production_target_preflight_accepts_current_owner_exact_digest_runtime_refs(self) -> None:
        self.assertIn("repository-ghcr-image.sh", self.preflight)
        self.assertIn('platform_prefix="${platform_repo}@sha256:"', self.preflight)
        self.assertIn('gateway_prefix="${gateway_repo}@sha256:"', self.preflight)
        self.assertIn('fail "Platform is not deployed by immutable digest from the current repository owner"', self.preflight)
        self.assertIn('fail "Gateway is not deployed by immutable digest from the current repository owner"', self.preflight)
        self.assertIn("OTERYN_GHCR_OWNER: ${{ github.repository_owner }}", self.preflight_workflow)

    def test_preflight_recovers_release_sha_after_owner_neutral_digest_validation(self) -> None:
        self.assertIn('docker image inspect --format', self.preflight)
        self.assertIn('org.opencontainers.image.revision', self.preflight)
        self.assertIn('[[ "$image_revision" =~ ^[a-f0-9]{40}$ ]]', self.preflight)
        self.assertIn('elif [[ "$image_revision" != "$deployed_release_sha" ]]', self.preflight)
        self.assertIn('fail "Gateway OCI revision does not match the Platform release SHA"', self.preflight)
        self.assertLess(
            self.preflight.index('platform_prefix="${platform_repo}@sha256:"'),
            self.preflight.index('org.opencontainers.image.revision'),
        )

    def test_runner_registration_coordinates_are_explicit_and_restart_safe(self) -> None:
        self.assertIn("RUNNER_URL=REQUIRED_REPOSITORY_URL", self.runner_env)
        self.assertIn("RUNNER_GHCR_OWNER=REQUIRED_REPOSITORY_OWNER_LOWERCASE", self.runner_env)
        self.assertIn("image: ${RUNNER_IMAGE:?RUNNER_IMAGE must be set explicitly}", self.runner_compose)
        self.assertIn("RUNNER_URL: ${RUNNER_URL:-}", self.runner_compose)
        self.assertIn('RUNNER_URL="${RUNNER_URL:-}"', self.runner_entrypoint)
        self.assertIn("if [[ ! -f .runner ]]; then", self.runner_entrypoint)
        self.assertIn("Provide the exact repository RUNNER_URL before first registration", self.runner_entrypoint)

    def test_runtime_example_is_owner_neutral(self) -> None:
        self.assertIn("OTERYN_GHCR_OWNER=REQUIRED_REPOSITORY_OWNER_LOWERCASE", self.runtime_env)
        self.assertIn("ghcr.io/${OTERYN_GHCR_OWNER}/oteryn-platform:main", self.runtime_env)
        self.assertIn("ghcr.io/${OTERYN_GHCR_OWNER}/oteryn-game-gateway:main", self.runtime_env)

    def test_transfer_sensitive_platform_paths_do_not_hardcode_old_owner_coordinates(self) -> None:
        texts = {
            "build": self.build_workflow,
            "deploy": self.workflow,
            "character": self.character_workflow,
            "liquid20": self.liquid20_workflow,
            "preflight": self.preflight,
            "runtime-env": self.runtime_env,
            "runner-env": self.runner_env,
            "runner-compose": self.runner_compose,
            "runner-entrypoint": self.runner_entrypoint,
        }
        forbidden = (
            "ghcr.io/blakinio/oteryn-platform",
            "ghcr.io/blakinio/oteryn-game-gateway",
            "ghcr.io/blakinio/oteryn-deploy-runner",
            "ghcr.io/blakinio/liquid20-collector",
            "https://github.com/blakinio/Oteryn-Platform",
        )
        failures = [
            f"{name}: {marker}"
            for name, text in texts.items()
            for marker in forbidden
            if marker in text
        ]
        self.assertEqual(failures, [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
