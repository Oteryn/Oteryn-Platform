#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("branch_lifecycle.py")
SPEC = importlib.util.spec_from_file_location("branch_lifecycle", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
branch_lifecycle = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = branch_lifecycle
SPEC.loader.exec_module(branch_lifecycle)


class BranchLifecycleTest(unittest.TestCase):
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
    def pull(
        number: int,
        branch: str,
        sha: str,
        *,
        state: str = "closed",
        merged_at: str | None = "2026-08-01T10:00:00Z",
    ) -> dict:
        return {
            "head": {
                "ref": branch,
                "repo": {"full_name": "blakinio/Oteryn-Platform"},
                "sha": sha,
            },
            "html_url": f"https://github.com/blakinio/Oteryn-Platform/pull/{number}",
            "merged_at": merged_at,
            "number": number,
            "state": state,
        }

    def snapshot(self) -> dict:
        return {
            "active_task_branches": ["task/live"],
            "branches": [
                {"name": "main", "protected": True, "sha": "m" * 40},
                {"name": "task/live", "protected": False, "sha": "a" * 40},
                {"name": "feature/open", "protected": False, "sha": "b" * 40},
                {"name": "docs/merged", "protected": False, "sha": "c" * 40},
                {"name": "feature/moved", "protected": False, "sha": "d" * 40},
                {"name": "recovery/manual", "protected": False, "sha": "e" * 40},
                {"name": "unknown", "protected": False, "sha": "f" * 40},
                {"name": "repair/issue-777", "protected": False, "sha": "1" * 40},
                {"name": "protected-extra", "protected": True, "sha": "2" * 40},
            ],
            "default_branch": "main",
            "generated_at": "2026-08-06T07:00:00+00:00",
            "issue_states": {"777": "open"},
            "pulls": [
                self.pull(10, "feature/open", "b" * 40, state="open", merged_at=None),
                self.pull(11, "docs/merged", "c" * 40),
                self.pull(12, "feature/moved", "0" * 40),
            ],
            "repository": "blakinio/Oteryn-Platform",
        }

    def classify(self, snapshot: dict | None = None) -> dict:
        return branch_lifecycle.classify_snapshot(
            self.policy, snapshot or self.snapshot(), root=self.root
        )

    def test_valid_policy(self) -> None:
        exceptions = branch_lifecycle.validate_policy(self.policy, self.root)
        self.assertEqual(["main"], sorted(exceptions))

    def test_policy_forbids_glob_and_unprotected_exception(self) -> None:
        policy = copy.deepcopy(self.policy)
        policy["retention_exceptions"][0]["branch"] = "release/*"
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "globs are forbidden"):
            branch_lifecycle.validate_policy(policy, self.root)
        policy = copy.deepcopy(self.policy)
        policy["retention_exceptions"][0]["protected_required"] = False
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "must be true"):
            branch_lifecycle.validate_policy(policy, self.root)

    def test_classification_precedence_and_fail_closed_boundaries(self) -> None:
        report = self.classify()
        by_branch = {item["branch"]: item for item in report["branches"]}
        self.assertEqual("PROTECTED", by_branch["main"]["classification"])
        self.assertEqual("ACTIVE_CLAIM", by_branch["task/live"]["classification"])
        self.assertEqual("OPEN_PR", by_branch["feature/open"]["classification"])
        self.assertEqual("TERMINAL_MERGED", by_branch["docs/merged"]["classification"])
        self.assertTrue(by_branch["docs/merged"]["deletion_candidate"])
        self.assertEqual("UNMERGED_ORPHAN", by_branch["feature/moved"]["classification"])
        self.assertEqual("UNKNOWN", by_branch["recovery/manual"]["classification"])
        self.assertEqual("UNKNOWN", by_branch["unknown"]["classification"])
        self.assertEqual("ACTIVE_CLAIM", by_branch["repair/issue-777"]["classification"])
        self.assertEqual("PROTECTED", by_branch["protected-extra"]["classification"])
        self.assertEqual(1, report["deletion_candidate_count"])

    def test_exact_retention_exception_must_be_live_protected(self) -> None:
        policy = copy.deepcopy(self.policy)
        policy["retention_exceptions"].append(
            {
                "branch": "release/1.x",
                "classification": "RELEASE",
                "owner": "release owner",
                "protected_required": True,
                "purpose": "supported release line",
                "review_trigger": "release end of life",
            }
        )
        snapshot = self.snapshot()
        snapshot["branches"].append(
            {"name": "release/1.x", "protected": False, "sha": "3" * 40}
        )
        report = branch_lifecycle.classify_snapshot(policy, snapshot, root=self.root)
        entry = next(item for item in report["branches"] if item["branch"] == "release/1.x")
        self.assertEqual("UNKNOWN", entry["classification"])
        self.assertIn("requires protection", entry["evidence"][0])

    def test_foreign_repo_pull_is_not_authoritative(self) -> None:
        snapshot = self.snapshot()
        snapshot["branches"].append(
            {"name": "foreign", "protected": False, "sha": "4" * 40}
        )
        pull = self.pull(20, "foreign", "4" * 40)
        pull["head"]["repo"]["full_name"] = "someone/fork"
        snapshot["pulls"].append(pull)
        entry = next(
            item for item in self.classify(snapshot)["branches"] if item["branch"] == "foreign"
        )
        self.assertEqual("UNKNOWN", entry["classification"])

    def test_manifest_generation_is_inert_and_exact(self) -> None:
        report = self.classify()
        manifest = branch_lifecycle.make_manifest(report)
        self.assertFalse(manifest["apply_on_main"])
        self.assertEqual(branch_lifecycle.CONFIRMATION, manifest["confirmation"])
        self.assertEqual(
            [{
                "branch": "docs/merged",
                "head_sha": "c" * 40,
                "merged_at": "2026-08-01T10:00:00Z",
                "merged_pr_number": 11,
            }],
            manifest["entries"],
        )
        validated = branch_lifecycle.validate_manifest(
            manifest, report, require_apply=False
        )
        self.assertEqual("docs/merged", validated[0]["branch"])

    def test_manifest_rejects_live_evidence_drift(self) -> None:
        report = self.classify()
        manifest = branch_lifecycle.make_manifest(report)
        manifest["entries"][0]["head_sha"] = "9" * 40
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "evidence drift"):
            branch_lifecycle.validate_manifest(manifest, report, require_apply=False)

    def test_manifest_apply_requires_review_flag(self) -> None:
        report = self.classify()
        manifest = branch_lifecycle.make_manifest(report)
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "apply_on_main"):
            branch_lifecycle.validate_manifest(manifest, report, require_apply=True)
        manifest["apply_on_main"] = True
        manifest["entries"] = []
        self.assertEqual(
            [], branch_lifecycle.validate_manifest(manifest, report, require_apply=True)
        )

    def test_active_task_branch_parser(self) -> None:
        task = self.root / "docs/agents/tasks/active/task.md"
        task.write_text(
            "```yaml\nbranch: repair/issue-658\nlock_branch: repair/issue-659\n"
            "branch: null\nbranch: UNKNOWN\n```\n",
            encoding="utf-8",
        )
        self.assertEqual(
            {"repair/issue-658", "repair/issue-659"},
            branch_lifecycle.active_task_branches(self.root),
        )

    def test_canonical_json_loader_fails_closed(self) -> None:
        path = self.root / "policy.json"
        path.write_text(json.dumps(self.policy), encoding="utf-8")
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "sorted keys"):
            branch_lifecycle.load_json(path)
        path.write_text(branch_lifecycle.canonical_json(self.policy), encoding="utf-8")
        self.assertEqual(self.policy, branch_lifecycle.load_json(path))

    def test_apply_refuses_non_main_context_before_deletion(self) -> None:
        report = self.classify()
        manifest = branch_lifecycle.make_manifest(report)
        manifest["apply_on_main"] = True

        class NeverDelete:
            def delete_branch(self, branch: str) -> None:
                raise AssertionError("must not delete")

        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "push to main"):
            branch_lifecycle.apply_manifest(
                NeverDelete(),
                report,
                manifest,
                evidence_path=self.root / "evidence.json",
                event_name="pull_request",
                ref_name="feature",
            )


if __name__ == "__main__":
    unittest.main()
