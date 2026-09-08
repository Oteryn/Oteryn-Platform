#!/usr/bin/env python3

from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CLASSIFIER = ROOT / "scripts/ci/classify_synology_builds.py"
GATEWAY_DOCKERFILE = ROOT / "deploy/synology/docker/gateway.Dockerfile"
GATEWAY_IGNORE = ROOT / "deploy/synology/docker/gateway.Dockerfile.dockerignore"
BUILD_WORKFLOW = ROOT / ".github/workflows/build-synology-staging-images.yml"
HEAD_SHA = "a" * 40


class SynologyGatewayBuildInputsContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.dockerfile = GATEWAY_DOCKERFILE.read_text(encoding="utf-8")
        cls.dockerignore = GATEWAY_IGNORE.read_text(encoding="utf-8")
        cls.workflow = BUILD_WORKFLOW.read_text(encoding="utf-8")

    def classify(self, path: str) -> dict[str, object]:
        result = subprocess.run(
            [
                sys.executable,
                str(CLASSIFIER),
                "--event",
                "pull_request",
                "--head",
                HEAD_SHA,
                "--paths-json",
                json.dumps([path]),
            ],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def test_gateway_runtime_sources_are_real_image_inputs(self) -> None:
        for path in (
            "services/game-gateway/go.mod",
            "services/game-gateway/cmd/game-gateway/main.go",
            "services/game-gateway/internal/server/server.go",
        ):
            with self.subTest(path=path):
                result = self.classify(path)
                self.assertTrue(result["gateway_changed"])
                self.assertTrue(result["has_image_builds"])
                include = result["matrix"]["include"]
                self.assertEqual([entry["name"] for entry in include], ["game-gateway"])

    def test_gateway_docs_and_unrelated_service_dockerfile_are_not_synology_image_inputs(self) -> None:
        for path in (
            "services/game-gateway/README.md",
            "services/game-gateway/Dockerfile",
        ):
            with self.subTest(path=path):
                result = self.classify(path)
                self.assertFalse(result["gateway_changed"])
                self.assertFalse(result["has_image_builds"])

    def test_gateway_dockerfile_copies_only_build_inputs(self) -> None:
        self.assertNotIn("COPY services/game-gateway/ ./", self.dockerfile)
        self.assertIn("COPY services/game-gateway/go.mod ./", self.dockerfile)
        self.assertIn("COPY services/game-gateway/cmd/ ./cmd/", self.dockerfile)
        self.assertIn("COPY services/game-gateway/internal/ ./internal/", self.dockerfile)

    def test_gateway_context_excludes_non_runtime_files(self) -> None:
        self.assertEqual(self.dockerignore.splitlines()[0], "**")
        for required in (
            "!services/game-gateway/go.mod",
            "!services/game-gateway/cmd/**",
            "!services/game-gateway/internal/**",
        ):
            self.assertIn(required, self.dockerignore)
        self.assertNotIn("!services/game-gateway/**", self.dockerignore)
        self.assertNotIn("!services/game-gateway/README.md", self.dockerignore)
        self.assertNotIn("!services/game-gateway/Dockerfile", self.dockerignore)

    def test_workflow_trigger_matches_truthful_gateway_inputs(self) -> None:
        self.assertNotIn("      - services/game-gateway/**", self.workflow)
        self.assertEqual(self.workflow.count("      - services/game-gateway/go.mod"), 2)
        self.assertEqual(self.workflow.count("      - services/game-gateway/cmd/**"), 2)
        self.assertEqual(self.workflow.count("      - services/game-gateway/internal/**"), 2)

    def test_provenance_fan_in_initializes_locals_before_expansion(self) -> None:
        self.assertIn('local name="$1"\n            local required="$2"\n            local file="artifacts/synology/${name}.env"', self.workflow)
        self.assertNotIn('local name="$1" required="$2" file="artifacts/synology/${name}.env"', self.workflow)


if __name__ == "__main__":
    unittest.main(verbosity=2)
