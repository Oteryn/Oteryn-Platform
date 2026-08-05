#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("checkpoint.py")
SPEC = importlib.util.spec_from_file_location("checkpoint", MODULE_PATH)
assert SPEC and SPEC.loader
checkpoint = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = checkpoint
SPEC.loader.exec_module(checkpoint)

VALID_BLOCK = """checkpoint_version: 1
updated_at: 2026-07-18T22:00:00Z
head: abc123
branch: task/example
pr: none
status: implementing
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/**
proven:
  - source inspection completed
derived:
  - minimal validator is sufficient
unknown:
  - CI result not known yet
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - tools/agents/checkpoint.py
validation:
  - command: python tools/agents/test_checkpoint.py
    result: PASS
    evidence: local unittest
blockers:
  - none
next_action: Open the draft pull request and inspect its current-head checks.
"""


def task_text(block: str = VALID_BLOCK) -> str:
    return f"# Task\n\n## Context checkpoint\n\n```yaml\n{block}```\n"


class CheckpointValidatorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        contract_path = Path(__file__).resolve().parents[2] / "docs/agents/GOVERNANCE_CONTRACT.json"
        cls.contract = checkpoint.load_contract(contract_path)

    def validate_text(self, text: str, *, require_checkpoint: bool = True) -> list[str]:
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "task.md"
            path.write_text(text, encoding="utf-8")
            return checkpoint.validate_task(
                path,
                self.contract,
                require_checkpoint=require_checkpoint,
            )

    def test_valid_checkpoint_passes(self) -> None:
        self.assertEqual([], self.validate_text(task_text()))

    def test_waiting_status_passes(self) -> None:
        block = VALID_BLOCK.replace("status: implementing", "status: waiting")
        self.assertEqual([], self.validate_text(task_text(block)))

    def test_completed_status_passes(self) -> None:
        block = VALID_BLOCK.replace("status: implementing", "status: completed")
        self.assertEqual([], self.validate_text(task_text(block)))

    def test_not_applicable_validation_passes_with_evidence(self) -> None:
        block = VALID_BLOCK.replace("result: PASS", "result: NOT_APPLICABLE").replace(
            "evidence: local unittest",
            "evidence: documentation-only task has no runtime E2E surface",
        )
        self.assertEqual([], self.validate_text(task_text(block)))

    def test_missing_checkpoint_fails(self) -> None:
        errors = self.validate_text("# Task\n")
        self.assertTrue(any("missing ## Context checkpoint section" in error for error in errors))

    def test_missing_next_action_fails(self) -> None:
        block = VALID_BLOCK.replace(
            "next_action: Open the draft pull request and inspect its current-head checks.\n",
            "",
        )
        errors = self.validate_text(task_text(block))
        self.assertTrue(any("missing checkpoint field next_action" in error for error in errors))

    def test_duplicate_next_action_fails(self) -> None:
        block = VALID_BLOCK + "next_action: Run another step.\n"
        errors = self.validate_text(task_text(block))
        self.assertTrue(any("duplicate top-level key 'next_action'" in error for error in errors))

    def test_unsupported_status_fails(self) -> None:
        block = VALID_BLOCK.replace("status: implementing", "status: done")
        errors = self.validate_text(task_text(block))
        self.assertTrue(any("unsupported checkpoint status 'done'" in error for error in errors))

    def test_unsupported_validation_result_fails(self) -> None:
        block = VALID_BLOCK.replace("result: PASS", "result: SUCCESS")
        errors = self.validate_text(task_text(block))
        self.assertTrue(any("unsupported result 'SUCCESS'" in error for error in errors))

    def test_placeholder_next_action_fails(self) -> None:
        block = VALID_BLOCK.replace(
            "next_action: Open the draft pull request and inspect its current-head checks.",
            "next_action: TBD",
        )
        errors = self.validate_text(task_text(block))
        self.assertTrue(any("next_action must be one concrete next step" in error for error in errors))

    def test_evidence_state_overlap_fails(self) -> None:
        block = VALID_BLOCK.replace(
            "unknown:\n  - CI result not known yet",
            "unknown:\n  - source inspection completed",
        )
        errors = self.validate_text(task_text(block))
        self.assertTrue(any("evidence fact appears in both" in error for error in errors))

    def test_wrong_checkpoint_version_fails(self) -> None:
        block = VALID_BLOCK.replace("checkpoint_version: 1", "checkpoint_version: 2")
        errors = self.validate_text(task_text(block))
        self.assertTrue(any("checkpoint_version must be 1" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
