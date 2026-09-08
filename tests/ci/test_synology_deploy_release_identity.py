#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
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
PLATFORM_IGNORE = ROOT / "deploy/synology/docker/platform.Dockerfile.dockerignore"
GATEWAY_IGNORE = ROOT / "deploy/synology/docker/gateway.Dockerfile.dockerignore"
RUNNER_IGNORE = ROOT / "deploy/synology/runner/Dockerfile.dockerignore"
RELEASE_STATE = ROOT / "deploy/synology/scripts/release-state.sh"
LEGACY_UPGRADE = ROOT / "deploy/synology/scripts/upgrade-legacy-release-state.sh"
LIB = ROOT / "deploy/synology/scripts/lib.sh"
DEPLOY_SCRIPT = ROOT / "deploy/synology/scripts/deploy.sh"
ROLLBACK = ROOT / "deploy/synology/scripts/rollback.sh"
HEAD_SHA = "a" * 40


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
        cls.platform_ignore = PLATFORM_IGNORE.read_text(encoding="utf-8")
        cls.gateway_ignore = GATEWAY_IGNORE.read_text(encoding="utf-8")
        cls.runner_ignore = RUNNER_IGNORE.read_text(encoding="utf-8")
        cls.release_state = RELEASE_STATE.read_text(encoding="utf-8")
        cls.legacy_upgrade = LEGACY_UPGRADE.read_text(encoding="utf-8")
        cls.lib = LIB.read_text(encoding="utf-8")
        cls.deploy_script = DEPLOY_SCRIPT.read_text(encoding="utf-8")
        cls.rollback = ROLLBACK.read_text(encoding="utf-8")

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
                HEAD_SHA,
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

    def run_classifier_range(self, event: str, cwd: Path, base: str, head: str) -> dict[str, object]:
        result = subprocess.run(
            [
                sys.executable,
                str(CLASSIFIER),
                "--event",
                event,
                "--base",
                base,
                "--head",
                head,
            ],
            cwd=cwd,
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
        return {str(entry["name"]): str(entry["mode"]) for entry in include if isinstance(entry, dict)}

    @staticmethod
    def git(cwd: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=cwd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if result.returncode != 0:
            raise AssertionError(result.stderr)
        return result.stdout.strip()

    def init_range_repo(self, root: Path) -> str:
        self.git(root, "init", "-q")
        self.git(root, "config", "user.email", "ci@example.invalid")
        self.git(root, "config", "user.name", "CI Contract")
        (root / "app").mkdir()
        (root / "services/game-gateway/internal").mkdir(parents=True)
        (root / "app/base.php").write_text("base\n")
        (root / "services/game-gateway/internal/base.go").write_text("package base\n")
        self.git(root, "add", ".")
        self.git(root, "commit", "-qm", "base")
        return self.git(root, "rev-parse", "HEAD")

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

    def test_classifier_platform_only_pr_builds_only_platform(self) -> None:
        result = self.run_classifier("pull_request", ["app/Http/Controllers/Example.php"])
        self.assertTrue(result["platform_changed"])
        self.assertTrue(result["has_image_builds"])
        self.assertEqual(self.matrix_modes(result), {"platform": "full"})

    def test_classifier_gateway_only_pr_builds_only_gateway(self) -> None:
        result = self.run_classifier("pull_request", ["services/game-gateway/cmd/game-gateway/main.go"])
        self.assertTrue(result["gateway_changed"])
        self.assertEqual(self.matrix_modes(result), {"game-gateway": "full"})

    def test_classifier_deploy_runner_only_pr_builds_only_runner(self) -> None:
        result = self.run_classifier("pull_request", ["deploy/synology/runner/entrypoint.sh"])
        self.assertTrue(result["deploy_runner_changed"])
        self.assertEqual(self.matrix_modes(result), {"deploy-runner": "full"})

    def test_runner_operator_package_change_does_not_rebuild_runner_image(self) -> None:
        result = self.run_classifier("pull_request", ["deploy/synology/runner/compose.yml"])
        self.assertFalse(result["deploy_runner_changed"])
        self.assertTrue(result["deployment_package_changed"])
        self.assertFalse(result["runtime_deployment_changed"])
        self.assertFalse(result["has_image_builds"])
        self.assertEqual(self.matrix_modes(result), {"noop": "noop"})

    def test_classifier_deployment_package_only_pr_allocates_zero_image_jobs(self) -> None:
        result = self.run_classifier("pull_request", ["deploy/synology/scripts/health-check.sh"])
        self.assertTrue(result["deployment_package_changed"])
        self.assertTrue(result["runtime_deployment_changed"])
        self.assertFalse(result["has_image_builds"])
        self.assertEqual(self.matrix_modes(result), {"noop": "noop"})
        self.assertIn("if: ${{ needs.classify.outputs.has_image_builds == 'true' }}", self.build_workflow)

    def test_classifier_docs_only_change_is_not_release_relevant(self) -> None:
        result = self.run_classifier("pull_request", ["README.md"])
        self.assertFalse(result["platform_changed"])
        self.assertFalse(result["gateway_changed"])
        self.assertFalse(result["deploy_runner_changed"])
        self.assertFalse(result["deployment_package_changed"])
        self.assertFalse(result["has_image_builds"])
        self.assertFalse(result["release_relevant"])
        self.assertNotIn("      - README.md", self.build_workflow)

    def test_classifier_mixed_pr_builds_only_affected_runtime_components(self) -> None:
        result = self.run_classifier(
            "pull_request",
            ["routes/web.php", "services/game-gateway/internal/server/server.go"],
        )
        self.assertEqual(
            self.matrix_modes(result),
            {"platform": "full", "game-gateway": "full"},
        )
        self.assertNotIn("deploy-runner", self.matrix_modes(result))

    def test_control_plane_change_fails_closed(self) -> None:
        path = ".github/workflows/build-synology-staging-images.yml"
        self.assertEqual(
            self.matrix_modes(self.run_classifier("pull_request", [path])),
            {"platform": "full", "game-gateway": "full", "deploy-runner": "full"},
        )
        main = self.run_classifier("push", [path])
        self.assertEqual(
            self.matrix_modes(main),
            {"platform": "full", "game-gateway": "full"},
        )
        self.assertFalse(main["build_deploy_runner"])
        self.assertTrue(main["release_relevant"])

    def test_platform_only_main_builds_one_and_releases(self) -> None:
        result = self.run_classifier("push", ["config/app.php"])
        self.assertEqual(self.matrix_modes(result), {"platform": "full"})
        self.assertTrue(result["release_relevant"])
        self.assertFalse(result["build_gateway"])

    def test_gateway_only_main_builds_one_and_releases(self) -> None:
        result = self.run_classifier("push", ["services/game-gateway/go.mod"])
        self.assertEqual(self.matrix_modes(result), {"game-gateway": "full"})
        self.assertTrue(result["release_relevant"])
        self.assertFalse(result["build_platform"])

    def test_deployment_package_only_main_builds_zero_and_reconciles(self) -> None:
        result = self.run_classifier("push", ["deploy/synology/nginx/internal.conf"])
        self.assertFalse(result["has_image_builds"])
        self.assertTrue(result["release_relevant"])
        self.assertEqual(self.matrix_modes(result), {"noop": "noop"})
        self.assertIn("needs.build.result == 'skipped'", self.build_workflow)

    def test_deploy_runner_only_main_never_publishes_or_deploys(self) -> None:
        result = self.run_classifier("push", ["deploy/synology/runner/entrypoint.sh"])
        self.assertFalse(result["build_deploy_runner"])
        self.assertFalse(result["has_image_builds"])
        self.assertFalse(result["release_relevant"])
        self.assertEqual(self.matrix_modes(result), {"noop": "noop"})

    def test_workflow_dispatch_exercises_all_component_build_paths_without_auto_deploy(self) -> None:
        result = self.run_classifier("workflow_dispatch", [])
        self.assertEqual(
            self.matrix_modes(result),
            {"platform": "full", "game-gateway": "full", "deploy-runner": "full"},
        )
        self.assertTrue(result["has_image_builds"])
        self.assertFalse(result["release_relevant"])

    def test_merge_group_classifier_supports_real_git_range(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            base = self.init_range_repo(root)
            (root / "services/game-gateway/internal/base.go").write_text("package changed\n")
            self.git(root, "add", ".")
            self.git(root, "commit", "-qm", "gateway")
            head = self.git(root, "rev-parse", "HEAD")
            result = self.run_classifier_range("merge_group", root, base, head)
        self.assertTrue(result["gateway_changed"])
        self.assertFalse(result["platform_changed"])
        self.assertEqual(self.matrix_modes(result), {"game-gateway": "full"})

    def test_push_classifier_handles_merge_commit_before_after_range(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            self.init_range_repo(root)
            main_branch = self.git(root, "branch", "--show-current")
            self.git(root, "checkout", "-qb", "feature")
            (root / "app/feature.php").write_text("feature\n")
            self.git(root, "add", ".")
            self.git(root, "commit", "-qm", "feature")
            self.git(root, "checkout", "-q", main_branch)
            (root / "README.md").write_text("docs\n")
            self.git(root, "add", ".")
            self.git(root, "commit", "-qm", "docs")
            before = self.git(root, "rev-parse", "HEAD")
            self.git(root, "merge", "--no-ff", "-qm", "merge feature", "feature")
            head = self.git(root, "rev-parse", "HEAD")
            result = self.run_classifier_range("push", root, before, head)
        self.assertTrue(result["platform_changed"])
        self.assertFalse(result["gateway_changed"])
        self.assertEqual(self.matrix_modes(result), {"platform": "full"})

    def test_component_source_shas_are_exact_head_for_new_builds(self) -> None:
        result = self.run_classifier("push", ["app/Example.php"])
        self.assertEqual(result["platform_source_sha"], HEAD_SHA)
        self.assertEqual(result["gateway_source_sha"], HEAD_SHA)
        self.assertEqual(result["runner_source_sha"], HEAD_SHA)

    def test_platform_runtime_inputs_are_bounded_and_complete(self) -> None:
        for path in (
            "artisan",
            "lang/pl.json",
            "storage/framework/.gitignore",
            "docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json",
            "docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md",
            "docs/architecture/SECURITY_ARCHITECTURE.md",
        ):
            self.assertEqual(
                self.matrix_modes(self.run_classifier("pull_request", [path])),
                {"platform": "full"},
                path,
            )
        self.assertNotIn("COPY . .", self.platform_dockerfile)
        self.assertIn("COPY docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json", self.platform_dockerfile)
        self.assertIn("COPY docs/architecture/adr/0013-wiki-administration.md", self.platform_dockerfile)

    def test_docker_contexts_physically_exclude_unrelated_repository_files(self) -> None:
        self.assertEqual(self.platform_ignore.splitlines()[0], "**")
        for required in (
            "!composer.json",
            "!app/**",
            "!deploy/synology/docker/platform-entrypoint.sh",
            "!docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json",
        ):
            self.assertIn(required, self.platform_ignore)
        for unrelated in ("!README.md", "!services/game-gateway/**", "!.github/**"):
            self.assertNotIn(unrelated, self.platform_ignore)

        self.assertEqual(self.gateway_ignore.splitlines()[0], "**")
        for required in (
            "!services/game-gateway/go.mod",
            "!services/game-gateway/cmd/**",
            "!services/game-gateway/internal/**",
        ):
            self.assertIn(required, self.gateway_ignore)
        self.assertNotIn("!services/game-gateway/**", self.gateway_ignore)
        self.assertNotIn("!app/**", self.gateway_ignore)

        self.assertEqual(self.runner_ignore.splitlines()[0], "**")
        self.assertIn("!deploy/synology/runner/entrypoint.sh", self.runner_ignore)
        self.assertIn("!deploy/synology/runner/test-entrypoint.sh", self.runner_ignore)
        self.assertNotIn("!deploy/synology/runner/compose.yml", self.runner_ignore)

    def test_full_build_labels_standard_oci_revision_with_component_source_sha(self) -> None:
        self.assertIn("org.opencontainers.image.revision=${{ matrix.source_sha }}", self.build_workflow)
        self.assertIn("io.oteryn.component.source-revision=${{ matrix.source_sha }}", self.build_workflow)
        self.assertIn('tags="${image}:source-${SOURCE_SHA}"', self.build_workflow)
        self.assertNotIn("mode == 'reuse'", self.build_workflow)
        self.assertNotIn("Reusable component provenance mismatch", self.build_workflow)

    def test_current_run_immutable_digest_not_cache_or_mutable_tag_drives_release(self) -> None:
        self.assertIn("DIGEST: ${{ steps.build-image.outputs.digest }}", self.build_workflow)
        self.assertIn('image_ref="${IMAGE}@${DIGEST}"', self.build_workflow)
        self.assertIn("Record immutable current-run component provenance", self.build_workflow)
        self.assertIn("Upload immutable current-run provenance", self.build_workflow)
        self.assertIn("Download immutable current-run component provenance", self.build_workflow)
        self.assertIn("actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a", self.build_workflow)
        self.assertIn("actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c", self.build_workflow)
        self.assertNotIn('docker pull "$SOURCE_REF"', self.build_workflow)

    def test_main_dispatch_marks_changed_components_and_leaves_reuse_inputs_empty(self) -> None:
        self.assertIn("PLATFORM_CHANGED: ${{ needs.classify.outputs.build_platform }}", self.build_workflow)
        self.assertIn("GATEWAY_CHANGED: ${{ needs.classify.outputs.build_gateway }}", self.build_workflow)
        self.assertIn("inputs[platform_changed]=${PLATFORM_CHANGED}", self.build_workflow)
        self.assertIn("inputs[gateway_changed]=${GATEWAY_CHANGED}", self.build_workflow)
        self.assertIn("inputs[platform_source_sha]=${PLATFORM_SOURCE_SHA}", self.build_workflow)
        self.assertIn("inputs[platform_image]=${PLATFORM_IMAGE}", self.build_workflow)
        self.assertIn("inputs[gateway_source_sha]=${GATEWAY_SOURCE_SHA}", self.build_workflow)
        self.assertIn("inputs[gateway_image]=${GATEWAY_IMAGE}", self.build_workflow)
        self.assertIn("Skipping superseded main deployment", self.build_workflow)

    def test_deploy_reuses_only_validated_persisted_current_release_provenance(self) -> None:
        for marker in (
            "release_sha:",
            "platform_changed:",
            "platform_source_sha:",
            "platform_image:",
            "gateway_changed:",
            "gateway_source_sha:",
            "gateway_image:",
        ):
            self.assertIn(marker, self.workflow)
        self.assertIn('current_file="$state_dir/current-release.env"', self.workflow)
        self.assertIn('release-state.sh validate "$current_file"', self.workflow)
        self.assertIn('git merge-base --is-ancestor "$persisted_release_sha" "$RELEASE_SHA"', self.workflow)
        self.assertIn('platform_source_sha="$persisted_platform_source_sha"', self.workflow)
        self.assertIn('platform_image="$persisted_platform_image"', self.workflow)
        self.assertIn('gateway_source_sha="$persisted_gateway_source_sha"', self.workflow)
        self.assertIn('gateway_image="$persisted_gateway_image"', self.workflow)
        self.assertIn("Component reuse rejected: current proven release state is missing.", self.workflow)
        self.assertNotIn(":source-${source_sha}", self.workflow)

    def test_deploy_requires_component_lineage_immutable_digest_and_component_oci_revision(self) -> None:
        self.assertIn('git merge-base --is-ancestor "$source_sha" "$RELEASE_SHA"', self.workflow)
        self.assertIn('[[ "$platform_image" =~ @sha256:[0-9a-f]{64}$ ]]', self.workflow)
        self.assertIn('[[ "$gateway_image" =~ @sha256:[0-9a-f]{64}$ ]]', self.workflow)
        self.assertIn('[[ "$platform_revision" == "$platform_source_sha" ]]', self.workflow)
        self.assertIn('[[ "$gateway_revision" == "$gateway_source_sha" ]]', self.workflow)
        self.assertNotIn('platform_revision" == "$RELEASE_SHA"', self.workflow)
        self.assertNotIn('gateway_revision" == "$RELEASE_SHA"', self.workflow)

    def test_ephemeral_env_separates_release_and_resolved_component_sources(self) -> None:
        self.assertIn("OTERYN_RELEASE_SHA=$release_sha", self.workflow)
        self.assertIn("PLATFORM_SOURCE_SHA=$platform_source_sha", self.workflow)
        self.assertIn("GATEWAY_SOURCE_SHA=$gateway_source_sha", self.workflow)
        self.assertIn("PLATFORM_SOURCE_SHA_RESOLVED", self.workflow)
        self.assertIn("GATEWAY_SOURCE_SHA_RESOLVED", self.workflow)
        self.assertIn("GATEWAY_VERSION=$gateway_version", self.workflow)
        self.assertIn("GATEWAY_VERSION=sha-REQUIRED_EXACT_40_CHAR_GATEWAY_SOURCE_SHA", self.runtime_env)

    def test_release_state_persists_component_source_sha_and_immutable_digests(self) -> None:
        self.assertIn("write_state_value PLATFORM_SOURCE_SHA", self.release_state)
        self.assertIn("write_state_value GATEWAY_SOURCE_SHA", self.release_state)
        self.assertIn("is_immutable_image", self.release_state)
        self.assertIn("write OUT RELEASE_SHA PLATFORM_SOURCE_SHA GATEWAY_SOURCE_SHA", self.release_state)
        self.assertIn("Invalid or missing PLATFORM_SOURCE_SHA", self.release_state)
        self.assertIn("Invalid or missing GATEWAY_SOURCE_SHA", self.release_state)

    def test_legacy_upgrade_covers_current_last_good_and_candidate_and_fails_closed(self) -> None:
        for name in ("current-release.env", "last-good-release.env", "candidate-release.env"):
            self.assertIn(name, self.legacy_upgrade)
        self.assertIn("partial/invalid component provenance", self.legacy_upgrade)
        self.assertIn("legacy Platform OCI revision does not match persisted RELEASE_SHA", self.legacy_upgrade)
        self.assertIn("legacy Gateway OCI revision does not match persisted RELEASE_SHA", self.legacy_upgrade)
        self.assertIn(".legacy-v1", self.legacy_upgrade)
        self.assertIn('"$release_sha" "$release_sha" "$release_sha"', self.legacy_upgrade)

    def test_runtime_release_guard_validates_each_component_independently(self) -> None:
        self.assertIn("_oteryn_verify_component_image_source", self.lib)
        self.assertIn('"$PLATFORM_IMAGE" "$PLATFORM_SOURCE_SHA" Platform', self.lib)
        self.assertIn('"$GATEWAY_IMAGE" "$GATEWAY_SOURCE_SHA" Gateway', self.lib)
        self.assertIn("GATEWAY_VERSION disagrees with Gateway component source SHA", self.lib)
        release_fn = self.lib.split("_oteryn_release_sha()", 1)[1].split("_oteryn_contract_from_platform_image()", 1)[0]
        self.assertNotIn("_oteryn_release_sha_for_images", release_fn)

    def test_candidate_resume_and_finalize_include_component_provenance(self) -> None:
        self.assertIn('"$RELEASE_SHA" "$PLATFORM_SOURCE_SHA" "$GATEWAY_SOURCE_SHA"', self.lib)
        self.assertIn("Candidate resume rejected: component source identity drifted.", self.lib)
        self.assertIn('"$RELEASE_SHA" "$PLATFORM_SOURCE_SHA" "$GATEWAY_SOURCE_SHA"', self.deploy_script)
        self.assertIn('_oteryn_verify_component_image_source "${candidate_state[3]}" "${candidate_state[1]}" Platform', self.deploy_script)
        self.assertIn('_oteryn_verify_component_image_source "${candidate_state[4]}" "${candidate_state[2]}" Gateway', self.deploy_script)
        self.assertIn("Previous candidate finalization rejected: candidate does not accept the proven schema.", self.deploy_script)

    def test_rollback_uses_exact_persisted_component_provenance(self) -> None:
        self.assertIn("PLATFORM_SOURCE_SHA GATEWAY_SOURCE_SHA", self.rollback)
        self.assertIn('GATEWAY_VERSION="sha-$GATEWAY_SOURCE_SHA"', self.rollback)
        self.assertIn('_oteryn_verify_component_image_source "$PLATFORM_IMAGE" "$PLATFORM_SOURCE_SHA" Platform', self.rollback)
        self.assertIn('_oteryn_verify_component_image_source "$GATEWAY_IMAGE" "$GATEWAY_SOURCE_SHA" Gateway', self.rollback)
        self.assertIn('cp "$last_good_file" "$state_dir/current-release.env.tmp"', self.rollback)
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
        self.assertIn(
            "ghcr.io/blakinio/canary@sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f",
            self.character_workflow,
        )

    def test_build_auto_deploy_keeps_approved_canary_immutable(self) -> None:
        self.assertIn(
            "CANARY_IMAGE_DIGEST: ghcr.io/blakinio/canary@sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f",
            self.build_workflow,
        )

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
