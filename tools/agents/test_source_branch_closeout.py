#!/usr/bin/env python3
from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

import source_branch_closeout as closeout


VALID_CLOSEOUT = """## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: durable diagnostic reference owned by platform operations
source_branch_evidence: retained under reviewed PR disposition with quarterly review trigger
```
"""


def task(status: str, closeout_text: str = "") -> str:
    return f"""# task

## Context checkpoint

```yaml
checkpoint_version: 1
status: {status}
```

{closeout_text}
"""


class SourceBranchCloseoutTest(unittest.TestCase):
    def test_completed_active_task_requires_terminal_closeout(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "bad.md").write_text(task("completed"), encoding="utf-8")
            checked, errors = closeout.validate_completed_active_tasks(root)
        self.assertEqual(1, checked)
        self.assertTrue(any("missing ## Source branch closeout" in error for error in errors))

    def test_completed_active_task_rejects_pending_evidence(self):
        text = task(
            "completed",
            """## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary merged PR
source_branch_evidence: pending merge verification
```
""",
        )
        self.assertTrue(
            any(
                "source_branch_evidence" in error
                for error in closeout.validate_closeout_text(text)
            )
        )

    def test_completed_active_task_accepts_terminal_evidence(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "done.md").write_text(task("completed", VALID_CLOSEOUT), encoding="utf-8")
            checked, errors = closeout.validate_completed_active_tasks(root)
        self.assertEqual(1, checked)
        self.assertEqual([], errors)

    def test_noncompleted_active_task_is_not_forced_terminal(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "working.md").write_text(task("validating"), encoding="utf-8")
            checked, errors = closeout.validate_completed_active_tasks(root)
        self.assertEqual(0, checked)
        self.assertEqual([], errors)

    def test_only_changed_archive_records_are_migration_gated(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            archive = repo / "docs/agents/tasks/archive"
            archive.mkdir(parents=True)
            subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=repo, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=repo, check=True)
            (archive / "legacy.md").write_text("# legacy without closeout\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=repo, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
            (archive / "new.md").write_text(VALID_CLOSEOUT, encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "new archive"], cwd=repo, check=True)
            changed = closeout.changed_archive_files(archive, base, repository_root=repo)
        self.assertEqual(["new.md"], [path.name for path in changed])


if __name__ == "__main__":
    unittest.main()
