#!/usr/bin/env python3
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
WRITE_WORKFLOW = ROOT / ".github/workflows/terminal-branch-lifecycle-reusable.yml"
READ_WORKFLOW = ROOT / ".github/workflows/terminal-branch-lifecycle-read-reusable.yml"
LOCAL_WORKFLOW = ROOT / ".github/workflows/terminal-branch-lifecycle.yml"


class ReusableTerminalBranchWorkflowTests(unittest.TestCase):
    def test_write_reusable_workflow_exists(self) -> None:
        self.assertTrue(WRITE_WORKFLOW.is_file(), "write-capable reusable terminal branch lifecycle workflow is missing")

    def test_read_reusable_workflow_exists_and_is_strictly_read_only(self) -> None:
        self.assertTrue(READ_WORKFLOW.is_file(), "read-only reusable terminal branch lifecycle workflow is missing")
        text = READ_WORKFLOW.read_text(encoding="utf-8")
        required = (
            "workflow_call:",
            "platform_ref:",
            "policy_path:",
            "approval_path:",
            "repository: Oteryn/Oteryn-Platform",
            "ref: ${{ inputs.platform_ref }}",
            "persist-credentials: false",
            "path: .oteryn-branch-lifecycle",
            "--root \"$GITHUB_WORKSPACE\"",
            "--policy \"${{ inputs.policy_path }}\"",
            "--mode inventory",
            "contents: read",
            "issues: read",
            "pull-requests: read",
        )
        for marker in required:
            with self.subTest(marker=marker):
                self.assertIn(marker, text)
        self.assertNotIn("contents: write", text)
        self.assertNotIn("--mode event", text)
        self.assertNotIn("--mode apply", text)
        self.assertNotIn("operation:", text)

    def test_write_reusable_workflow_preserves_repository_local_authority(self) -> None:
        text = WRITE_WORKFLOW.read_text(encoding="utf-8")
        required = (
            "workflow_call:",
            "operation:",
            "platform_ref:",
            "policy_path:",
            "approval_path:",
            "repository: Oteryn/Oteryn-Platform",
            "ref: ${{ inputs.platform_ref }}",
            "persist-credentials: false",
            "path: .oteryn-branch-lifecycle",
            "--root \"$GITHUB_WORKSPACE\"",
            "--policy \"${{ inputs.policy_path }}\"",
            "inputs.operation == 'close'",
            "inputs.operation == 'apply'",
            "ref: main",
            "contents: write",
            "contents: read",
        )
        for marker in required:
            with self.subTest(marker=marker):
                self.assertIn(marker, text)
        self.assertNotIn("inputs.operation == 'read'", text)
        self.assertNotIn("secrets: inherit", text)
        self.assertNotIn("pull_request_target:", text)
        self.assertNotIn("branches: [", text)

    def test_close_and_apply_use_existing_exact_head_controls(self) -> None:
        text = WRITE_WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("terminal_branch_cleanup.py", text)
        self.assertIn("terminal_branch_approval.py", text)
        self.assertIn("--mode event", text)
        self.assertIn("--mode apply", text)
        self.assertIn("--event \"$GITHUB_EVENT_PATH\"", text)
        self.assertIn("--event-name push", text)
        self.assertIn("--ref-name main", text)

    def test_platform_validation_executes_reusable_contract_test(self) -> None:
        text = LOCAL_WORKFLOW.read_text(encoding="utf-8")
        self.assertIn('".github/workflows/terminal-branch-lifecycle-reusable.yml"', text)
        self.assertIn('".github/workflows/terminal-branch-lifecycle-read-reusable.yml"', text)
        self.assertIn('"tools/agents/test_terminal_branch_reusable.py"', text)
        self.assertIn("python3 tools/agents/test_terminal_branch_reusable.py", text)


if __name__ == "__main__":
    unittest.main()
