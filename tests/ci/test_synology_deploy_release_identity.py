#!/usr/bin/env python3

from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/deploy-synology-staging.yml"
PREFLIGHT = ROOT / "deploy/synology/scripts/production-target-preflight.sh"


class SynologyDeployReleaseIdentityContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")
        cls.preflight = PREFLIGHT.read_text(encoding="utf-8")

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
        self.assertNotIn("PLATFORM_IMAGE=ghcr.io/blakinio/oteryn-platform:$RELEASE_TAG", self.workflow)
        self.assertNotIn("GATEWAY_IMAGE=ghcr.io/blakinio/oteryn-game-gateway:$RELEASE_TAG", self.workflow)

    def test_digest_resolution_occurs_before_environment_is_written(self) -> None:
        self.assertLess(
            self.workflow.index("Resolve immutable runtime image digests"),
            self.workflow.index("Write ephemeral staging environment"),
        )

    def test_production_target_preflight_accepts_exact_digest_runtime_refs(self) -> None:
        self.assertIn("oteryn-platform@sha256:", self.preflight)
        self.assertIn("oteryn-game-gateway@sha256:", self.preflight)
        self.assertIn('fail "Platform is not deployed by immutable digest"', self.preflight)
        self.assertIn('fail "Gateway is not deployed by immutable digest"', self.preflight)
        self.assertNotIn("Platform is not deployed from an exact sha-<40 hex> tag", self.preflight)
        self.assertNotIn("Gateway release tag does not match the exact Platform release SHA", self.preflight)

    def test_preflight_recovers_release_sha_from_immutable_oci_revision_metadata(self) -> None:
        self.assertIn('docker image inspect --format', self.preflight)
        self.assertIn('org.opencontainers.image.revision', self.preflight)
        self.assertIn('[[ "$image_revision" =~ ^[a-f0-9]{40}$ ]]', self.preflight)
        self.assertIn('elif [[ "$image_revision" != "$deployed_release_sha" ]]', self.preflight)
        self.assertIn('fail "Gateway OCI revision does not match the Platform release SHA"', self.preflight)
        self.assertLess(
            self.preflight.index('[[ "$platform_image" =~ ^ghcr\\.io/blakinio/oteryn-platform@sha256:'),
            self.preflight.index('org.opencontainers.image.revision'),
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
