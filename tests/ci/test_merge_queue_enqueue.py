#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOOL_PATH = ROOT / "scripts/github/merge_queue_enqueue.py"
spec = importlib.util.spec_from_file_location("merge_queue_enqueue", TOOL_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"unable to load {TOOL_PATH}")
mod = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = mod
spec.loader.exec_module(mod)

HEAD = "a" * 40


def pull_payload(**overrides):
    payload = {
        "state": "open",
        "merged": False,
        "draft": False,
        "node_id": "PR_kwDOexample",
        "base": {"ref": "main"},
        "head": {
            "sha": HEAD,
            "repo": {"full_name": "Oteryn/Oteryn-Platform"},
        },
    }
    payload.update(overrides)
    return payload


def checks_payload(*, status="completed", conclusion="success", head_sha=HEAD, run_id=10):
    return {
        "check_runs": [
            {
                "id": run_id,
                "name": "platform-gate",
                "head_sha": head_sha,
                "status": status,
                "conclusion": conclusion,
            }
        ]
    }


class FakeClient:
    def __init__(self, pull=None, checks=None, graphql_data=None):
        self.pull = pull if pull is not None else pull_payload()
        self.checks = checks if checks is not None else checks_payload()
        self.graphql_data = graphql_data or {
            "enqueuePullRequest": {
                "mergeQueueEntry": {"id": "MQE_example", "position": 1, "state": "AWAITING_CHECKS"}
            }
        }
        self.rest_calls = []
        self.graphql_calls = []

    def rest(self, method, path):
        self.rest_calls.append((method, path))
        if "/pulls/" in path:
            return self.pull
        if "/check-runs?" in path:
            return self.checks
        raise AssertionError(f"unexpected REST path {path}")

    def graphql(self, query, variables):
        self.graphql_calls.append((query, dict(variables)))
        return self.graphql_data


class MergeQueueEnqueueTest(unittest.TestCase):
    def qualify(self, client=None, **kwargs):
        return mod.qualify_pull_request(
            client or FakeClient(),
            repository=kwargs.get("repository", "Oteryn/Oteryn-Platform"),
            pr_number=kwargs.get("pr_number", 1363),
            expected_head_sha=kwargs.get("expected_head_sha", HEAD),
        )

    def test_rejects_wrong_repository_before_network(self):
        client = FakeClient()
        with self.assertRaisesRegex(ValueError, "repository must be exactly"):
            self.qualify(client, repository="Other/Repo")
        self.assertEqual([], client.rest_calls)

    def test_rejects_invalid_expected_head(self):
        client = FakeClient()
        with self.assertRaisesRegex(ValueError, "40-character"):
            self.qualify(client, expected_head_sha="abc")
        self.assertEqual([], client.rest_calls)

    def test_rejects_closed_pull_request(self):
        with self.assertRaisesRegex(ValueError, "open and unmerged"):
            self.qualify(FakeClient(pull=pull_payload(state="closed")))

    def test_rejects_draft_pull_request(self):
        with self.assertRaisesRegex(ValueError, "not draft"):
            self.qualify(FakeClient(pull=pull_payload(draft=True)))

    def test_rejects_wrong_base(self):
        with self.assertRaisesRegex(ValueError, "base must be exactly main"):
            self.qualify(FakeClient(pull=pull_payload(base={"ref": "develop"})))

    def test_rejects_changed_head(self):
        with self.assertRaisesRegex(ValueError, "head changed"):
            self.qualify(
                FakeClient(
                    pull=pull_payload(
                        head={
                            "sha": "b" * 40,
                            "repo": {"full_name": "Oteryn/Oteryn-Platform"},
                        }
                    )
                )
            )

    def test_rejects_cross_repository_head(self):
        with self.assertRaisesRegex(ValueError, "same-repository"):
            self.qualify(
                FakeClient(
                    pull=pull_payload(
                        head={"sha": HEAD, "repo": {"full_name": "fork/Oteryn-Platform"}}
                    )
                )
            )

    def test_rejects_missing_platform_gate(self):
        with self.assertRaisesRegex(ValueError, "no exact-head platform-gate"):
            self.qualify(FakeClient(checks={"check_runs": []}))

    def test_rejects_platform_gate_for_different_head(self):
        with self.assertRaisesRegex(ValueError, "matches the exact expected head"):
            self.qualify(FakeClient(checks=checks_payload(head_sha="b" * 40)))

    def test_rejects_non_success_latest_platform_gate(self):
        with self.assertRaisesRegex(ValueError, "completed/success"):
            self.qualify(FakeClient(checks=checks_payload(conclusion="failure")))

    def test_uses_latest_matching_platform_gate_by_run_id(self):
        checks = {
            "check_runs": [
                {"id": 11, "name": "platform-gate", "head_sha": HEAD, "status": "completed", "conclusion": "success"},
                {"id": 12, "name": "platform-gate", "head_sha": HEAD, "status": "completed", "conclusion": "failure"},
            ]
        }
        with self.assertRaisesRegex(ValueError, "completed/success"):
            self.qualify(FakeClient(checks=checks))

    def test_successful_qualification_is_exact_head_fenced(self):
        client = FakeClient()
        qualified = self.qualify(client)
        self.assertEqual("PR_kwDOexample", qualified.node_id)
        self.assertEqual(HEAD, qualified.head_sha)
        self.assertIn(f"/commits/{HEAD}/check-runs?", client.rest_calls[1][1])
        self.assertIn("check_name=platform-gate", client.rest_calls[1][1])

    def test_enqueue_uses_only_enqueue_mutation_and_expected_head_oid(self):
        client = FakeClient()
        qualified = self.qualify(client)
        entry = mod.enqueue_pull_request(client, qualified)
        self.assertEqual("MQE_example", entry["id"])
        query, variables = client.graphql_calls[0]
        self.assertIn("enqueuePullRequest", query)
        self.assertNotIn("mergePullRequest", query)
        self.assertEqual("PR_kwDOexample", variables["pullRequestId"])
        self.assertEqual(HEAD, variables["expectedHeadOid"])

    def test_enqueue_rejects_missing_queue_entry(self):
        client = FakeClient(graphql_data={"enqueuePullRequest": {"mergeQueueEntry": None}})
        qualified = self.qualify(client)
        with self.assertRaisesRegex(mod.GitHubRequestError, "did not return a merge queue entry"):
            mod.enqueue_pull_request(client, qualified)


if __name__ == "__main__":
    unittest.main()
