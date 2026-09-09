#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
TOOL_PATH = ROOT / "scripts/github/merge_queue_enqueue.py"
WORKFLOW_PATH = ROOT / ".github/workflows/merge-queue-enqueue.yml"
DOC_PATH = ROOT / "docs/operations/MERGE_QUEUE_EXECUTOR.md"
TASK_PATH = ROOT / "docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md"
spec = importlib.util.spec_from_file_location("merge_queue_enqueue", TOOL_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"unable to load {TOOL_PATH}")
mod = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = mod
spec.loader.exec_module(mod)

HEAD = "a" * 40
UUID = "630b9d5e-3f2a-4f7e-8b0c-2d5f9a8c1e42"


def pull_payload(**overrides):
    payload = {
        "state": "open", "merged": False, "draft": False,
        "base": {"ref": "main"},
        "head": {"sha": HEAD, "repo": {"full_name": "Oteryn/Oteryn-Platform"}},
    }
    payload.update(overrides)
    return payload


def checks_payload(*, status="completed", conclusion="success", head_sha=HEAD, run_id=10):
    return {"check_runs": [{
        "id": run_id, "name": "platform-gate", "head_sha": head_sha,
        "status": status, "conclusion": conclusion,
    }]}


def async_payload(*, server_uuid=UUID, status="pending", action="merge_queue", head=HEAD):
    return {"status": status, "details": {
        "uuid": server_uuid, "merge_action": action, "expected_head_sha": head,
    }}


class FakeClient:
    def __init__(self, *, pull=None, post_pull=None, checks=None, submit_status=202,
                 submit=None, readback=None):
        self.pull = pull if pull is not None else pull_payload()
        self.post_pull = post_pull if post_pull is not None else self.pull
        self.checks = checks if checks is not None else checks_payload()
        self.submit_status = submit_status
        self.submit = submit if submit is not None else async_payload()
        self.readback = readback if readback is not None else async_payload()
        self.calls = []
        self.pull_reads = 0

    def rest(self, method, path, *, body=None, allowed_statuses=(200,)):
        self.calls.append((method, path, body, allowed_statuses))
        if method == "PUT":
            return mod.Response(self.submit_status, self.submit)
        if "/merge-async/" in path:
            return mod.Response(200, self.readback)
        if "/check-runs?" in path:
            return mod.Response(200, self.checks)
        if "/pulls/" in path:
            self.pull_reads += 1
            return mod.Response(200, self.pull if self.pull_reads == 1 else self.post_pull)
        raise AssertionError(f"unexpected REST path {path}")


