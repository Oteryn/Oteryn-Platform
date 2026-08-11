#!/usr/bin/env python3
"""Regression tests for cross-document agent policy consistency."""

from __future__ import annotations

import shutil
import tempfile
import unittest
from pathlib import Path

from policy_consistency import CHECKED_PATHS, REPO_ROOT, validate_policy


class PolicyConsistencyTests(unittest.TestCase):
    def _fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        for relative_path in CHECKED_PATHS:
            source = REPO_ROOT / relative_path
            target = root / relative_path
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
        return temporary, root

    def _replace(self, root: Path, relative_path: str, old: str, new: str) -> None:
        path = root / relative_path
        text = path.read_text(encoding="utf-8")
        self.assertIn(old, text, f"fixture source marker missing in {relative_path}")
        path.write_text(text.replace(old, new, 1), encoding="utf-8")

    def _append_after_allowlist_marker(self, root: Path, line: str) -> None:
        path = root / "AGENTS.md"
        text = path.read_text(encoding="utf-8")
        marker = "- Do not push Oteryn Platform code into the Canary repository."
        self.assertIn(marker, text)
        path.write_text(text.replace(marker, marker + "\n" + line, 1), encoding="utf-8")

    def _append_after_allowlist_section(self, root: Path, line: str) -> None:
        path = root / "AGENTS.md"
        text = path.read_text(encoding="utf-8")
        marker = "## Global context efficiency baseline"
        self.assertIn(marker, text)
        path.write_text(text.replace(marker, line + "\n\n" + marker, 1), encoding="utf-8")

    def test_current_repository_policy_is_consistent(self) -> None:
        self.assertEqual([], validate_policy(REPO_ROOT))

    def test_checkpoint_status_drift_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        self._replace(
            root,
            "docs/agents/AGENTS.md",
            "investigating | implementing | validating | ready | waiting | blocked | completed",
            "investigating | implementing | validating | ready | waiting | blocked",
        )
        findings = "\n".join(validate_policy(root))
        self.assertIn("checkpoint task statuses drift", findings)

    def test_budget_drift_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        self._replace(
            root,
            "AGENTS.override.md",
            "Default to 60 minutes per foreground invocation",
            "Default to 61 minutes per foreground invocation",
        )
        findings = "\n".join(validate_policy(root))
        self.assertIn("normal_foreground_runtime_minutes drift", findings)

    def test_duplicate_conflicting_budget_declaration_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        path = root / "AGENTS.override.md"
        text = path.read_text(encoding="utf-8")
        marker = "Default to 60 minutes per foreground invocation"
        self.assertIn(marker, text)
        path.write_text(text.replace(marker, marker + "\nDefault to 999 minutes per foreground invocation", 1), encoding="utf-8")
        findings = "\n".join(validate_policy(root))
        self.assertIn("normal_foreground_runtime_minutes drift", findings)
        self.assertIn("999", findings)

    def test_markdown_duplicate_conflicting_budget_declaration_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        path = root / "AGENTS.override.md"
        text = path.read_text(encoding="utf-8")
        marker = "Default to 60 minutes per foreground invocation"
        self.assertIn(marker, text)
        path.write_text(text.replace(marker, marker + "\nDefault to **999** minutes per foreground invocation", 1), encoding="utf-8")
        findings = "\n".join(validate_policy(root))
        self.assertIn("normal_foreground_runtime_minutes drift", findings)
        self.assertIn("999", findings)

    def test_repository_scope_drift_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        self._replace(
            root,
            "AGENTS.md",
            "The only repository where autonomous write operations are allowed by this file is `blakinio/Oteryn-Platform`.",
            "The only repository where autonomous write operations are allowed by this file is `blakinio/other`.",
        )
        findings = "\n".join(validate_policy(root))
        self.assertIn("missing required governance marker", findings)

    def test_contradictory_repository_write_grant_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_marker(root, f"- Autonomous write operations are also allowed for `{foreign_repo}`.")
        findings = "\n".join(validate_policy(root))
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_unscoped_explicit_repository_write_grant_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_marker(root, f"- Autonomous writes to `{foreign_repo}` are explicitly allowed.")
        findings = "\n".join(validate_policy(root))
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_asserted_current_task_authorization_is_not_an_exception(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_marker(root, f"- The user explicitly authorizes writes to {foreign_repo} in the current task.")
        findings = "\n".join(validate_policy(root))
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_unquoted_foreign_repository_write_grant_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_marker(root, f"- Autonomous writes to {foreign_repo} are explicitly allowed.")
        findings = "\n".join(validate_policy(root))
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_negated_read_only_foreign_write_grant_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_marker(root, f"- `{foreign_repo}` is no longer read-only; autonomous write operations are allowed.")
        findings = "\n".join(validate_policy(root))
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_contracted_negated_read_only_foreign_write_grant_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_marker(root, f"- `{foreign_repo}` isn't read-only; autonomous write operations are allowed.")
        findings = "\n".join(validate_policy(root))
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_foreign_write_grant_outside_allowlist_section_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_section(root, f"Autonomous writes to {foreign_repo} are explicitly allowed.")
        findings = "\n".join(validate_policy(root))
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_explicit_user_authorization_exception_is_not_a_contradiction(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_marker(
            root,
            f"- Writes to `{foreign_repo}` are allowed only when the user explicitly authorizes that repository in the current task.",
        )
        self.assertEqual([], validate_policy(root))

    def test_wrapped_markdown_foreign_write_grant_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_marker(root, f"- Autonomous writes to {foreign_repo}\n  are explicitly allowed.")
        findings = "\n".join(validate_policy(root))
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_read_only_exemption_is_scoped_to_repository(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        read_only_repo = "blakinio" + "/canary"
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_marker(
            root,
            f"- `{read_only_repo}` is read-only, but autonomous writes to `{foreign_repo}` are allowed.",
        )
        findings = "\n".join(validate_policy(root))
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_override_foreign_repository_grant_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        path = root / "AGENTS.override.md"
        text = path.read_text(encoding="utf-8")
        path.write_text(text + f"\nAutonomous writes to {foreign_repo} are explicitly allowed.\n", encoding="utf-8")
        findings = "\n".join(validate_policy(root))
        self.assertIn("AGENTS.override.md: contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_edit_grant_without_write_word_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_marker(root, f"- Agents may edit {foreign_repo} autonomously.")
        findings = "\n".join(validate_policy(root))
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_push_grant_without_write_word_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        foreign_repo = "blakinio" + "/other"
        self._append_after_allowlist_marker(root, f"- Autonomous pushes to {foreign_repo} are allowed.")
        findings = "\n".join(validate_policy(root))
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn(foreign_repo, findings)

    def test_closeout_marker_drift_fails_closed(self) -> None:
        temporary, root = self._fixture()
        self.addCleanup(temporary.cleanup)
        self._replace(
            root,
            "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md",
            "## Related PR hygiene",
            "## Related change hygiene",
        )
        findings = "\n".join(validate_policy(root))
        self.assertIn("## Related PR hygiene", findings)


if __name__ == "__main__":
    unittest.main()
