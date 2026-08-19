#!/usr/bin/env python3
"""Regression tests for current Platform agent policy consistency."""
from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

from policy_consistency import CHECKED_PATHS, REPO_ROOT, validate_policy


class PolicyConsistencyTests(unittest.TestCase):
    def fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path]:
        temp = tempfile.TemporaryDirectory()
        root = Path(temp.name)
        for relative in CHECKED_PATHS:
            source = REPO_ROOT / relative
            target = root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
        return temp, root

    def replace(self, root: Path, relative: str, old: str, new: str) -> None:
        path = root / relative
        text = path.read_text(encoding="utf-8")
        self.assertIn(old, text)
        path.write_text(text.replace(old, new, 1), encoding="utf-8")

    def findings(self, root: Path) -> str:
        return "\n".join(validate_policy(root))

    def test_current_repository_policy_is_consistent(self) -> None:
        self.assertEqual([], validate_policy(REPO_ROOT))

    def test_same_directory_override_fails_closed(self) -> None:
        temp, root = self.fixture(); self.addCleanup(temp.cleanup)
        (root / "AGENTS.override.md").write_text("replacement\n", encoding="utf-8")
        self.assertIn("replacement semantics", self.findings(root))

    def test_missing_issue_lifecycle_marker_fails_closed(self) -> None:
        temp, root = self.fixture(); self.addCleanup(temp.cleanup)
        self.replace(root, "AGENTS.md", "GitHub Issue is authoritative", "Issue tracking is optional")
        self.assertIn("GitHub Issue is authoritative", self.findings(root))

    def test_legacy_coordinate_requires_historical_context(self) -> None:
        temp, root = self.fixture(); self.addCleanup(temp.cleanup)
        with (root / "AGENTS.md").open("a", encoding="utf-8") as handle:
            handle.write("\nWrite to blakinio/Oteryn-Platform for current tasks.\n")
        self.assertIn("outside an explicit historical statement", self.findings(root))

    def test_status_drift_fails_closed(self) -> None:
        temp, root = self.fixture(); self.addCleanup(temp.cleanup)
        self.replace(root, "docs/agents/AGENTS.md", "investigating | implementing | validating | ready | waiting | blocked | completed", "investigating | implementing | validating | ready | blocked | completed")
        self.assertIn("checkpoint task statuses drift", self.findings(root))

    def test_terminal_result_drift_fails_closed(self) -> None:
        temp, root = self.fixture(); self.addCleanup(temp.cleanup)
        self.replace(root, "docs/agents/AGENTS.md", "DONE | WAITING | BLOCKED | ROTATE", "DONE | WAITING | BLOCKED")
        self.assertIn("terminal invocation results drift", self.findings(root))

    def test_contract_status_drift_fails_closed(self) -> None:
        temp, root = self.fixture(); self.addCleanup(temp.cleanup)
        path = root / "docs/agents/GOVERNANCE_CONTRACT.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        data["shared_checkpoint_contract"]["allowed_statuses"].remove("waiting")
        path.write_text(json.dumps(data), encoding="utf-8")
        self.assertIn("checkpoint task statuses drift", self.findings(root))

    def test_lane_repository_drift_fails_closed(self) -> None:
        temp, root = self.fixture(); self.addCleanup(temp.cleanup)
        path = root / "docs/agents/PROJECT_LANES.json"
        data = json.loads(path.read_text(encoding="utf-8")); data["repository"] = "blakinio/Oteryn-Platform"
        path.write_text(json.dumps(data), encoding="utf-8")
        self.assertIn("repository drift", self.findings(root))

    def test_missing_delivery_marker_fails_closed(self) -> None:
        temp, root = self.fixture(); self.addCleanup(temp.cleanup)
        self.replace(root, "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md", "## Terminal closeout", "## Closeout")
        self.assertIn("Terminal closeout", self.findings(root))


if __name__ == "__main__":
    unittest.main(verbosity=2)