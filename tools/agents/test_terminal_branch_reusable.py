#!/usr/bin/env python3
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/terminal-branch-lifecycle-reusable.yml"


class ReusableTerminalBranchWorkflowTests(unittest.TestCase):
    def test_reusable_workflow_exists(self) -> None:
        self.assertTrue(WORKFLOW.is_file(), "reusable terminal branch lifecycle workflow is missing")

    def test_reusable_workflow_preserves_repository_local_authority(self) -> None:
        self.assertTrue(WORKFLOW.is_file(), "reusable terminal branch lifecycle workflow is missing")
        text = WORKFLOW.read_text(encoding="utf-8")

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
            "inputs.operation == 'read'",
            "inputs.operation == 'close'",
            "inputs.operation == 'apply'",
            "ref: main",
            "contents: write",
            "contents: read",
        )
        for marker in required:
            with self.subTest(marker=marker):
                self.assertIn(marker, text)

        self.assertNotIn("secrets: inherit", text)
        self.assertNotIn("pull_request_target:", text)
        self.assertNotIn("branches: [", text)

    def test_close_and_apply_use_existing_exact_head_controls(self) -> None:
        self.assertTrue(WORKFLOW.is_file(), "reusable terminal branch lifecycle workflow is missing")
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("terminal_branch_cleanup.py", text)
        self.assertIn("terminal_branch_approval.py", text)
        self.assertIn("--mode event", text)
        self.assertIn("--mode apply", text)
        self.assertIn("--event \"$GITHUB_EVENT_PATH\"", text)
        self.assertIn("--event-name push", text)
        self.assertIn("--ref-name main", text)


if __name__ == "__main__":
    unittest.main()
