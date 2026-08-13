#!/usr/bin/env python3

from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/game-catalog-contract.yml"


class GameCatalogCrossRepositoryTriggerContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")

    def test_historical_pull_request_number_is_not_a_gate(self) -> None:
        self.assertNotIn("github.event.pull_request.number == 272", self.workflow)
        self.assertNotIn("pull_request.number == 272", self.workflow)

    def test_cross_repository_proof_has_explicit_manual_selector(self) -> None:
        self.assertIn("workflow_dispatch:", self.workflow)
        self.assertIn("run_cross_repository_staging:", self.workflow)
        self.assertIn("type: boolean", self.workflow)
        self.assertIn("default: false", self.workflow)
        self.assertIn(
            "if: github.event_name == 'workflow_dispatch' && inputs.run_cross_repository_staging",
            self.workflow,
        )

    def test_integration_test_remains_explicitly_invoked_and_triggered(self) -> None:
        self.assertIn("tests/Integration/GameCatalog/**", self.workflow)
        self.assertIn(
            "tests/Integration/GameCatalog/CrossRepositoryGameCatalogTest.php",
            self.workflow,
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
