#!/usr/bin/env python3
from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import branch_lifecycle
from test_terminal_branch_guarded_legacy import *  # noqa: F401,F403


class BranchRefConsistencyTest(unittest.TestCase):
    def client(self) -> tuple[branch_lifecycle.GitHubClient, str]:
        root = Path(tempfile.mkdtemp())
        client = branch_lifecycle.GitHubClient(
            "blakinio/Oteryn-Platform", "test-token", root=root
        )
        sha = "a" * 40

        def request(method: str, path: str, **kwargs: object):
            self.assertEqual("GET", method)
            return (
                {"ref": "refs/heads/docs/merged", "object": {"sha": sha}},
                {},
            )

        client.request = request  # type: ignore[method-assign]
        client._delete_ref_with_lease = lambda branch, expected_sha: None  # type: ignore[method-assign]
        return client, sha

    def test_git_absence_wins_over_stale_rest_after_exact_delete(self) -> None:
        client, sha = self.client()
        client._git_remote_ref_sha = lambda branch: None  # type: ignore[method-assign]
        client.delete_branch("docs/merged", expected_sha=sha)
        self.assertIsNone(client.get_ref("docs/merged"))

    def test_reappeared_git_ref_fails_closed_instead_of_reporting_absent(self) -> None:
        client, sha = self.client()
        client._git_remote_ref_sha = lambda branch: "b" * 40  # type: ignore[method-assign]
        client.delete_branch("docs/merged", expected_sha=sha)
        self.assertIsNotNone(client.get_ref("docs/merged"))


if __name__ == "__main__":
    unittest.main()
