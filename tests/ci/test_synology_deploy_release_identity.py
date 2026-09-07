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
REUSE_DOCKERFILE = ROOT / "deploy/synology/docker/reuse-release.Dockerfile"


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
        cls.reuse_dockerfile = REUSE_DOCKERFILE.read_text(encoding="utf-8")

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

    def run_classifier(self, event: str, paths: list[str]) -> dict[str, object]:
        result = subprocess.run(
            [
                sys.executable,
                str(CLASSIFIER),
                "--event",
                event,
                "--head",
                "HEAD",
                "--paths-json",
                json.dumps(paths),
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
    def matrix_modes(result: dict[str, object]) -> dict[str, str]:
        matrix = result["matrix"]
        assert isinstance(matrix, dict)
        include = matrix["include"]
        assert isinstance(include, list)
        return {
            str(entry["name"]): str(entry["mode"])
            for entry in include
            if isinstance(entry, dict)
        }

    def test_out_of_scope_operational_assets_are_absent(self) -> None:
        forbidden = (
            ROOT / ".github/workflows/liquid20-synology-control.yml",
            ROOT / "deploy/liquid20",
        )
        existing = [path.relative_to(ROOT).as_posix() for path in forbidden if path.exists()]
        self.assertEqual(existing, [], f"out-of-scope operational assets present: {existing}")

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

    def test_platform_and_gateway_release_revision_still_matches_release_sha(self) -> None:
        self.assertIn("org.opencontainers.image.revision", self.workflow)
        self.assertIn('platform_revision" != "$RELEASE_SHA"', self.workflow)
        self.assertIn('gateway_revision" != "$RELEASE_SHA"', self.workflow)
        self.assertIn("org.opencontainers.image.revision=${{ steps.identity.outputs.release_sha }}", self.build_workflow)

    def test_component_source_provenance_is_separate_from_release_revision(self) -> None:
        marker = "io.oteryn.component.source-revision"
        self.assertIn(marker, self.build_workflow)
        self.assertIn(marker, self.reuse_dockerfile)
        self.assertIn('source-${SOURCE_SHA}', self.build_workflow)
        self.assertIn("BASE_IMAGE=${{ steps.reuse.outputs.base_ref }}", self.build_workflow)
        self.assertIn("Reusable component provenance mismatch", self.build_workflow)

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
        self.assertIn('"package": "oteryn-platform"', self.classifier)
        self.assertIn('"package": "oteryn-game-gateway"', self.classifier)
        self.assertIn('"package": "oteryn-deploy-runner"', self.classifier)
        self.assertIn('platform_repo="$(bash deploy/synology/scripts/repository-ghcr-image.sh oteryn-platform)"', self.workflow)
        self.assertIn('gateway_repo="$(bash deploy/synology/scripts/repository-ghcr-image.sh oteryn-game-gateway)"', self.workflow)

    def test_deployment_only_pr_validates_without_runtime_rebuild(self) -> None:
        result = self.run_classifier("pull_request", ["deploy/synology/scripts/health-check.sh"])
        self.assertTrue(result["deployment_changed"])
        self.assertTrue(result["runtime_deployment_changed"])
        self.assertEqual(self.matrix_modes(result), {"noop": "noop"})

    def test_deployment_only_main_reuses_both_runtime_components(self) -> None:
        result = self.run_classifier("push", ["deploy/synology/scripts/health-check.sh"])
        self.assertTrue(result["release_relevant"])
        self.assertEqual(
            self.matrix_modes(result),
            {"platform": "reuse", "game-gateway": "reuse"},
        )

    def test_platform_change_is_proportional_on_pr_and_main(self) -> None:
        pr = self.run_classifier("pull_request", ["app/Http/Controllers/Example.php"])
        self.assertEqual(self.matrix_modes(pr), {"platform": "full"})
        main = self.run_classifier("push", ["app/Http/Controllers/Example.php"])
        self.assertEqual(
            self.matrix_modes(main),
            {"platform": "full", "game-gateway": "reuse"},
        )

    def test_gateway_change_is_proportional_on_pr_and_main(self) -> None:
        pr = self.run_classifier("pull_request", ["services/game-gateway/cmd/game-gateway/main.go"])
        self.assertEqual(self.matrix_modes(pr), {"game-gateway": "full"})
        main = self.run_classifier("push", ["services/game-gateway/cmd/game-gateway/main.go"])
        self.assertEqual(
            self.matrix_modes(main),
            {"platform": "reuse", "game-gateway": "full"},
        )

    def test_runner_change_is_validated_on_pr_but_does_not_release_on_main(self) -> None:
        path = "deploy/synology/runner/entrypoint.sh"
        pr = self.run_classifier("pull_request", [path])
        self.assertEqual(self.matrix_modes(pr), {"deploy-runner": "full"})
        main = self.run_classifier("push", [path])
        self.assertTrue(main["deployment_changed"])
        self.assertFalse(main["runtime_deployment_changed"])
        self.assertFalse(main["release_relevant"])
        self.assertEqual(self.matrix_modes(main), {"noop": "noop"})

    def test_control_plane_change_fails_closed_to_full_builds(self) -> None:
        path = ".github/workflows/build-synology-staging-images.yml"
        pr = self.run_classifier("pull_request", [path])
        self.assertEqual(
            self.matrix_modes(pr),
            {"platform": "full", "game-gateway": "full", "deploy-runner": "full"},
        )
        main = self.run_classifier("push", [path])
        self.assertEqual(
            self.matrix_modes(main),
            {"platform": "full", "game-gateway": "full"},
        )

    def test_platform_runtime_input_model_covers_bounded_runtime_inputs(self) -> None:
        for path in (
            "artisan",
            "lang/pl.json",
            "storage/framework/.gitignore",
            "docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json",
            "docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md",
            "docs/architecture/SECURITY_ARCHITECTURE.md",
        ):
            result = self.run_classifier("pull_request", [path])
            self.assertEqual(self.matrix_modes(result), {"platform": "full"}, path)

    def test_platform_dockerfile_has_no_repository_wide_copy_invalidation(self) -> None:
        self.assertNotIn("COPY . .", self.platform_dockerfile)
        for marker in (
            "COPY artisan ./artisan",
            "COPY app/ ./app/",
            "COPY public/ ./public/",
            "COPY resources/ ./resources/",
            "COPY lang/ ./lang/",
            "COPY storage/ ./storage/",
            "COPY deploy/synology/release-contract.env ./deploy/synology/release-contract.env",
            "COPY docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json ./docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json",
            "COPY docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md ./docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md",
            "COPY docs/architecture/adr/0013-wiki-administration.md ./docs/architecture/adr/0013-wiki-administration.md",
        ):
            self.assertIn(marker, self.platform_dockerfile)

    def test_build_trigger_covers_bounded_wiki_runtime_provenance(self) -> None:
        push = self.build_workflow.split("  push:\n", 1)[1].split("  workflow_dispatch:\n", 1)[0]
        for marker in (
            "docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json",
            "docs/architecture/adr/0004-authoritative-platform-account-ownership.md",
            "docs/architecture/adr/0005-character-creation-product-policy.md",
            "docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md",
            "docs/contracts/OTCLIENT_GAME_AUTH_CONTRACT.md",
            "docs/agents/PROJECT_STATE.md",
            "docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md",
            "docs/architecture/SECURITY_ARCHITECTURE.md",
            "docs/architecture/adr/0013-wiki-administration.md",
        ):
            self.assertIn(marker, push)

    def test_reuse_release_image_is_metadata_only(self) -> None:
        self.assertIn("FROM ${BASE_IMAGE}", self.reuse_dockerfile)
        self.assertIn('org.opencontainers.image.revision="${RELEASE_SHA}"', self.reuse_dockerfile)
        self.assertIn('io.oteryn.component.source-revision="${COMPONENT_SOURCE_SHA}"', self.reuse_dockerfile)
        self.assertNotIn("COPY ", self.reuse_dockerfile)
        self.assertNotIn("RUN ", self.reuse_dockerfile)

    def test_build_workflow_uses_dynamic_component_matrix_and_safe_fallback(self) -> None:
        build_push = self.build_workflow.split("  push:\n", 1)[1].split("  workflow_dispatch:\n", 1)[0]
        self.assertIn("deploy/synology/**", build_push)
        self.assertIn("scripts/ci/classify_synology_builds.py", build_push)
        self.assertIn("lang/**", build_push)
        self.assertIn("storage/**", build_push)
        self.assertIn("matrix: ${{ fromJson(needs.classify.outputs.matrix) }}", self.build_workflow)
        self.assertIn("falling back to a full build", self.build_workflow)
        self.assertIn("Build and publish metadata-only reused release image", self.build_workflow)
        self.assertIn("Build changed component or safe fallback", self.build_workflow)

    def test_character_bazaar_uses_current_owner_for_platform_images_but_preserves_canary_pin(self) -> None:
        self.assertIn("repository-ghcr-image.sh oteryn-platform", self.character_workflow)
        self.assertIn("repository-ghcr-image.sh oteryn-game-gateway", self.character_workflow)
        self.assertIn(
            "ghcr.io/blakinio/canary@sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f",
            self.character_workflow,
        )
        self.assertIn("contains(github.event.head_commit.message, '[character-bazaar-staging]')", self.character_workflow)

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

    def test_runner_registration_isolated_to_repository_scoped_custom_label(self) -> None:
        self.assertIn('RUNNER_LABELS="${RUNNER_LABELS:-oteryn-staging}"', self.runner_entrypoint)
        self.assertIn('--labels "$RUNNER_LABELS"', self.runner_entrypoint)
        self.assertIn("--no-default-labels", self.runner_entrypoint)

    def test_runtime_example_is_owner_neutral(self) -> None:
        self.assertIn("OTERYN_GHCR_OWNER=REQUIRED_REPOSITORY_OWNER_LOWERCASE", self.runtime_env)
        self.assertIn("ghcr.io/${OTERYN_GHCR_OWNER}/oteryn-platform:main", self.runtime_env)
        self.assertIn("ghcr.io/${OTERYN_GHCR_OWNER}/oteryn-game-gateway:main", self.runtime_env)

    def test_transfer_sensitive_platform_paths_do_not_hardcode_old_owner_coordinates(self) -> None:
        texts = {
            "build": self.build_workflow,
            "deploy": self.workflow,
            "character": self.character_workflow,
            "preflight": self.preflight,
            "runtime-env": self.runtime_env,
            "runner-env": self.runner_env,
            "runner-compose": self.runner_compose,
            "runner-entrypoint": self.runner_entrypoint,
            "classifier": self.classifier,
            "reuse-dockerfile": self.reuse_dockerfile,
        }
        forbidden = (
            "ghcr.io/blakinio/oteryn-platform",
            "ghcr.io/blakinio/oteryn-game-gateway",
            "ghcr.io/blakinio/oteryn-deploy-runner",
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
