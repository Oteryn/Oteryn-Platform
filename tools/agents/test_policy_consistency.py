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

    def _append_root(self, root: Path, line: str) -> None:
        path = root / "AGENTS.md"
        text = path.read_text(encoding="utf-8")
        marker = "- Do not push Oteryn Platform code into the Canary repository."
        self.assertIn(marker, text)
        path.write_text(text.replace(marker, marker + "\n" + line, 1), encoding="utf-8")

    def _append_override(self, root: Path, line: str) -> None:
        path = root / "AGENTS.override.md"
        path.write_text(path.read_text(encoding="utf-8") + "\n" + line + "\n", encoding="utf-8")

    def _findings(self, root: Path) -> str:
        return "\n".join(validate_policy(root))

    def test_current_repository_policy_is_consistent(self) -> None:
        self.assertEqual([], validate_policy(REPO_ROOT))

    def test_checkpoint_status_drift_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._replace(root, "docs/agents/AGENTS.md", "investigating | implementing | validating | ready | waiting | blocked | completed", "investigating | implementing | validating | ready | waiting | blocked")
        self.assertIn("checkpoint task statuses drift", self._findings(root))

    def test_duplicate_conflicting_status_declaration_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "docs/agents/AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8") + "\nUse these checkpoint task statuses only:\n\n```text\ninvestigating | implementing | validating | ready | waiting | blocked\n```\n", encoding="utf-8")
        self.assertIn("checkpoint task statuses drift", self._findings(root))

    def test_combined_emphasis_conflicting_status_declaration_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "docs/agents/AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8") + "\n***Use these checkpoint task statuses only:***\n\n```text\ninvestigating | implementing | validating | ready | waiting | blocked\n```\n", encoding="utf-8")
        self.assertIn("checkpoint task statuses drift", self._findings(root))

    def test_duplicate_conflicting_terminal_declaration_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "docs/agents/AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8") + "\nUse these terminal invocation results only:\n\n```text\nDONE | WAITING | BLOCKED\n```\n", encoding="utf-8")
        self.assertIn("terminal invocation results drift", self._findings(root))

    def test_budget_drift_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._replace(root, "AGENTS.override.md", "Default to 60 minutes per foreground invocation", "Default to 61 minutes per foreground invocation")
        self.assertIn("normal_foreground_runtime_minutes drift", self._findings(root))

    def test_duplicate_conflicting_budget_declaration_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_override(root, "Default to 999 minutes per foreground invocation")
        findings = self._findings(root)
        self.assertIn("normal_foreground_runtime_minutes drift", findings)
        self.assertIn("999", findings)

    def test_markdown_duplicate_conflicting_budget_declaration_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_override(root, "Default to **999** minutes per foreground invocation")
        findings = self._findings(root)
        self.assertIn("normal_foreground_runtime_minutes drift", findings)
        self.assertIn("999", findings)

    def test_repository_scope_marker_drift_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._replace(root, "AGENTS.md", "The only repository where autonomous write operations are allowed by this file is `blakinio/Oteryn-Platform`.", "The only repository where autonomous write operations are allowed by this file is `blakinio/other`.")
        self.assertIn("missing required governance marker", self._findings(root))

    def test_unquoted_unknown_owner_edit_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may edit acme/production autonomously.")
        findings = self._findings(root)
        self.assertIn("contradictory repository mutation authorization", findings)
        self.assertIn("acme/production", findings)

    def test_slash_term_repository_client_server_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may edit client/server autonomously.")
        self.assertIn("client/server", self._findings(root))

    def test_slash_term_repository_read_write_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may edit read/write autonomously.")
        self.assertIn("read/write", self._findings(root))

    def test_unquoted_known_owner_write_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Autonomous writes to blakinio/other are explicitly allowed.")
        self.assertIn("blakinio/other", self._findings(root))

    def test_override_foreign_repository_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_override(root, "Autonomous pushes to acme/production are allowed.")
        findings = self._findings(root)
        self.assertIn("AGENTS.override.md: contradictory repository mutation authorization", findings)
        self.assertIn("acme/production", findings)

    def test_asserted_current_task_authorization_is_not_an_exception(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- The user explicitly authorizes writes to acme/production in the current task.")
        self.assertIn("acme/production", self._findings(root))

    def test_explicit_user_authorization_exception_is_not_a_contradiction(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to `acme/production` are allowed only when the user explicitly authorizes that repository in the current task.")
        self.assertEqual([], validate_policy(root))

    def test_authorization_exception_is_scoped_to_repository(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- `blakinio/canary` is writable only when the user explicitly authorizes it in the current task, but autonomous writes to `blakinio/other` are allowed.")
        findings = self._findings(root)
        self.assertIn("blakinio/other", findings)
        self.assertNotIn("authoritative policy: blakinio/canary", findings)

    def test_authorization_exception_split_after_and_additionally(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to `blakinio/canary` are allowed only when the user explicitly authorizes it in the current task and additionally autonomous writes to `blakinio/other` are allowed.")
        self.assertIn("blakinio/other", self._findings(root))

    def test_authorization_exception_split_after_and_the_agents(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to `blakinio/canary` are allowed only when the user explicitly authorizes it in the current task and the agents may edit `blakinio/other` autonomously.")
        self.assertIn("blakinio/other", self._findings(root))

    def test_wrapped_markdown_foreign_write_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Autonomous writes to acme/production\n  are explicitly allowed.")
        self.assertIn("acme/production", self._findings(root))

    def test_read_only_exemption_is_scoped_to_repository(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- `blakinio/canary` is read-only, but autonomous writes to `blakinio/other` are allowed.")
        findings = self._findings(root)
        self.assertIn("blakinio/other", findings)
        self.assertNotIn("authoritative policy: blakinio/canary", findings)

    def test_negated_read_only_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- `acme/production` isn't read-only; autonomous writes to `acme/production` are allowed.")
        self.assertIn("acme/production", self._findings(root))

    def test_denied_foreign_repository_mutation_is_not_a_grant(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents are not allowed to edit acme/production autonomously.")
        self.assertEqual([], validate_policy(root))

    def test_unauthorized_foreign_repository_mutation_is_not_a_grant(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Unauthorized mutation of acme/production is forbidden.")
        self.assertEqual([], validate_policy(root))

    def test_slash_prose_is_not_treated_as_repository(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_override(root, "An intermediate message is allowed only after commit/PR creation; authentication/session mutation remains unauthorized.")
        self.assertEqual([], validate_policy(root))

    def test_mutation_adjacent_slash_prose_is_not_repository(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may edit commit/PR metadata autonomously.")
        self.assertEqual([], validate_policy(root))

    def test_closeout_marker_drift_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._replace(root, "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md", "## Related PR hygiene", "## Related change hygiene")
        self.assertIn("## Related PR hygiene", self._findings(root))


if __name__ == "__main__":
    unittest.main(verbosity=2)
