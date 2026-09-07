#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import subprocess
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
BUILD_WORKFLOW = ROOT / ".github/workflows/build-synology-staging-images.yml"
DEPLOY_WORKFLOW = ROOT / ".github/workflows/deploy-synology-staging.yml"
CHARACTER_WORKFLOW = ROOT / ".github/workflows/character-bazaar-staging-control.yml"
PREFLIGHT_WORKFLOW = ROOT / ".github/workflows/synology-production-target-preflight.yml"
PREFLIGHT = ROOT / "deploy/synology/scripts/production-target-preflight.sh"
GHCR_HELPER = ROOT / "deploy/synology/scripts/repository-ghcr-image.sh"
RUNTIME_ENV = ROOT / "deploy/synology/.env.example"
RUNNER_ENV = ROOT / "deploy/synology/runner/.env.example"
RUNNER_COMPOSE = ROOT / "deploy/synology/runner/compose.yml"
RUNNER_ENTRYPOINT = ROOT / "deploy/synology/runner/entrypoint.sh"
CLASSIFIER = ROOT / "scripts/ci/classify_synology_builds.py"
PLATFORM_DOCKERFILE = ROOT / "deploy/synology/docker/platform.Dockerfile"
RELEASE_STATE = ROOT / "deploy/synology/scripts/release-state.sh"
LIB = ROOT / "deploy/synology/scripts/lib.sh"
DEPLOY_SCRIPT = ROOT / "deploy/synology/scripts/deploy.sh"
ROLLBACK = ROOT / "deploy/synology/scripts/rollback.sh"


class SynologyDeployReleaseIdentityContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.build_workflow = BUILD_WORKFLOW.read_text(encoding="utf-8")
        cls.workflow = DEPLOY_WORKFLOW.read_text(encoding="utf-8")
        cls.character_workflow = CHARACTER_WORKFLOW.read_text(encoding="utf-8")
        cls.preflight_workflow = PREFLIGHT_WORKFLOW.read_text(encoding="utf-8")
        cls.preflight = PREFLIGHT.read_text(encoding="utf-8")
        cls.helper = GHCR_HELPER.read_text(encoding="utf-8")
        cls.runtime_env = RUNTIME_ENV.read_text(encoding="utf-8")
        cls.runner_env = RUNNER_ENV.read_text(encoding="utf-8")
        cls.runner_compose = RUNNER_COMPOSE.read_text(encoding="utf-8")
        cls.runner_entrypoint = RUNNER_ENTRYPOINT.read_text(encoding="utf-8")
        cls.classifier = CLASSIFIER.read_text(encoding="utf-8")
        cls.platform_dockerfile = PLATFORM_DOCKERFILE.read_text(encoding="utf-8")
        cls.release_state = RELEASE_STATE.read_text(encoding="utf-8")
        cls.lib = LIB.read_text(encoding="utf-8")
        cls.deploy_script = DEPLOY_SCRIPT.read_text(encoding="utf-8")
        cls.rollback = ROLLBACK.read_text(encoding="utf-8")

    def run_helper(self, owner: str, package: str = "oteryn-platform") -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env.pop("OTERYN_GHCR_OWNER", None)
        env["GITHUB_REPOSITORY_OWNER"] = owner
        return subprocess.run(
            ["bash", str(GHCR_HELPER), package], cwd=ROOT, env=env, text=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
        )

    def run_classifier(self, event: str, paths: list[str]) -> dict[str, object]:
        result = subprocess.run(
            [sys.executable, str(CLASSIFIER), "--event", event, "--head", "HEAD", "--paths-json", json.dumps(paths)],
            cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    @staticmethod
    def matrix_modes(result: dict[str, object]) -> dict[str, str]:
        matrix = result["matrix"]
        assert isinstance(matrix, dict)
        include = matrix["include"]
        assert isinstance(include, list)
        return {str(entry["name"]): str(entry["mode"]) for entry in include if isinstance(entry, dict)}

    def test_out_of_scope_operational_assets_are_absent(self) -> None:
        forbidden = (ROOT / ".github/workflows/liquid20-synology-control.yml", ROOT / "deploy/liquid20")
        self.assertEqual([path.relative_to(ROOT).as_posix() for path in forbidden if path.exists()], [])

    def test_repository_ghcr_helper_lowercases_current_owner(self) -> None:
        result = self.run_helper("Oteryn")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "ghcr.io/oteryn/oteryn-platform")

    def test_repository_ghcr_helper_rejects_invalid_coordinates(self) -> None:
        self.assertNotEqual(self.run_helper("bad_owner").returncode, 0)
        self.assertNotEqual(self.run_helper("Oteryn", "Bad/Package").returncode, 0)

    def test_classifier_exposes_independent_component_source_shas(self) -> None:
        result = self.run_classifier("push", ["deploy/synology/scripts/health-check.sh"])
        for key in ("platform_source_sha", "gateway_source_sha", "runner_source_sha"):
            self.assertIn(key, result)
            self.assertEqual(len(str(result[key])), 40)
        self.assertIn("platform_source_sha=", self.classifier)
        self.assertIn("gateway_source_sha=", self.classifier)

    def test_deployment_only_pr_validates_without_runtime_rebuild(self) -> None:
        result = self.run_classifier("pull_request", ["deploy/synology/scripts/health-check.sh"])
        self.assertTrue(result["runtime_deployment_changed"])
        self.assertEqual(self.matrix_modes(result), {"noop": "noop"})

    def test_deployment_only_main_reuses_both_runtime_components(self) -> None:
        result = self.run_classifier("push", ["deploy/synology/scripts/health-check.sh"])
        self.assertTrue(result["release_relevant"])
        self.assertEqual(self.matrix_modes(result), {"platform": "reuse", "game-gateway": "reuse"})

    def test_platform_and_gateway_changes_are_proportional(self) -> None:
        platform = self.run_classifier("push", ["app/Http/Controllers/Example.php"])
        self.assertEqual(self.matrix_modes(platform), {"platform": "full", "game-gateway": "reuse"})
        gateway = self.run_classifier("push", ["services/game-gateway/cmd/game-gateway/main.go"])
        self.assertEqual(self.matrix_modes(gateway), {"platform": "reuse", "game-gateway": "full"})

    def test_runner_change_is_validated_on_pr_but_never_released_automatically(self) -> None:
        path = "deploy/synology/runner/entrypoint.sh"
        self.assertEqual(self.matrix_modes(self.run_classifier("pull_request", [path])), {"deploy-runner": "full"})
        main = self.run_classifier("push", [path])
        self.assertFalse(main["release_relevant"])
        self.assertEqual(self.matrix_modes(main), {"noop": "noop"})

    def test_control_plane_change_fails_closed_to_full_builds(self) -> None:
        path = ".github/workflows/build-synology-staging-images.yml"
        self.assertEqual(
            self.matrix_modes(self.run_classifier("pull_request", [path])),
            {"platform": "full", "game-gateway": "full", "deploy-runner": "full"},
        )
        self.assertEqual(
            self.matrix_modes(self.run_classifier("push", [path])),
            {"platform": "full", "game-gateway": "full"},
        )

    def test_platform_runtime_inputs_are_bounded_and_complete(self) -> None:
        for path in (
            "artisan", "lang/pl.json", "storage/framework/.gitignore",
            "docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json",
            "docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md",
            "docs/architecture/SECURITY_ARCHITECTURE.md",
        ):
            self.assertEqual(self.matrix_modes(self.run_classifier("pull_request", [path])), {"platform": "full"}, path)
        self.assertNotIn("COPY . .", self.platform_dockerfile)
        self.assertIn("COPY docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json", self.platform_dockerfile)
        self.assertIn("COPY docs/architecture/adr/0013-wiki-administration.md", self.platform_dockerfile)

    def test_reuse_never_creates_a_synthetic_release_image(self) -> None:
        self.assertFalse((ROOT / "deploy/synology/docker/reuse-release.Dockerfile").exists())
        self.assertNotIn("metadata-only", self.build_workflow)
        self.assertNotIn("BASE_IMAGE=", self.build_workflow)
        self.assertIn('docker tag "$SOURCE_REF" "${IMAGE}:main"', self.build_workflow)
        self.assertIn('docker push "${IMAGE}:main"', self.build_workflow)
        self.assertIn("Reusable component provenance mismatch", self.build_workflow)

    def test_full_build_labels_standard_oci_revision_with_component_source_sha(self) -> None:
        self.assertIn("org.opencontainers.image.revision=${{ matrix.source_sha }}", self.build_workflow)
        self.assertNotIn("org.opencontainers.image.revision=${{ steps.identity.outputs.release_sha }}", self.build_workflow)
        self.assertIn('tags="${image}:source-${SOURCE_SHA}"', self.build_workflow)

    def test_main_dispatch_resolves_exact_component_digests_before_deploy(self) -> None:
        self.assertIn("Resolve immutable component release inputs", self.build_workflow)
        self.assertIn("PLATFORM_SOURCE_SHA: ${{ needs.classify.outputs.platform_source_sha }}", self.build_workflow)
        self.assertIn("GATEWAY_SOURCE_SHA: ${{ needs.classify.outputs.gateway_source_sha }}", self.build_workflow)
        self.assertIn("inputs[platform_source_sha]=${PLATFORM_SOURCE_SHA}", self.build_workflow)
        self.assertIn("inputs[platform_image]=${PLATFORM_IMAGE}", self.build_workflow)
        self.assertIn("inputs[gateway_source_sha]=${GATEWAY_SOURCE_SHA}", self.build_workflow)
        self.assertIn("inputs[gateway_image]=${GATEWAY_IMAGE}", self.build_workflow)
        self.assertIn("Skipping superseded main deployment", self.build_workflow)

    def test_deploy_accepts_overall_release_and_exact_component_provenance(self) -> None:
        for marker in ("release_sha:", "platform_source_sha:", "platform_image:", "gateway_source_sha:", "gateway_image:"):
            self.assertIn(marker, self.workflow)
        self.assertIn("Checkout exact release tooling", self.workflow)
        self.assertIn("inputs.action == 'deploy' && inputs.release_sha || 'main'", self.workflow)
        self.assertIn('platform_revision" == "$PLATFORM_SOURCE_SHA"', self.workflow)
        self.assertIn('gateway_revision" == "$GATEWAY_SOURCE_SHA"', self.workflow)
        self.assertNotIn('platform_revision" != "$RELEASE_SHA"', self.workflow)

    def test_ephemeral_env_separates_release_and_component_sources(self) -> None:
        self.assertIn("OTERYN_RELEASE_SHA=$release_sha", self.workflow)
        self.assertIn("PLATFORM_SOURCE_SHA=$platform_source_sha", self.workflow)
        self.assertIn("GATEWAY_SOURCE_SHA=$gateway_source_sha", self.workflow)
        self.assertIn("GATEWAY_VERSION=$gateway_version", self.workflow)
        self.assertIn("GATEWAY_VERSION=sha-REQUIRED_EXACT_40_CHAR_GATEWAY_SOURCE_SHA", self.runtime_env)

    def test_release_state_persists_component_source_sha_and_immutable_digests(self) -> None:
        self.assertIn("write_state_value PLATFORM_SOURCE_SHA", self.release_state)
        self.assertIn("write_state_value GATEWAY_SOURCE_SHA", self.release_state)
        self.assertIn("is_immutable_image", self.release_state)
        self.assertIn("write OUT RELEASE_SHA PLATFORM_SOURCE_SHA GATEWAY_SOURCE_SHA", self.release_state)

    def test_runtime_release_guard_validates_each_component_independently(self) -> None:
        self.assertIn("_oteryn_verify_component_image_source", self.lib)
        self.assertIn('"$PLATFORM_IMAGE" "$PLATFORM_SOURCE_SHA" Platform', self.lib)
        self.assertIn('"$GATEWAY_IMAGE" "$GATEWAY_SOURCE_SHA" Gateway', self.lib)
        self.assertIn('GATEWAY_VERSION disagrees with Gateway component source SHA', self.lib)
        release_fn = self.lib.split("_oteryn_release_sha()", 1)[1].split("_oteryn_contract_from_platform_image()", 1)[0]
        self.assertNotIn("_oteryn_release_sha_for_images", release_fn)

    def test_candidate_resume_and_finalize_include_component_provenance(self) -> None:
        self.assertIn('"$RELEASE_SHA" "$PLATFORM_SOURCE_SHA" "$GATEWAY_SOURCE_SHA"', self.lib)
        self.assertIn("Candidate resume rejected: component source identity drifted.", self.lib)
        self.assertIn('"$RELEASE_SHA" "$PLATFORM_SOURCE_SHA" "$GATEWAY_SOURCE_SHA"', self.deploy_script)
        self.assertIn("Previous candidate finalization rejected: candidate does not accept the proven schema.", self.deploy_script)

    def test_rollback_uses_persisted_component_provenance(self) -> None:
        self.assertIn("PLATFORM_SOURCE_SHA GATEWAY_SOURCE_SHA", self.rollback)
        self.assertIn('GATEWAY_VERSION="sha-$GATEWAY_SOURCE_SHA"', self.rollback)
        self.assertIn('_oteryn_verify_component_image_source "$PLATFORM_IMAGE" "$PLATFORM_SOURCE_SHA" Platform', self.rollback)
        self.assertIn('_oteryn_verify_component_image_source "$GATEWAY_IMAGE" "$GATEWAY_SOURCE_SHA" Gateway', self.rollback)
        self.assertNotIn("last_good_revision=", self.rollback)

    def test_production_preflight_uses_persisted_release_and_component_provenance(self) -> None:
        self.assertIn("current-release.env", self.preflight)
        self.assertIn("PLATFORM_SOURCE_SHA", self.preflight)
        self.assertIn("GATEWAY_SOURCE_SHA", self.preflight)
        self.assertIn("platform_source_sha", self.preflight)
        self.assertIn("gateway_source_sha", self.preflight)
        self.assertNotIn("Gateway OCI revision does not match the Platform release SHA", self.preflight)

    def test_character_bazaar_keeps_owner_neutral_platform_coordinates_and_canary_pin(self) -> None:
        self.assertIn("repository-ghcr-image.sh oteryn-platform", self.character_workflow)
        self.assertIn("repository-ghcr-image.sh oteryn-game-gateway", self.character_workflow)
        self.assertIn("ghcr.io/blakinio/canary@sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f", self.character_workflow)

    def test_runner_registration_remains_repository_scoped_and_restart_safe(self) -> None:
        self.assertIn("RUNNER_URL=REQUIRED_REPOSITORY_URL", self.runner_env)
        self.assertIn("RUNNER_GHCR_OWNER=REQUIRED_REPOSITORY_OWNER_LOWERCASE", self.runner_env)
        self.assertIn("image: ${RUNNER_IMAGE:?RUNNER_IMAGE must be set explicitly}", self.runner_compose)
        self.assertIn("if [[ ! -f .runner ]]; then", self.runner_entrypoint)
        self.assertIn("--no-default-labels", self.runner_entrypoint)

    def test_preflight_keeps_current_owner_exact_digest_coordinates(self) -> None:
        self.assertIn("repository-ghcr-image.sh", self.preflight)
        self.assertIn('platform_prefix="${platform_repo}@sha256:"', self.preflight)
        self.assertIn('gateway_prefix="${gateway_repo}@sha256:"', self.preflight)
        self.assertIn("OTERYN_GHCR_OWNER: ${{ github.repository_owner }}", self.preflight_workflow)


if __name__ == "__main__":
    unittest.main(verbosity=2)
