#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
POLICY_TOOL_PATH = REPOSITORY_ROOT / "scripts/github/repository_policy.py"

spec = importlib.util.spec_from_file_location("repository_policy", POLICY_TOOL_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"Unable to load repository policy tool from {POLICY_TOOL_PATH}")
repository_policy = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = repository_policy
spec.loader.exec_module(repository_policy)


class RepositoryPolicyTest(unittest.TestCase):
    def policy(self) -> dict[str, object]:
        return {
            "repository": "Oteryn/Oteryn-Platform",
            "default_branch": "main",
            "repository_settings": {
                "allow_squash_merge": True,
                "allow_merge_commit": False,
                "delete_branch_on_merge": True,
            },
            "branch_protection": {
                "required_status_checks": {
                    "strict": True,
                    "contexts": ["test", "classify-changes"],
                },
                "enforce_admins": True,
                "required_pull_request_reviews": {
                    "dismiss_stale_reviews": True,
                    "require_code_owner_reviews": False,
                    "required_approving_review_count": 0,
                    "require_last_push_approval": False,
                },
                "restrictions": None,
                "required_linear_history": True,
                "allow_force_pushes": False,
                "allow_deletions": False,
                "block_creations": False,
                "required_conversation_resolution": True,
                "lock_branch": False,
                "allow_fork_syncing": True,
            },
            "security_features": {
                "secret_scanning": True,
                "secret_scanning_push_protection": True,
            },
            "environments": {},
        }

    def test_repository_patch_includes_merge_and_security_settings(self) -> None:
        patch = repository_policy.repository_patch(self.policy())

        self.assertTrue(patch["allow_squash_merge"])
        self.assertFalse(patch["allow_merge_commit"])
        self.assertTrue(patch["delete_branch_on_merge"])
        self.assertEqual(
            "enabled", patch["security_and_analysis"]["secret_scanning"]["status"]
        )
        self.assertEqual(
            "enabled",
            patch["security_and_analysis"]["secret_scanning_push_protection"][
                "status"
            ],
        )

    def test_repository_drift_reports_only_different_values(self) -> None:
        drift = repository_policy.repository_drift(
            self.policy(),
            {
                "allow_squash_merge": True,
                "allow_merge_commit": True,
                "delete_branch_on_merge": True,
            },
        )

        self.assertEqual(1, len(drift))
        self.assertEqual("repository_settings.allow_merge_commit", drift[0]["path"])
        self.assertFalse(drift[0]["expected"])
        self.assertTrue(drift[0]["actual"])

    def test_branch_protection_normalizes_enabled_objects_and_context_order(self) -> None:
        drift = repository_policy.branch_protection_drift(
            self.policy(),
            {
                "required_status_checks": {
                    "strict": True,
                    "contexts": ["classify-changes", "test"],
                },
                "enforce_admins": {"enabled": True},
                "required_pull_request_reviews": {
                    "dismiss_stale_reviews": True,
                    "require_code_owner_reviews": False,
                    "required_approving_review_count": 0,
                    "require_last_push_approval": False,
                },
                "restrictions": None,
                "required_linear_history": {"enabled": True},
                "allow_force_pushes": {"enabled": False},
                "allow_deletions": {"enabled": False},
                "block_creations": {"enabled": False},
                "required_conversation_resolution": {"enabled": True},
                "lock_branch": {"enabled": False},
                "allow_fork_syncing": {"enabled": True},
            },
        )

        self.assertEqual([], drift)

    def test_environment_drift_normalizes_reviewer_response_shape(self) -> None:
        drift = repository_policy.environment_drift(
            "production",
            {
                "wait_timer": 5,
                "prevent_self_review": False,
                "reviewers": [
                    {"type": "User", "id": 75369544, "login": "blakinio"}
                ],
                "deployment_branch_policy": {
                    "protected_branches": True,
                    "custom_branch_policies": False,
                },
                "expected_can_admins_bypass": False,
            },
            {
                "can_admins_bypass": False,
                "protection_rules": [
                    {"type": "wait_timer", "wait_timer": 5},
                    {
                        "type": "required_reviewers",
                        "prevent_self_review": False,
                        "reviewers": [
                            {
                                "type": "User",
                                "reviewer": {"id": 75369544, "type": "User"},
                            }
                        ],
                    },
                ],
                "deployment_branch_policy": {
                    "protected_branches": True,
                    "custom_branch_policies": False,
                },
            },
        )

        self.assertEqual([], drift)

    def test_load_policy_rejects_missing_required_keys(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "policy.json"
            path.write_text(json.dumps({"repository": "example/repo"}), encoding="utf-8")

            with self.assertRaisesRegex(RuntimeError, "missing required keys"):
                repository_policy.load_policy(path)


if __name__ == "__main__":
    unittest.main()
