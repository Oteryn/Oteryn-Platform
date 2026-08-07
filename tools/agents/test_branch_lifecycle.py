#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

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

    def apply_fixture(
        self, report: dict | None = None, *, branch: str = "docs/merged"
    ) -> tuple[object, dict]:
        report = report or self.classify()
        manifest = branch_lifecycle.make_manifest(report)
        manifest["apply_on_main"] = True
        manifest["entries"] = [
            entry for entry in manifest["entries"] if entry["branch"] == branch
        ]
        self.assertEqual(1, len(manifest["entries"]))
        entry = manifest["entries"][0]
        expected_sha = entry["head_sha"]
        default_branch = report["default_branch"]
        default_sha = report["default_branch_sha"]

        class FakeApplyClient:
            repo = "blakinio/Oteryn-Platform"

            def __init__(client_self) -> None:
                client_self.refs = {
                    default_branch: default_sha,
                    branch: expected_sha,
                }
                client_self.branches = {
                    branch: {
                        "name": branch,
                        "protected": False,
                        "commit": {"sha": expected_sha},
                    }
                }
                client_self.open_pulls: dict[str, list[dict]] = {}
                client_self.issue_states: dict[int, str] = {}
                match = branch_lifecycle.REPAIR_ISSUE_RE.fullmatch(branch)
                if match:
                    client_self.issue_states[int(match.group(1))] = "closed"
                client_self.deleted: list[str] = []
                client_self.mutate_on_delete_sha: str | None = None

            def get_ref(client_self, name: str) -> dict | None:
                sha = client_self.refs.get(name)
                if sha is None:
                    return None
                return {"ref": f"refs/heads/{name}", "object": {"sha": sha}}

            def get_branch(client_self, name: str) -> dict | None:
                return copy.deepcopy(client_self.branches.get(name))

            def open_pulls_for_branch(client_self, name: str) -> list[dict]:
                return copy.deepcopy(client_self.open_pulls.get(name, []))

            def get_issue_state(client_self, issue_number: int) -> str:
                return client_self.issue_states.get(issue_number, "unknown")

            def delete_branch(
                client_self, name: str, *, expected_sha: str | None = None
            ) -> None:
                if client_self.mutate_on_delete_sha is not None:
                    changed = client_self.mutate_on_delete_sha
                    client_self.mutate_on_delete_sha = None
                    client_self.refs[name] = changed
                    client_self.branches[name]["commit"]["sha"] = changed
                current_sha = client_self.refs.get(name)
                if expected_sha is not None and current_sha != expected_sha:
                    raise branch_lifecycle.ValidationError(
                        f"pre-delete SHA drift for {name}: expected {expected_sha}, "
                        f"got {current_sha or 'missing'}"
                    )
                client_self.deleted.append(name)
                client_self.refs.pop(name, None)
                client_self.branches.pop(name, None)

        return FakeApplyClient(), manifest

    def apply(
        self,
        client: object,
        report: dict,
        manifest: dict,
        *,
        policy: dict | None = None,
    ) -> None:
        branch_lifecycle.apply_manifest(
            client,
            report,
            manifest,
            policy=policy or self.policy,
            root=self.root,
            evidence_path=self.root / "evidence.json",
            event_name="push",
            ref_name="main",
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
            def delete_branch(self, branch: str, *, expected_sha: str | None = None) -> None:
                raise AssertionError("must not delete")

        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "push to main"):
            branch_lifecycle.apply_manifest(
                NeverDelete(),
                report,
                manifest,
                policy=self.policy,
                root=self.root,
                evidence_path=self.root / "evidence.json",
                event_name="pull_request",
                ref_name="feature",
            )

    def test_apply_revalidates_and_deletes_unchanged_entry(self) -> None:
        report = self.classify()
        client, manifest = self.apply_fixture(report)
        self.apply(client, report, manifest)
        self.assertEqual(["docs/merged"], client.deleted)
        evidence = json.loads((self.root / "evidence.json").read_text(encoding="utf-8"))
        self.assertEqual("docs/merged", evidence["deleted"][0]["branch"])

    def test_apply_rejects_sha_change_after_manifest_validation(self) -> None:
        report = self.classify()
        client, manifest = self.apply_fixture(report)
        client.refs["docs/merged"] = "9" * 40
        client.branches["docs/merged"]["commit"]["sha"] = "9" * 40
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "pre-delete SHA drift"):
            self.apply(client, report, manifest)
        self.assertEqual([], client.deleted)

    def test_apply_rejects_new_open_pull_request(self) -> None:
        report = self.classify()
        client, manifest = self.apply_fixture(report)
        client.open_pulls["docs/merged"] = [
            self.pull(99, "docs/merged", "c" * 40, state="open", merged_at=None)
        ]
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "open pull request appeared"):
            self.apply(client, report, manifest)
        self.assertEqual([], client.deleted)

    def test_apply_rejects_new_active_task_claim(self) -> None:
        report = self.classify()
        client, manifest = self.apply_fixture(report)
        task = self.root / "docs/agents/tasks/active/race.md"
        task.write_text("```yaml\nbranch: docs/merged\n```\n", encoding="utf-8")
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "active task claim appeared"):
            self.apply(client, report, manifest)
        self.assertEqual([], client.deleted)

    def test_apply_rejects_reactivated_remediation_issue(self) -> None:
        snapshot = self.snapshot()
        snapshot["issue_states"]["777"] = "closed"
        snapshot["pulls"].append(self.pull(13, "repair/issue-777", "1" * 40))
        report = self.classify(snapshot)
        client, manifest = self.apply_fixture(report, branch="repair/issue-777")
        client.issue_states[777] = "open"
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "Issue #777 is open"):
            self.apply(client, report, manifest)
        self.assertEqual([], client.deleted)

    def test_apply_rejects_branch_that_becomes_protected(self) -> None:
        report = self.classify()
        client, manifest = self.apply_fixture(report)
        client.branches["docs/merged"]["protected"] = True
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "became protected"):
            self.apply(client, report, manifest)
        self.assertEqual([], client.deleted)

    def test_apply_rejects_retention_policy_race(self) -> None:
        report = self.classify()
        client, manifest = self.apply_fixture(report)
        changed_policy = copy.deepcopy(self.policy)
        changed_policy["retention_exceptions"].append(
            {
                "branch": "docs/merged",
                "classification": "RECOVERY",
                "owner": "recovery owner",
                "protected_required": True,
                "purpose": "new recovery hold",
                "review_trigger": "recovery complete",
            }
        )
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "policy drift"):
            self.apply(client, report, manifest, policy=changed_policy)
        self.assertEqual([], client.deleted)

    def test_apply_rejects_default_branch_drift(self) -> None:
        report = self.classify()
        client, manifest = self.apply_fixture(report)
        client.refs["main"] = "8" * 40
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "default branch drift"):
            self.apply(client, report, manifest)
        self.assertEqual([], client.deleted)

    def test_delete_call_rechecks_expected_sha(self) -> None:
        report = self.classify()
        client, manifest = self.apply_fixture(report)
        client.mutate_on_delete_sha = "7" * 40
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "pre-delete SHA drift"):
            self.apply(client, report, manifest)
        self.assertEqual([], client.deleted)

    def test_github_client_delete_branch_guards_expected_sha(self) -> None:
        class GuardClient(branch_lifecycle.GitHubClient):
            def __init__(client_self) -> None:
                client_self.repo = "blakinio/Oteryn-Platform"
                client_self.current_sha = "a" * 40
                client_self.leases: list[tuple[str, str]] = []

            def get_ref(client_self, branch: str) -> dict | None:
                return {"object": {"sha": client_self.current_sha}}

            def _delete_ref_with_lease(client_self, branch: str, expected_sha: str) -> None:
                client_self.leases.append((branch, expected_sha))

        client = GuardClient()
        with self.assertRaisesRegex(branch_lifecycle.ValidationError, "pre-delete SHA drift"):
            client.delete_branch("docs/merged", expected_sha="b" * 40)
        self.assertEqual([], client.leases)
        client.delete_branch("docs/merged", expected_sha="a" * 40)
        self.assertEqual([("docs/merged", "a" * 40)], client.leases)

    def test_github_client_delete_without_expected_sha_leases_observed_sha(self) -> None:
        class RecoveryClient(branch_lifecycle.GitHubClient):
            def __init__(client_self) -> None:
                client_self.repo = "blakinio/Oteryn-Platform"
                client_self.leases: list[tuple[str, str]] = []

            def get_ref(client_self, branch: str) -> dict | None:
                return {"object": {"sha": "a" * 40}}

            def _delete_ref_with_lease(client_self, branch: str, expected_sha: str) -> None:
                client_self.leases.append((branch, expected_sha))

        client = RecoveryClient()
        client.delete_branch("recovery-test/example")
        self.assertEqual([("recovery-test/example", "a" * 40)], client.leases)

    def test_github_client_atomic_delete_uses_exact_remote_lease_without_token(self) -> None:
        client = branch_lifecycle.GitHubClient(
            "blakinio/Oteryn-Platform", "super-secret-token"
        )
        expected_sha = "a" * 40
        client.get_ref = lambda branch: {"object": {"sha": expected_sha}}
        completed = branch_lifecycle.subprocess.CompletedProcess(
            args=[], returncode=0, stdout="", stderr=""
        )
        with mock.patch.object(
            branch_lifecycle.subprocess, "run", return_value=completed
        ) as run:
            client.delete_branch("docs/merged", expected_sha=expected_sha)
        command = run.call_args.args[0]
        self.assertEqual(
            [
                "git",
                "push",
                "--porcelain",
                f"--force-with-lease=refs/heads/docs/merged:{expected_sha}",
                "origin",
                ":refs/heads/docs/merged",
            ],
            command,
        )
        self.assertNotIn("super-secret-token", " ".join(command))
        self.assertFalse(run.call_args.kwargs["check"])
        self.assertTrue(run.call_args.kwargs["capture_output"])
        self.assertEqual(60, run.call_args.kwargs["timeout"])

    def test_github_client_atomic_delete_rejects_last_instruction_race(self) -> None:
        client = branch_lifecycle.GitHubClient(
            "blakinio/Oteryn-Platform", "super-secret-token"
        )
        expected_sha = "a" * 40
        advanced_sha = "b" * 40
        state = {"sha": expected_sha}

        def get_ref(branch: str) -> dict:
            return {"object": {"sha": state["sha"]}}

        def race_remote(command: list[str], **kwargs: object):
            self.assertIn(
                f"--force-with-lease=refs/heads/docs/merged:{expected_sha}", command
            )
            state["sha"] = advanced_sha
            return branch_lifecycle.subprocess.CompletedProcess(
                args=command, returncode=1, stdout="", stderr="stale info"
            )

        client.get_ref = get_ref
        with mock.patch.object(
            branch_lifecycle.subprocess, "run", side_effect=race_remote
        ):
            with self.assertRaisesRegex(
                branch_lifecycle.ValidationError, "atomic delete lease rejected"
            ):
                client.delete_branch("docs/merged", expected_sha=expected_sha)
        self.assertEqual(advanced_sha, state["sha"])


if __name__ == "__main__":
    unittest.main()