class MergeQueuePreflightTest(unittest.TestCase):
    def qualify(self, client=None, **kwargs):
        return mod.qualify_pull_request(
            client or FakeClient(), repository=kwargs.get("repository", "Oteryn/Oteryn-Platform"),
            pr_number=kwargs.get("pr_number", 1363),
            expected_head_sha=kwargs.get("expected_head_sha", HEAD),
        )

    def test_rejects_wrong_repository_before_network(self):
        client = FakeClient()
        with self.assertRaisesRegex(ValueError, "repository must be exactly"):
            self.qualify(client, repository="Other/Repo")
        self.assertEqual([], client.calls)

    def test_rejects_nonpositive_or_boolean_pr_number(self):
        for number in (0, -1, False):
            with self.subTest(number=number), self.assertRaisesRegex(ValueError, "positive"):
                self.qualify(FakeClient(), pr_number=number)

    def test_rejects_invalid_expected_head(self):
        with self.assertRaisesRegex(ValueError, "40-character"):
            self.qualify(FakeClient(), expected_head_sha="abc")

    def test_rejects_closed_or_merged_pull_request(self):
        for payload in (pull_payload(state="closed"), pull_payload(merged=True)):
            with self.subTest(payload=payload), self.assertRaisesRegex(ValueError, "open and unmerged"):
                self.qualify(FakeClient(pull=payload))

    def test_rejects_draft_pull_request(self):
        with self.assertRaisesRegex(ValueError, "not draft"):
            self.qualify(FakeClient(pull=pull_payload(draft=True)))

    def test_rejects_wrong_base(self):
        with self.assertRaisesRegex(ValueError, "base must be exactly main"):
            self.qualify(FakeClient(pull=pull_payload(base={"ref": "develop"})))

    def test_rejects_changed_head(self):
        with self.assertRaisesRegex(ValueError, "head changed"):
            self.qualify(FakeClient(pull=pull_payload(head={
                "sha": "b" * 40, "repo": {"full_name": "Oteryn/Oteryn-Platform"},
            })))

    def test_rejects_cross_repository_head(self):
        with self.assertRaisesRegex(ValueError, "same-repository"):
            self.qualify(FakeClient(pull=pull_payload(head={
                "sha": HEAD, "repo": {"full_name": "fork/Oteryn-Platform"},
            })))

    def test_rejects_missing_or_wrong_head_platform_gate(self):
        with self.assertRaisesRegex(ValueError, "no exact-head platform-gate"):
            self.qualify(FakeClient(checks={"check_runs": []}))
        with self.assertRaisesRegex(ValueError, "matches the exact expected head"):
            self.qualify(FakeClient(checks=checks_payload(head_sha="b" * 40)))

    def test_rejects_non_success_latest_platform_gate(self):
        checks = {"check_runs": [
            {"id": 11, "name": "platform-gate", "head_sha": HEAD,
             "status": "completed", "conclusion": "success"},
            {"id": 12, "name": "platform-gate", "head_sha": HEAD,
             "status": "completed", "conclusion": "failure"},
        ]}
        with self.assertRaisesRegex(ValueError, "completed/success"):
            self.qualify(FakeClient(checks=checks))

    def test_successful_qualification_is_exact_head_fenced(self):
        client = FakeClient()
        qualified = self.qualify(client)
        self.assertEqual(("Oteryn/Oteryn-Platform", 1363, "main", HEAD), (
            qualified.repository, qualified.number, qualified.base, qualified.head_sha,
        ))
        self.assertIn(f"/commits/{HEAD}/check-runs?", client.calls[1][1])
        self.assertIn("check_name=platform-gate", client.calls[1][1])


