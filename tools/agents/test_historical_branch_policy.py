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
    merged_pr: int | None = None,
    ahead: int = 1,
) -> dict:
    pulls = []
    if merged_pr is not None:
        pulls.append(
            {
                "head_sha": sha,
                "merged": True,
                "merged_at": "2026-08-14T00:00:00Z",
                "number": merged_pr,
                "state": "closed",
                "title": "merged",
            }
        )
    return {
        "active_claims": ["task.md"] if active else [],
        "branch": name,
        "disposition": disposition,
        "head_sha": sha,
        "open_pr_numbers": [1] if disposition == "OPEN_PR" else [],
        "protected": disposition == "PROTECTED",
        "pull_history": pulls,
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

    def by_name(self, result: list[dict]) -> dict[str, dict]:
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
        self.assertIn("REACHABLE_FROM_LIVE_ANCHOR:live", result["old"]["cleanup_proof"])
        self.assertEqual(result["live"]["disposition"], "OPEN_PR")

    def test_active_task_ref_is_never_collapsed(self) -> None:
        result = self.by_name(
            self.collapse(
                [
                    branch("active", "a" * 40, "RETAIN", active=True),
                    branch("later", "b" * 40, "RETAIN", ahead=2),
                ],
                {("a" * 40, "b" * 40)},
            )
        )
        self.assertEqual(result["active"]["disposition"], "RETAIN")

    def test_reserved_recovery_ref_is_never_collapsed(self) -> None:
        result = self.by_name(
            self.collapse(
                [
                    branch("recovery", "a" * 40, "RECOVERY", reserved=True),
                    branch("live", "b" * 40, "OPEN_PR", ahead=2),
                ],
                {("a" * 40, "b" * 40)},
            )
        )
        self.assertEqual(result["recovery"]["disposition"], "RECOVERY")

    def test_exact_merged_pr_head_is_delete_when_not_reserved(self) -> None:
        result = self.by_name(
            self.collapse(
                [branch("merged-work", "a" * 40, "RETAIN", merged_pr=42)],
                set(),
            )
        )
        self.assertEqual(result["merged-work"]["disposition"], "DELETE")
        self.assertEqual(result["merged-work"]["cleanup_proof"], "EXACT_MERGED_PR_HEAD:42")

    def test_reserved_exact_merged_head_stays_recovery(self) -> None:
        result = self.by_name(
            self.collapse(
                [branch("backup/recovery", "a" * 40, "RECOVERY", reserved=True, merged_pr=42)],
                set(),
            )
        )
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
        self.assertIn("DUPLICATE_HEAD_RETAINED_AS:x", result["x2"]["cleanup_proof"])

    def test_nonmaximal_generic_tip_is_delete(self) -> None:
        result = self.by_name(
            self.collapse(
                [
                    branch("old", "a" * 40, "RETAIN", ahead=1),
                    branch("new", "b" * 40, "RETAIN", ahead=3),
                ],
                {("a" * 40, "b" * 40)},
            )
        )
        self.assertEqual(result["old"]["disposition"], "DELETE")
        self.assertEqual(result["new"]["disposition"], "RETAIN")
        self.assertIn("REACHABLE_FROM_RETAINED_TIP:new", result["old"]["cleanup_proof"])

    def test_divergent_generic_tips_are_both_retained(self) -> None:
        result = self.by_name(
            self.collapse(
                [
                    branch("left", "a" * 40, "RETAIN"),
                    branch("right", "b" * 40, "RETAIN"),
                ],
                set(),
            )
        )
        self.assertEqual(result["left"]["disposition"], "RETAIN")
        self.assertEqual(result["right"]["disposition"], "RETAIN")

    def test_base_delete_keeps_git_proof(self) -> None:
        item = branch("old", "a" * 40, "DELETE")
        item["relation"]["proof"] = "ANCESTOR_OF_MAIN"
        result = self.by_name(self.collapse([item], set()))
        self.assertEqual(result["old"]["cleanup_proof"], "ANCESTOR_OF_MAIN")


if __name__ == "__main__":
    unittest.main(verbosity=2)
