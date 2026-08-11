#!/usr/bin/env python3

from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/deploy-synology-staging.yml"


class SynologyDeployReleaseIdentityContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")

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


if __name__ == "__main__":
    unittest.main(verbosity=2)
