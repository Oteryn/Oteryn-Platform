#!/usr/bin/env python3
from __future__ import annotations

import copy
import hashlib
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import historical_branch_audit as audit  # noqa: E402


def relation(*, incorporated: bool, proof: str = "UNIQUE_HISTORY_REMAINS") -> dict:
    return {
        "incorporated": incorporated,
        "proof": proof,
    }


class DispositionTests(unittest.TestCase):
    def test_protected_wins(self) -> None:
        disposition, _ = audit.decide_disposition(
            branch="main",
            protected=True,
            open_pr_numbers=[],
            active_claims=[],
            relation=relation(incorporated=True, proof="ANCESTOR_OF_MAIN"),
        )
        self.assertEqual(disposition, "PROTECTED")

    def test_open_pr_wins_over_incorporated_history(self) -> None:
        disposition, _ = audit.decide_disposition(
            branch="feature/live",
            protected=False,
            open_pr_numbers=[42],
            active_claims=[],
            relation=relation(incorporated=True, proof="ANCESTOR_OF_MAIN"),
        )
        self.assertEqual(disposition, "OPEN_PR")

    def test_active_claim_wins_over_incorporated_history(self) -> None:
        disposition, _ = audit.decide_disposition(
            branch="feature/live",
            protected=False,
            open_pr_numbers=[],
            active_claims=["docs/agents/tasks/active/task.md"],
            relation=relation(incorporated=True, proof="ANCESTOR_OF_MAIN"),
        )
        self.assertEqual(disposition, "RETAIN")

    def test_incorporated_history_is_delete_even_for_old_backup_name(self) -> None:
        disposition, reason = audit.decide_disposition(
            branch="backup/old-snapshot",
            protected=False,
            open_pr_numbers=[],
            active_claims=[],
            relation=relation(incorporated=True, proof="PATCH_EQUIVALENT_TO_MAIN"),
        )
        self.assertEqual(disposition, "DELETE")
        self.assertIn("PATCH_EQUIVALENT_TO_MAIN", reason)

    def test_unique_backup_history_is_recovery(self) -> None:
        disposition, _ = audit.decide_disposition(
            branch="backup/unique-snapshot",
            protected=False,
            open_pr_numbers=[],
            active_claims=[],
            relation=relation(incorporated=False),
        )
        self.assertEqual(disposition, "RECOVERY")

    def test_unique_generic_history_is_retained(self) -> None:
        disposition, _ = audit.decide_disposition(
            branch="tmp/unknown",
            protected=False,
            open_pr_numbers=[],
            active_claims=[],
            relation=relation(incorporated=False),
        )
        self.assertEqual(disposition, "RETAIN")


class ApprovalTests(unittest.TestCase):
    def setUp(self) -> None:
        self.entries = [
            {"branch": "old/a", "head_sha": "a" * 40},
            {"branch": "old/b", "head_sha": "b" * 40},
        ]
        self.impl = hashlib.sha256(b"implementation").hexdigest()
        self.approval = {
            "apply_on_main": True,
            "candidate_count": 2,
            "confirmation": audit.CONFIRMATION,
            "entries": copy.deepcopy(self.entries),
            "entries_sha256": audit.entries_sha256(self.entries),
            "implementation_sha256": self.impl,
            "issue": audit.ISSUE_NUMBER,
            "review_summary": "reviewed exact redundant historical refs",
            "reviewed_at": "2026-08-14T20:00:00Z",
            "reviewed_by": "repository owner instruction executed by remediation agent",
            "schema_version": audit.SCHEMA_VERSION,
            "source_artifact": "run 1 artifact 2",
        }
        self.manifest = {
            "confirmation": audit.CONFIRMATION,
            "entries": copy.deepcopy(self.entries),
            "implementation_sha256": self.impl,
            "issue": audit.ISSUE_NUMBER,
            "schema_version": audit.SCHEMA_VERSION,
        }

    def test_inventory_approval_binds_exact_entries_and_implementation(self) -> None:
        result = audit.validate_inventory_approval(self.approval, self.manifest)
        self.assertEqual(result["result"], "PASS")
        self.assertEqual(result["candidate_count"], 2)

    def test_inventory_approval_rejects_entry_drift(self) -> None:
        drifted = copy.deepcopy(self.manifest)
        drifted["entries"][0]["head_sha"] = "c" * 40
        with self.assertRaises(audit.ValidationError):
            audit.validate_inventory_approval(self.approval, drifted)

    def test_apply_accepts_already_absent_approved_entry(self) -> None:
        current = copy.deepcopy(self.manifest)
        current["entries"] = [copy.deepcopy(self.entries[0])]
        present, absent = audit.validate_apply_candidate_set(
            approval=self.approval,
            current_manifest=current,
            current_branches={"old/a": "a" * 40, "main": "f" * 40},
        )
        self.assertEqual(present, [self.entries[0]])
        self.assertEqual(absent, [self.entries[1]])

    def test_apply_rejects_new_unreviewed_delete_candidate(self) -> None:
        current = copy.deepcopy(self.manifest)
        current["entries"].append({"branch": "old/c", "head_sha": "c" * 40})
        with self.assertRaisesRegex(audit.ValidationError, "unreviewed deletion"):
            audit.validate_apply_candidate_set(
                approval=self.approval,
                current_manifest=current,
                current_branches={
                    "old/a": "a" * 40,
                    "old/b": "b" * 40,
                    "old/c": "c" * 40,
                },
            )

    def test_apply_rejects_present_approved_branch_that_is_no_longer_delete(self) -> None:
        current = copy.deepcopy(self.manifest)
        current["entries"] = [copy.deepcopy(self.entries[0])]
        with self.assertRaisesRegex(audit.ValidationError, "no longer classifies DELETE"):
            audit.validate_apply_candidate_set(
                approval=self.approval,
                current_manifest=current,
                current_branches={"old/a": "a" * 40, "old/b": "b" * 40},
            )

    def test_apply_rejects_sha_drift(self) -> None:
        with self.assertRaisesRegex(audit.ValidationError, "SHA drift"):
            audit.validate_apply_candidate_set(
                approval=self.approval,
                current_manifest=self.manifest,
                current_branches={"old/a": "c" * 40, "old/b": "b" * 40},
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