class MergeAsyncContractTest(unittest.TestCase):
    def qualify(self, client):
        return mod.qualify_pull_request(
            client, repository="Oteryn/Oteryn-Platform", pr_number=1363,
            expected_head_sha=HEAD,
        )

    def submit(self, mutation_client, *, read_client=None):
        read_client = read_client or mutation_client
        pull = self.qualify(read_client)
        return mod.submit_merge_queue(mutation_client, pull, target_client=read_client)

    def test_read_and_mutation_credentials_are_separated(self):
        read_client = FakeClient()
        mutation_client = FakeClient()
        result = self.submit(mutation_client, read_client=read_client)
        self.assertEqual("REQUEST_ACCEPTED_NON_TERMINAL", result["result"])
        self.assertTrue(any("/check-runs?" in call[1] for call in read_client.calls))
        self.assertTrue(any(call[0] == "GET" and call[1].endswith("/pulls/1363") for call in read_client.calls))
        self.assertFalse(any(call[0] == "PUT" for call in read_client.calls))
        self.assertEqual("PUT", mutation_client.calls[0][0])
        self.assertEqual("/repos/Oteryn/Oteryn-Platform/pulls/1363/merge-async", mutation_client.calls[0][1])
        self.assertEqual({"sha": HEAD, "merge_action": "merge_queue"}, mutation_client.calls[0][2])
        self.assertTrue(any("/merge-async/" in call[1] for call in mutation_client.calls))
        self.assertFalse(any("/check-runs?" in call[1] for call in mutation_client.calls))
        self.assertFalse(any(
            call[0] == "GET" and call[1].endswith("/pulls/1363")
            for call in mutation_client.calls
        ))

    def test_202_records_receipt_and_strictly_later_same_uuid_readback(self):
        result = self.submit(FakeClient())
        self.assertEqual("REQUEST_ACCEPTED_NON_TERMINAL", result["result"])
        self.assertEqual(UUID, result["receipt"]["server_uuid"])
        self.assertEqual(UUID, result["readback"]["server_uuid"])
        self.assertEqual(1, result["receipt"]["sequence"])
        self.assertEqual(2, result["readback"]["sequence"])
        for evidence in (result["receipt"], result["readback"]):
            self.assertEqual("Oteryn/Oteryn-Platform", evidence["repository"])
            self.assertEqual(1363, evidence["pr_number"])
            self.assertEqual("main", evidence["base"])
            self.assertEqual(HEAD, evidence["head_sha"])
            self.assertEqual("merge_queue", evidence["merge_action"])

    def test_202_rejects_missing_or_malformed_uuid(self):
        for value in (None, "not-a-uuid"):
            payload = async_payload()
            payload["details"]["uuid"] = value
            with self.subTest(value=value), self.assertRaisesRegex(mod.GitHubRequestError, "server UUID"):
                self.submit(FakeClient(submit=payload))

    def test_rejects_uuid_mismatch(self):
        with self.assertRaisesRegex(mod.GitHubRequestError, "UUID does not match"):
            self.submit(FakeClient(readback=async_payload(
                server_uuid="730b9d5e-3f2a-4f7e-8b0c-2d5f9a8c1e42"
            )))

    def test_rejects_readback_action_or_head_mismatch(self):
        for payload, message in (
            (async_payload(action="default"), "explicit merge_queue"),
            (async_payload(head="b" * 40), "head does not match"),
        ):
            with self.subTest(message=message), self.assertRaisesRegex(mod.GitHubRequestError, message):
                self.submit(FakeClient(readback=payload))

    def test_rejects_post_submission_retarget_or_head_change(self):
        cases = (
            (pull_payload(base={"ref": "develop"}), "target mismatch"),
            (pull_payload(head={"sha": "b" * 40, "repo": {"full_name": "Oteryn/Oteryn-Platform"}}), "target mismatch"),
        )
        for payload, message in cases:
            with self.subTest(payload=payload), self.assertRaisesRegex(mod.GitHubRequestError, message):
                self.submit(FakeClient(post_pull=payload))

    def test_rejects_equal_lower_boolean_or_nonpositive_sequence(self):
        for values in ((1, 1), (2, 1), (True,), (0,), (-1,)):
            with self.subTest(values=values), mock.patch.object(
                mod.ExecutorSequence, "next", side_effect=values
            ), self.assertRaisesRegex(mod.GitHubRequestError, "sequence"):
                self.submit(FakeClient())

    def test_200_without_uuid_is_live_nonterminal_reconciliation(self):
        client = FakeClient(
            submit_status=200,
            submit={"status": "merged", "details": {"message": "already"}},
            post_pull=pull_payload(state="closed", merged=True),
        )
        result = self.submit(client)
        self.assertEqual("RECONCILIATION_REQUIRED", result["result"])
        self.assertFalse(result["accepted"])
        self.assertIsNone(result["receipt"])
        self.assertIsNone(result["readback"])
        self.assertEqual(2, client.pull_reads)

    def test_200_or_409_with_uuid_reconciles_identity_after_pr_completes(self):
        completed = pull_payload(state="closed", merged=True)
        for status in (200, 409):
            with self.subTest(status=status):
                result = self.submit(FakeClient(
                    submit_status=status,
                    submit=async_payload(status="enqueued"),
                    post_pull=completed,
                ))
                self.assertEqual("RECONCILIATION_REQUIRED", result["result"])
                self.assertEqual(status, result["http_status"])
                self.assertFalse(result["accepted"])
                self.assertIsNone(result["receipt"])
                self.assertEqual(UUID, result["readback"]["server_uuid"])

    def test_409_with_existing_uuid_is_live_nonterminal_reconciliation(self):
        result = self.submit(FakeClient(submit_status=409))
        self.assertEqual("RECONCILIATION_REQUIRED", result["result"])
        self.assertEqual(409, result["http_status"])
        self.assertFalse(result["accepted"])
        self.assertIsNone(result["receipt"])
        self.assertEqual(UUID, result["readback"]["server_uuid"])

    def test_fail_closed_status_classification(self):
        for status, classification in (
            (400, "REQUEST_REJECTED_HTTP_400"), (422, "REQUEST_REJECTED_HTTP_422"),
            (403, "BLOCKED_CAPABILITY_UNAVAILABLE"), (404, "BLOCKED_CAPABILITY_UNAVAILABLE"),
        ):
            with self.subTest(status=status), self.assertRaisesRegex(mod.GitHubRequestError, classification):
                self.submit(FakeClient(submit_status=status, submit={"message": "denied"}))

    def test_source_forbids_graphql_direct_and_default_alternatives(self):
        text = TOOL_PATH.read_text(encoding="utf-8")
        self.assertNotIn("graphql", text.lower())
        self.assertNotIn("enqueuePullRequest", text)
        self.assertNotIn("expectedHeadOid", text)
        self.assertNotIn("direct_merge", text)
        self.assertNotIn('"merge_action": "default"', text)
        self.assertNotIn('/pulls/{pull.number}/merge"', text)


