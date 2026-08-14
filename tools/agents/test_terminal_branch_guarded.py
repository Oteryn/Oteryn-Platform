#!/usr/bin/env python3
from __future__ import annotations

import unittest

import terminal_branch_guarded as guarded


class TerminalBranchRetentionGuardTest(unittest.TestCase):
    def candidate_report(self):
        return {
            "branches": [
                {
                    "branch": "repair/issue-123",
                    "classification": "TERMINAL_CLOSED_UNMERGED",
                    "deletion_candidate": True,
                    "head_sha": "a" * 40,
                    "closed_unmerged_pr": {"number": 77, "closed_at": "2026-08-01T00:00:00Z"},
                    "evidence": ["legacy candidate"],
                }
            ],
            "counts": {"TERMINAL_CLOSED_UNMERGED": 1},
            "deletion_candidate_count": 1,
        }

    def snapshot(self, body: str):
        return {"pulls": [{"number": 77, "body": body}]}

    def test_explicit_retain_is_not_a_historical_deletion_candidate(self):
        report = guarded.apply_historical_retention_guard(
            self.candidate_report(),
            self.snapshot(
                "Branch-Disposition: retain\n"
                "Branch-Disposition-Reason: durable recovery reference\n"
            ),
        )
        item = report["branches"][0]
        self.assertEqual("UNMERGED_ORPHAN", item["classification"])
        self.assertFalse(item["deletion_candidate"])
        self.assertEqual(0, report["deletion_candidate_count"])
        self.assertEqual("retain", item["closed_unmerged_pr"]["disposition"])

    def test_legacy_pr_without_metadata_remains_reviewable(self):
        report = guarded.apply_historical_retention_guard(
            self.candidate_report(), self.snapshot("legacy body")
        )
        item = report["branches"][0]
        self.assertEqual("TERMINAL_CLOSED_UNMERGED", item["classification"])
        self.assertTrue(item["deletion_candidate"])

    def test_explicit_delete_remains_reviewable(self):
        report = guarded.apply_historical_retention_guard(
            self.candidate_report(),
            self.snapshot(
                "Branch-Disposition: delete\n"
                "Branch-Disposition-Reason: superseded implementation\n"
            ),
        )
        self.assertTrue(report["branches"][0]["deletion_candidate"])

    def test_malformed_metadata_fails_closed(self):
        report = guarded.apply_historical_retention_guard(
            self.candidate_report(),
            self.snapshot("Branch-Disposition: retain\nBranch-Disposition: retain\n"),
        )
        item = report["branches"][0]
        self.assertFalse(item["deletion_candidate"])
        self.assertIn("malformed", item["evidence"][0])

    def test_event_parser_rejects_duplicate_or_missing_reason(self):
        with self.assertRaises(guarded.ValidationError):
            guarded.event_disposition(
                "Branch-Disposition: delete\nBranch-Disposition: delete\n"
                "Branch-Disposition-Reason: duplicate\n"
            )
        with self.assertRaises(guarded.ValidationError):
            guarded.event_disposition("Branch-Disposition: retain\n")


if __name__ == "__main__":
    unittest.main()
