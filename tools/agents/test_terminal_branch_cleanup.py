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
SPEC = importlib.util.spec_from_file_location("terminal_branch_cleanup", TOOLS_DIR / "terminal_branch_cleanup.py")
assert SPEC is not None and SPEC.loader is not None
terminal = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = terminal
SPEC.loader.exec_module(terminal)


class TerminalBranchCleanupTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        adr = self.root / "docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md"
        adr.parent.mkdir(parents=True)
        adr.write_text("# ADR\n", encoding="utf-8")
        (self.root / "docs/agents/tasks/active").mkdir(parents=True)
        self.policy = {
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

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    @staticmethod
    def pull(number: int, branch: str, sha: str, *, state: str = "closed", merged_at=None, closed_at="2026-08-10T10:00:00Z") -> dict:
        return {
            "base": {"repo": {"full_name": "Oteryn/Oteryn-Platform"}},
            "body": "",
            "closed_at": closed_at,
            "head": {
                "ref": branch,
                "repo": {"full_name": "Oteryn/Oteryn-Platform"},
                "sha": sha,
            },
            "html_url": f"https://github.com/Oteryn/Oteryn-Platform/pull/{number}",
            "merged": merged_at is not None,
            "merged_at": merged_at,
            "number": number,
            "state": state,
        }

    def snapshot(self) -> dict:
        return {
            "active_task_branches": ["task/live"],
            "branches": [
                {"name": "main", "protected": True, "sha": "a" * 40},
                {"name": "closed/exact", "protected": False, "sha": "b" * 40},
                {"name": "closed/moved", "protected": False, "sha": "c" * 40},
                {"name": "feature/open", "protected": False, "sha": "d" * 40},
                {"name": "task/live", "protected": False, "sha": "e" * 40},
                {"name": "backup/history", "protected": False, "sha": "f" * 40},
                {"name": "docs/merged", "protected": False, "sha": "1" * 40},
            ],
            "default_branch": "main",
            "generated_at": "2026-08-14T13:00:00+00:00",
            "issue_states": {},
            "pulls": [
                self.pull(10, "closed/exact", "b" * 40),
                self.pull(11, "closed/moved", "0" * 40),
                self.pull(12, "feature/open", "d" * 40, state="open", closed_at=None),
                self.pull(13, "task/live", "e" * 40),
                self.pull(14, "backup/history", "f" * 40),
                self.pull(15, "docs/merged", "1" * 40, merged_at="2026-08-11T10:00:00Z"),
            ],
            "repository": "Oteryn/Oteryn-Platform",
        }

    def classify(self, snapshot=None) -> dict:
        return terminal.classify_snapshot(self.policy, snapshot or self.snapshot(), root=self.root)

    def test_only_exact_closed_unmerged_branch_becomes_candidate(self) -> None:
        report = self.classify()
        by_branch = {item["branch"]: item for item in report["branches"]}
        self.assertEqual("TERMINAL_CLOSED_UNMERGED", by_branch["closed/exact"]["classification"])
        self.assertTrue(by_branch["closed/exact"]["deletion_candidate"])
        self.assertEqual("UNMERGED_ORPHAN", by_branch["closed/moved"]["classification"])
        self.assertFalse(by_branch["closed/moved"]["deletion_candidate"])
        self.assertEqual("OPEN_PR", by_branch["feature/open"]["classification"])
        self.assertEqual("ACTIVE_CLAIM", by_branch["task/live"]["classification"])
        self.assertEqual("UNKNOWN", by_branch["backup/history"]["classification"])
        self.assertEqual("TERMINAL_MERGED", by_branch["docs/merged"]["classification"])
        self.assertEqual(1, report["deletion_candidate_count"])

    def test_multiple_exact_closed_prs_fail_closed(self) -> None:
        snapshot = self.snapshot()
        snapshot["pulls"].append(
            self.pull(
                16,
                "closed/exact",
                "b" * 40,
                closed_at="2026-08-11T10:00:00Z",
            )
        )
        report = self.classify(snapshot)
        entry = next(
            item for item in report["branches"] if item["branch"] == "closed/exact"
        )
        self.assertEqual("UNMERGED_ORPHAN", entry["classification"])
        self.assertFalse(entry["deletion_candidate"])
        self.assertIn("multiple closed unmerged", entry["evidence"][0])

    def test_foreign_closed_pr_is_not_authoritative(self) -> None:
        snapshot = self.snapshot()
        foreign = self.pull(20, "closed/exact", "b" * 40)
        foreign["head"]["repo"]["full_name"] = "someone/fork"
        snapshot["pulls"] = [foreign]
        report = self.classify(snapshot)
        entry = next(item for item in report["branches"] if item["branch"] == "closed/exact")
        self.assertEqual("UNKNOWN", entry["classification"])
        self.assertFalse(entry["deletion_candidate"])

    def test_manifest_is_exact_and_inert_until_reviewed(self) -> None:
        report = self.classify()
        manifest = terminal.make_manifest(report)
        self.assertFalse(manifest["apply_on_main"])
        self.assertEqual(terminal.CONFIRMATION, manifest["confirmation"])
        self.assertEqual([{
            "branch": "closed/exact",
            "closed_at": "2026-08-10T10:00:00Z",
            "closed_pr_number": 10,
            "head_sha": "b" * 40,
        }], manifest["entries"])
        validated = terminal.validate_manifest(manifest, report, require_apply=False)
        self.assertEqual("closed/exact", validated[0]["branch"])

    def test_manifest_rejects_evidence_drift(self) -> None:
        report = self.classify()
        manifest = terminal.make_manifest(report)
        manifest["entries"][0]["head_sha"] = "9" * 40
        with self.assertRaisesRegex(terminal.ValidationError, "evidence drift"):
            terminal.validate_manifest(manifest, report, require_apply=False)

    def test_delete_disposition_requires_reason_and_detects_conflict(self) -> None:
        self.assertEqual((None, None), terminal._event_disposition("nothing"))
        self.assertEqual(("retain", "needed for rollback"), terminal._event_disposition(
            "Branch-Disposition: retain\nBranch-Disposition-Reason: needed for rollback\n"
        ))
        self.assertEqual(("delete", "superseded by #99"), terminal._event_disposition(
            "Branch-Disposition: delete\nBranch-Disposition-Reason: superseded by #99\n"
        ))
        with self.assertRaisesRegex(terminal.ValidationError, "requires Branch-Disposition-Reason"):
            terminal._event_disposition("Branch-Disposition: delete\n")
        with self.assertRaisesRegex(terminal.ValidationError, "conflicting"):
            terminal._event_disposition(
                "Branch-Disposition: delete\nBranch-Disposition: retain\nBranch-Disposition-Reason: conflict\n"
            )

    def test_repair_branch_stays_active_while_issue_is_open(self) -> None:
        snapshot = self.snapshot()
        snapshot["branches"].append({"name": "repair/issue-999", "protected": False, "sha": "2" * 40})
        snapshot["pulls"].append(self.pull(30, "repair/issue-999", "2" * 40))
        snapshot["issue_states"]["999"] = "open"
        report = self.classify(snapshot)
        entry = next(item for item in report["branches"] if item["branch"] == "repair/issue-999")
        self.assertEqual("ACTIVE_CLAIM", entry["classification"])
        self.assertFalse(entry["deletion_candidate"])


if __name__ == "__main__":
    unittest.main()
