#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

TOOLS_DIR = Path(__file__).parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))
SPEC = importlib.util.spec_from_file_location(
    "test_terminal_branch_cleanup_module", TOOLS_DIR / "test_terminal_branch_cleanup.py"
)
assert SPEC is not None and SPEC.loader is not None
fixture_module = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = fixture_module
SPEC.loader.exec_module(fixture_module)

import terminal_branch_approval as approval  # noqa: E402
import terminal_branch_cleanup as terminal  # noqa: E402


class TerminalBranchApprovalTest(fixture_module.TerminalBranchCleanupTest):
    def test_reviewed_digest_materializes_exact_runtime_manifest(self) -> None:
        report = self.classify()
        manifest = terminal.make_manifest(report)
        reviewed = {
            "apply_on_main": True,
            "candidate_count": len(manifest["entries"]),
            "confirmation": approval.CONFIRMATION,
            "entries_sha256": approval.entries_sha256(manifest["entries"]),
            "issue": approval.ISSUE_NUMBER,
            "policy_sha256": manifest["policy_sha256"],
            "review_summary": "reviewed exact candidate set",
            "reviewed_at": "2026-08-14T13:40:00Z",
            "reviewed_by": "repository owner",
            "schema_version": 1,
            "source_artifact": "fixture",
        }
        result = approval.validate_approval(
            reviewed, manifest, report, require_apply=True
        )
        self.assertEqual("PASS", result["result"])
        runtime = approval.materialize_runtime_manifest(reviewed, manifest, report)
        self.assertTrue(runtime["apply_on_main"])
        self.assertEqual(manifest["entries"], runtime["entries"])

    def test_reviewed_digest_rejects_candidate_drift(self) -> None:
        report = self.classify()
        manifest = terminal.make_manifest(report)
        reviewed = {
            "apply_on_main": True,
            "candidate_count": len(manifest["entries"]),
            "confirmation": approval.CONFIRMATION,
            "entries_sha256": "0" * 64,
            "issue": approval.ISSUE_NUMBER,
            "policy_sha256": manifest["policy_sha256"],
            "review_summary": "reviewed exact candidate set",
            "reviewed_at": "2026-08-14T13:40:00Z",
            "reviewed_by": "repository owner",
            "schema_version": 1,
            "source_artifact": "fixture",
        }
        with self.assertRaisesRegex(approval.ValidationError, "candidate entries drift"):
            approval.validate_approval(reviewed, manifest, report, require_apply=True)

    def test_reviewed_digest_rejects_policy_drift(self) -> None:
        report = self.classify()
        manifest = terminal.make_manifest(report)
        reviewed = {
            "apply_on_main": True,
            "candidate_count": len(manifest["entries"]),
            "confirmation": approval.CONFIRMATION,
            "entries_sha256": approval.entries_sha256(manifest["entries"]),
            "issue": approval.ISSUE_NUMBER,
            "policy_sha256": "f" * 64,
            "review_summary": "reviewed exact candidate set",
            "reviewed_at": "2026-08-14T13:40:00Z",
            "reviewed_by": "repository owner",
            "schema_version": 1,
            "source_artifact": "fixture",
        }
        with self.assertRaisesRegex(approval.ValidationError, "policy drift"):
            approval.validate_approval(reviewed, manifest, report, require_apply=True)


if __name__ == "__main__":
    unittest.main()