class WorkflowAndDocumentationContractTest(unittest.TestCase):
    def test_workflow_is_app_free_and_keeps_github_token_read_only(self):
        text = WORKFLOW_PATH.read_text(encoding="utf-8")
        self.assertIn("permissions:\n  contents: read\n  issues: read\n  pull-requests: read", text)
        self.assertIn("MQ_GITHUB_READ_TOKEN: ${{ github.token }}", text)
        self.assertIn("MQ_GITHUB_MUTATION_TOKEN: ${{ secrets.OTERYN_MQ_TOKEN }}", text)
        for forbidden in (
            "actions/create-github-app-token",
            "OTERYN_MQ_APP_CLIENT_ID",
            "OTERYN_MQ_APP_PRIVATE_KEY",
            "permission-contents: write",
            "permission-merge-queues",
            "permissions: write-all",
        ):
            self.assertNotIn(forbidden, text)
        self.assertIn("actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1", text)

    def test_exact_authorized_pr_comment_contract(self):
        text = WORKFLOW_PATH.read_text(encoding="utf-8")
        self.assertIn("workflow_dispatch:", text)
        self.assertIn("issue_comment:", text)
        self.assertIn("github.event.issue.number", text)
        self.assertIn("github.event.issue.pull_request != null", text)
        self.assertIn("github.event.comment.author_association", text)
        self.assertIn("^(OWNER|MEMBER|COLLABORATOR)$", text)
        self.assertIn("^/oteryn-mq-enqueue\\ ([0-9a-fA-F]{40})$", text)
        self.assertIn("needs.authorize.outputs.allowed == 'true'", text)

    def test_malformed_unauthorized_and_non_pr_comments_cannot_authorize(self):
        command_re = mod.re.compile(r"^/oteryn-mq-enqueue ([0-9a-fA-F]{40})$")
        self.assertIsNotNone(command_re.fullmatch(f"/oteryn-mq-enqueue {HEAD}"))
        for command in (
            "/oteryn-mq-enqueue",
            f"/oteryn-mq-enqueue {HEAD} extra",
            f"x/oteryn-mq-enqueue {HEAD}",
        ):
            self.assertIsNone(command_re.fullmatch(command))
        authorized = {"OWNER", "MEMBER", "COLLABORATOR"}
        self.assertNotIn("CONTRIBUTOR", authorized)
        self.assertNotIn("NONE", authorized)
        self.assertIn("ISSUE_IS_PULL_REQUEST\" == true", WORKFLOW_PATH.read_text(encoding="utf-8"))

    def test_docs_and_task_forbid_custom_app_bootstrap(self):
        for path in (DOC_PATH, TASK_PATH):
            text = path.read_text(encoding="utf-8")
            self.assertIn("merge-async", text)
            self.assertIn("OTERYN_MQ_TOKEN", text)
            self.assertIn("fine-grained", text.lower())
            self.assertNotIn("OTERYN_MQ_APP_CLIENT_ID", text)
            self.assertNotIn("OTERYN_MQ_APP_PRIVATE_KEY", text)
            self.assertNotIn("create-github-app-token", text)
        self.assertIn("real canary", TASK_PATH.read_text(encoding="utf-8").lower())


if __name__ == "__main__":
    unittest.main()
