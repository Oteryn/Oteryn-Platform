#!/usr/bin/env python3

from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RUNNER_ENTRYPOINT = ROOT / "deploy/synology/runner/entrypoint.sh"


class SynologyRunnerRegistrationLabelsContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.entrypoint = RUNNER_ENTRYPOINT.read_text(encoding="utf-8")

    def test_custom_staging_label_is_preserved(self) -> None:
        self.assertIn('RUNNER_LABELS="${RUNNER_LABELS:-oteryn-staging}"', self.entrypoint)
        self.assertIn('--labels "$RUNNER_LABELS"', self.entrypoint)

    def test_github_default_labels_are_not_suppressed(self) -> None:
        self.assertNotIn("--no-default-labels", self.entrypoint)


if __name__ == "__main__":
    unittest.main(verbosity=2)
