#!/usr/bin/env python3
from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import historical_branch_policy as policy  # noqa: E402


def branch(
    name: str,
    sha: str,
    disposition: str,
    *,
    active: bool = False,
    reserved: bool = False,
    ahead: int = 1,
) -> dict:
    return {
        "active_claims": ["task.md"] if active else [],
        "branch": name,
        "disposition": disposition,
        "head_sha": sha,
        "open_pr_numbers": [1] if disposition == "OPEN_PR" else [],
        "protected": disposition == "PROTECTED",
        "pull_history": [],
        "reason": "initial",
        "relation": {"ahead": ahead, "proof": "UNIQUE_HISTORY_REMAINS"},
        "reserved_purpose_name": reserved,
    }


class CollapseTests(unittest.TestCase):
    def collapse(self, branches: list[dict], ancestry: set[tuple[str, str]]) -> list[dict]:
        return policy.collapse_redundant_refs(
            copy.deepcopy(branches),
            ancestor_predicate=lambda ancestor, descendant: ancestor == descendant
            or (ancestor, descendant) in ancestry,
        )

    @staticmethod
    def by_name(result: list[dict]) -> dict[str, dict]:
        return {item["branch"]: item for item in result}

    def test_generic_ref_reachable_from_open_pr_is_delete(self) -> None:
        result = self.by_name(
            self.collapse(
                [
                    branch("old", "a" * 40, "RETAIN"),
                    branch("live", "b" * 40, "OPEN_PR", ahead=2),
                ],
                {("a" * 40, "b" * 40)},
            )
        )
        self.assertEqual(result["old"]["disposition"], "DELETE")
        self.assertEqual(
            result["old"]["cleanup_proof"],
            "REACHABLE_FROM_LIVE_ANCHOR:live",
        )
        self.assertEqual(result["live"]["disposition"], "OPEN_PR")

    def test_active_task_ref_is_anchor_and_never_collapsed(self) -> None:
        result = self.by_name(
            self.collapse(
                [
                    branch("older", "a" * 40, "RETAIN"),
                    branch("active", "b" * 40, "RETAIN", active=True, ahead=2),
                ],
                {("a" * 40, "b" * 40)},
            )
        )
        self.assertEqual(result["older"]["disposition"], "DELETE")
        self.assertEqual(result["active"]["disposition"], "RETAIN")
        self.assertEqual(
            result["older"]["cleanup_proof"],
            "REACHABLE_FROM_LIVE_ANCHOR:active",
        )

    def test_recovery_ref_is_anchor_and_never_collapsed(self) -> None:
        result = self.by_name(
            self.collapse(
                [
                    branch("older", "a" * 40, "RETAIN"),
                    branch("backup/recovery", "b" * 40, "RECOVERY", reserved=True, ahead=2),
                ],
                {("a" * 40, "b" * 40)},
            )
        )
        self.assertEqual(result["older"]["disposition"], "DELETE")
        self.assertEqual(result["backup/recovery"]["disposition"], "RECOVERY")

    def test_duplicate_generic_refs_keep_one_deterministic_anchor(self) -> None:
        result = self.by_name(
            self.collapse(
                [
                    branch("x", "a" * 40, "RETAIN"),
                    branch("x2", "a" * 40, "RETAIN"),
                    branch("x3", "a" * 40, "RETAIN"),
                ],
                set(),
            )
        )
        self.assertEqual(result["x"]["disposition"], "RETAIN")
        self.assertEqual(result["x2"]["disposition"], "DELETE")
        self.assertEqual(result["x3"]["disposition"], "DELETE")
        self.assertEqual(
            result["x2"]["cleanup_proof"],
            "DUPLICATE_HEAD_RETAINED_AS:x",
        )

    def test_unique_generic_ancestor_of_other_generic_is_still_retained(self) -> None:
        result = self.by_name(
            self.collapse(
                [
                    branch("older-semantic-label", "a" * 40, "RETAIN", ahead=1),
                    branch("newer-unclaimed", "b" * 40, "RETAIN", ahead=3),
                ],
                {("a" * 40, "b" * 40)},
            )
        )
        self.assertEqual(result["older-semantic-label"]["disposition"], "RETAIN")
        self.assertEqual(result["newer-unclaimed"]["disposition"], "RETAIN")

    def test_base_delete_keeps_git_proof(self) -> None:
        item = branch("old", "a" * 40, "DELETE")
        item["relation"]["proof"] = "ANCESTOR_OF_MAIN"
        result = self.by_name(self.collapse([item], set()))
        self.assertEqual(result["old"]["cleanup_proof"], "ANCESTOR_OF_MAIN")

    def test_invalid_base_delete_proof_fails_closed(self) -> None:
        item = branch("old", "a" * 40, "DELETE")
        item["relation"]["proof"] = "UNIQUE_HISTORY_REMAINS"
        with self.assertRaises(policy.ValidationError):
            self.collapse([item], set())


if __name__ == "__main__":
    unittest.main(verbosity=2)
