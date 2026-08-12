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

    def test_outer_fenced_status_example_is_not_authoritative(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "docs/agents/AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8") + "\n````markdown\nUse these checkpoint task statuses only:\n\n```text\ninvestigating | implementing | validating | ready | waiting | blocked\n```\n````\n", encoding="utf-8")
        self.assertEqual([], validate_policy(root))

    def test_nested_yaml_status_example_is_not_authoritative(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md"
        path.write_text(path.read_text(encoding="utf-8") + "\n````markdown\n```yaml\ncheckpoint_task_statuses:\n  - investigating\n  - implementing\n  - validating\n  - ready\n  - waiting\n  - blocked\n```\n````\n", encoding="utf-8")
        self.assertEqual([], validate_policy(root))

    def test_combined_emphasis_conflicting_status_declaration_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "docs/agents/AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8") + "\n***Use these checkpoint task statuses only:***\n\n```text\ninvestigating | implementing | validating | ready | waiting | blocked\n```\n", encoding="utf-8")
        self.assertIn("checkpoint task statuses drift", self._findings(root))

    def test_nested_emphasis_conflicting_status_declaration_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "docs/agents/AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8") + "\n**_Use these checkpoint task statuses only:_**\n\n```text\ninvestigating | implementing | validating | ready | waiting | blocked\n```\n", encoding="utf-8")
        self.assertIn("checkpoint task statuses drift", self._findings(root))

    def test_duplicate_conflicting_terminal_declaration_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "docs/agents/AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8") + "\nUse these terminal invocation results only:\n\n```text\nDONE | WAITING | BLOCKED\n```\n", encoding="utf-8")
        self.assertIn("terminal invocation results drift", self._findings(root))

    def test_wrapped_override_status_declaration_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_override(root, "- checkpoint task status:\n  `investigating` `implementing` `validating` `ready` `waiting` `blocked`")
        self.assertIn("checkpoint task statuses drift", self._findings(root))

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

    def test_fenced_budget_example_is_not_authoritative(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_override(root, "````markdown\nDefault to 999 minutes per foreground invocation\n```text\nDefault to 998 minutes per foreground invocation\n```\n````")
        self.assertEqual([], validate_policy(root))

    def test_nested_yaml_budget_example_is_not_authoritative(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md"
        path.write_text(path.read_text(encoding="utf-8") + "\n````markdown\n```yaml\nnormal_foreground_runtime_minutes: 999\n```\n````\n", encoding="utf-8")
        self.assertEqual([], validate_policy(root))

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

    def test_permission_based_foreign_repository_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents have permission to edit acme/production autonomously.")
        self.assertIn("acme/production", self._findings(root))

    def test_have_write_access_foreign_repository_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents have write access to acme/production.")
        self.assertIn("acme/production", self._findings(root))

    def test_has_write_access_foreign_repository_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- An agent has write access to acme/production.")
        self.assertIn("acme/production", self._findings(root))

    def test_mandatory_must_foreign_repository_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents must edit acme/production autonomously.")
        self.assertIn("acme/production", self._findings(root))

    def test_mandatory_shall_foreign_repository_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents shall edit acme/production autonomously.")
        self.assertIn("acme/production", self._findings(root))

    def test_required_foreign_repository_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents are required to edit acme/production autonomously.")
        self.assertIn("acme/production", self._findings(root))

    def test_intervening_edit_object_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may edit files in acme/production autonomously.")
        self.assertIn("acme/production", self._findings(root))

    def test_intervening_push_object_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may push changes to acme/production autonomously.")
        self.assertIn("acme/production", self._findings(root))

    def test_gerund_repository_edit_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Editing acme/production is explicitly allowed.")
        self.assertIn("acme/production", self._findings(root))

    def test_passive_repository_edit_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- acme/production may be edited autonomously.")
        self.assertIn("acme/production", self._findings(root))

    def test_passive_repository_modify_grant_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- acme/production is allowed to be modified autonomously.")
        self.assertIn("acme/production", self._findings(root))

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

    def test_read_authorization_does_not_unlock_foreign_repository_writes(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to acme/production are allowed only when the project owner explicitly authorizes read access in the current task.")
        self.assertIn("acme/production", self._findings(root))

    def test_other_repository_authorization_does_not_unlock_target_writes(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to acme/production are allowed only when the user explicitly authorizes acme/other in the current task.")
        self.assertIn("acme/production", self._findings(root))

    def test_matching_write_authorization_is_not_a_contradiction(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to acme/production are allowed only when the project owner explicitly authorizes write access to acme/production in the current task.")
        self.assertEqual([], validate_policy(root))

    def test_unless_authorized_is_not_an_exception(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to acme/production are allowed unless the user explicitly authorizes that repository in the current task.")
        self.assertIn("acme/production", self._findings(root))

    def test_passive_explicit_authorization_is_not_a_contradiction(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to acme/production are allowed only when explicit permission is granted by the project owner for the current task.")
        self.assertEqual([], validate_policy(root))

    def test_denied_passive_authorization_is_not_an_exception(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to acme/production are allowed only when explicit permission is denied by the project owner for the current task.")
        self.assertIn("acme/production", self._findings(root))

    def test_withheld_passive_authorization_is_not_an_exception(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to acme/production are allowed only when explicit permission is withheld by the project owner for the current task.")
        self.assertIn("acme/production", self._findings(root))

    def test_negated_passive_authorization_is_not_an_exception(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to acme/production are allowed only when explicit permission is not granted by the project owner for the current task.")
        self.assertIn("acme/production", self._findings(root))

    def test_condition_conjunction_stays_in_same_authorization_clause(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to acme/production are allowed only when the user explicitly authorizes it and the authorization applies to the current task.")
        self.assertEqual([], validate_policy(root))

    def test_condition_gerund_stays_in_same_authorization_clause(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to acme/production are allowed only when the user explicitly authorizes it and authorization applies to the current task before editing begins.")
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

    def test_authorization_exception_split_after_indefinite_agent(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to `blakinio/canary` are allowed only when the user explicitly authorizes it in the current task and an agent may edit `acme/production` autonomously.")
        findings = self._findings(root)
        self.assertIn("acme/production", findings)
        self.assertNotIn("authoritative policy: blakinio/canary", findings)

    def test_authorization_exception_split_after_modal_adverb(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to `blakinio/canary` are allowed only when the user explicitly authorizes it in the current task and agents may also edit acme/production autonomously.")
        findings = self._findings(root)
        self.assertIn("acme/production", findings)
        self.assertNotIn("authoritative policy: blakinio/canary", findings)

    def test_authorization_exception_split_after_passive_allowed_grant(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to `blakinio/canary` are allowed only when the user explicitly authorizes it in the current task and agents are allowed to edit acme/production autonomously.")
        findings = self._findings(root)
        self.assertIn("acme/production", findings)
        self.assertNotIn("authoritative policy: blakinio/canary", findings)

    def test_authorization_exception_split_after_permission_grant(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Writes to `blakinio/canary` are allowed only when the user explicitly authorizes it in the current task and agents have permission to edit acme/production autonomously.")
        findings = self._findings(root)
        self.assertIn("acme/production", findings)
        self.assertNotIn("authoritative policy: blakinio/canary", findings)

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

    def test_mandatory_mutation_denial_is_not_a_grant(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents must not edit acme/production autonomously.")
        self.assertEqual([], validate_policy(root))

    def test_shared_modal_mutation_denial_is_not_a_grant(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may inspect and not edit acme/production.")
        self.assertEqual([], validate_policy(root))

    def test_refrain_from_mutation_is_not_a_grant(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may refrain from editing acme/production.")
        self.assertEqual([], validate_policy(root))

    def test_refrain_denial_does_not_hide_later_affirmative_grant(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may refrain from editing acme/production, but agents may edit acme/other autonomously.")
        findings = self._findings(root)
        self.assertIn("acme/other", findings)
        self.assertNotIn("authoritative policy: acme/production", findings)

    def test_negated_repository_mutation_stays_denied_with_other_positive_mutation(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may edit local documentation and not edit acme/production.")
        self.assertEqual([], validate_policy(root))

    def test_do_not_grant_write_access_is_not_a_grant(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Do not grant write access to acme/production.")
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

    def test_quoted_repository_with_metadata_suffix_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may edit `acme/production` metadata autonomously.")
        self.assertIn("acme/production", self._findings(root))

    def test_quoted_repository_with_underscore_and_metadata_suffix_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_root(root, "- Agents may edit `acme/prod_repo` metadata autonomously.")
        self.assertIn("acme/prod_repo", self._findings(root))

    def test_fenced_example_is_not_authoritative_policy(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_override(root, "```text\nAgents may edit acme/production autonomously.\n```")
        self.assertEqual([], validate_policy(root))

    def test_long_fence_does_not_close_on_short_inner_fence(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_override(root, "````text\nouter example\n```\nAgents may edit acme/production autonomously.\n```\n````")
        self.assertEqual([], validate_policy(root))

    def test_fence_info_string_is_not_a_closer(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._append_override(root, "```text\nouter example\n```python\nAgents may edit acme/production autonomously.\n```")
        self.assertEqual([], validate_policy(root))

    def test_closeout_marker_drift_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        self._replace(root, "docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md", "## Related PR hygiene", "## Related change hygiene")
        self.assertIn("## Related PR hygiene", self._findings(root))


if __name__ == "__main__":
    unittest.main(verbosity=2)