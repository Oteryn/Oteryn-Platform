#!/usr/bin/env python3
from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import terminal_branch_cleanup as guarded


class FakeClient:
    repo = "blakinio/Oteryn-Platform"

    def __init__(self, body: str):
        self.body = body

    def get_branch(self, branch: str):
        return {"commit": {"sha": "b" * 40}, "protected": False}

    def open_pulls_for_branch(self, branch: str):
        return []

    def get_issue_state(self, issue_number: int):
        return "closed"

    def get_ref(self, branch: str):
        sha = "a" * 40 if branch == "main" else "b" * 40
        return {"object": {"sha": sha}}

    def request(self, method: str, path: str, expected=(200,)):
        return (
            {
                "state": "closed",
                "merged": False,
                "merged_at": None,
                "body": self.body,
                "head": {
                    "ref": "closed/exact",
                    "sha": "b" * 40,
                    "repo": {"full_name": self.repo},
                },
            },
            {},
        )


class TerminalBranchRetentionGuardTest(unittest.TestCase):
    def candidate_report(self):
        return {
            "branches": [
                {
                    "branch": "closed/exact",
                    "classification": "TERMINAL_CLOSED_UNMERGED",
                    "deletion_candidate": True,
                    "head_sha": "b" * 40,
                    "closed_unmerged_pr": {"number": 77, "closed_at": "2026-08-01T00:00:00Z"},
                    "evidence": ["legacy candidate"],
                }
            ],
            "counts": {"TERMINAL_CLOSED_UNMERGED": 1},
            "deletion_candidate_count": 1,
        }

    def snapshot(self, body: str):
        return {"pulls": [{"number": 77, "body": body}]}

    def policy_and_root(self):
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        adr = root / "docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md"
        adr.parent.mkdir(parents=True)
        adr.write_text("# ADR\n", encoding="utf-8")
        (root / "docs/agents/tasks/active").mkdir(parents=True)
        policy = {
            "accepted_adr": "docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md",
            "default_branch": "main",
            "deletion_requirements": {
                "active_claim_forbidden": True,
                "classification": "TERMINAL_MERGED",
                "exact_head_sha_required": True,
                "merged_pr_required": True,
                "open_pr_forbidden": True,
                "protected_forbidden": True,
                "reserved_name_without_exception_forbidden": True,
            },
            "issue": 658,
            "retention_exceptions": [
                {
                    "branch": "main",
                    "classification": "PROTECTED",
                    "owner": "repository owner",
                    "protected_required": True,
                    "purpose": "default branch",
                    "review_trigger": "default branch changes",
                }
            ],
            "schema_version": 1,
        }
        return temporary, root, policy

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
            guarded._event_disposition(
                "Branch-Disposition: delete\nBranch-Disposition: delete\n"
                "Branch-Disposition-Reason: duplicate\n"
            )
        with self.assertRaises(guarded.ValidationError):
            guarded._event_disposition("Branch-Disposition: retain\n")

    def test_predelete_revalidation_refuses_new_retain_marker(self):
        temporary, root, policy = self.policy_and_root()
        try:
            report = {
                "default_branch": "main",
                "default_branch_sha": "a" * 40,
                "repository": "blakinio/Oteryn-Platform",
            }
            entry = {
                "branch": "closed/exact",
                "closed_at": "2026-08-01T00:00:00Z",
                "closed_pr_number": 77,
                "head_sha": "b" * 40,
            }
            client = FakeClient(
                "Branch-Disposition: retain\n"
                "Branch-Disposition-Reason: operator retained before apply\n"
            )
            with self.assertRaisesRegex(guarded.ValidationError, "explicitly retains"):
                guarded._revalidate_delete_entry(
                    client, policy, report, entry, root=root
                )
        finally:
            temporary.cleanup()


if __name__ == "__main__":
    unittest.main()
