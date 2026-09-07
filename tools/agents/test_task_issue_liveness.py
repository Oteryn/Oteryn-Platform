#!/usr/bin/env python3
from __future__ import annotations

import tempfile
import contextlib
import io
from unittest import mock
import unittest
from pathlib import Path

import task_issue_liveness as live

POLICY = live.Policy(
    schema_version=1,
    governing_issue_field="governing_issue",
    allowed_issue_states=frozenset({"open"}),
    report_schema_version=1,
)


def task_text(issue="91"):
    line = f"governing_issue: {issue}\n" if issue is not None else ""
    return f"""---
task_id: OTERYN-TEST-issue-live
{line}---
# Fixture
"""


class FakeClient:
    def __init__(self, payload=None, error=None):
        self.payload = payload or {"number": 91, "state": "open"}
        self.error = error

    def get_issue(self, number):
        if self.error:
            raise self.error
        return self.payload


class IssueLivenessTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.active = Path(self.tmp.name) / "active"
        self.active.mkdir()

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, text):
        (self.active / "task.md").write_text(text, encoding="utf-8")

    def evaluate(self, client):
        return live.evaluate_tasks(
            self.active,
            repository="Oteryn/Oteryn-Platform",
            client=client,
            policy=POLICY,
        )

    def test_open_governing_issue_passes(self):
        self.write(task_text())
        report = self.evaluate(FakeClient())
        self.assertTrue(report["live_valid"])

    def test_missing_governing_issue_fails_closed(self):
        self.write(task_text(None))
        report = self.evaluate(FakeClient())
        self.assertFalse(report["live_valid"])
        self.assertEqual(report["tasks"][0]["findings"][0]["code"], "missing_governing_issue")

    def test_closed_governing_issue_requires_archive(self):
        self.write(task_text())
        report = self.evaluate(FakeClient({"number": 91, "state": "closed"}))
        self.assertFalse(report["live_valid"])
        self.assertEqual(report["tasks"][0]["findings"][0]["code"], "governing_issue_terminal")

    def test_pull_request_cannot_be_governing_issue(self):
        self.write(task_text())
        report = self.evaluate(FakeClient({"number": 91, "state": "open", "pull_request": {}}))
        self.assertFalse(report["live_valid"])
        self.assertEqual(report["tasks"][0]["findings"][0]["code"], "governing_identity_is_pull_request")

    def test_api_failure_fails_closed(self):
        self.write(task_text())
        report = self.evaluate(FakeClient(error=live.IssueLivenessError("GitHub Issue state unavailable")))
        self.assertFalse(report["live_valid"])
        self.assertEqual(report["tasks"][0]["findings"][0]["code"], "github_issue_state_unavailable")


    def test_missing_inventory_is_not_a_valid_empty_directory(self):
        self.active.rmdir()
        client = mock.Mock()
        with self.assertRaisesRegex(live.IssueLivenessError, "active task inventory must be an existing directory"):
            self.evaluate(client)
        client.get_issue.assert_not_called()

    def test_file_inventory_is_not_a_valid_empty_directory(self):
        self.active.rmdir()
        self.active.write_text("not a directory", encoding="utf-8")
        client = mock.Mock()
        with self.assertRaisesRegex(live.IssueLivenessError, "active task inventory must be an existing directory"):
            self.evaluate(client)
        client.get_issue.assert_not_called()

    def test_existing_empty_inventory_needs_no_api_calls(self):
        client = mock.Mock()
        report = self.evaluate(client)
        self.assertTrue(report["live_valid"])
        self.assertEqual(report["tasks"], [])
        self.assertEqual(report["errors"], 0)
        client.get_issue.assert_not_called()

    def test_readme_only_inventory_needs_no_api_calls(self):
        (self.active / "README.md").write_text("# Active tasks\n", encoding="utf-8")
        client = mock.Mock()
        report = self.evaluate(client)
        self.assertTrue(report["live_valid"])
        self.assertEqual(report["tasks"], [])
        client.get_issue.assert_not_called()

    def test_cli_returns_failure_without_success_report_for_invalid_inventory(self):
        self.active.rmdir()
        output = Path(self.tmp.name) / "report.json"
        stderr = io.StringIO()
        with mock.patch.object(live, "load_policy", return_value=POLICY), \
                mock.patch.object(live, "GitHubClient") as client, \
                contextlib.redirect_stderr(stderr):
            result = live.main([
                "--repository", "Oteryn/Oteryn-Platform", "--active", str(self.active),
                "--report-json", str(output),
            ])
        self.assertEqual(result, 1)
        self.assertIn("active task inventory must be an existing directory", stderr.getvalue())
        self.assertFalse(output.exists())
        client.return_value.get_issue.assert_not_called()


if __name__ == "__main__":
    unittest.main()
