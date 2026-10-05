#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "ci" / "heavy_evidence_reuse.py"
SCRIPT_DIR = SCRIPT.parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

spec = importlib.util.spec_from_file_location("heavy_evidence_reuse", SCRIPT)
if spec is None or spec.loader is None:
    raise RuntimeError(f"Unable to load {SCRIPT}")
reuse = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = reuse
spec.loader.exec_module(reuse)


class HeavyEvidenceReuseTest(unittest.TestCase):
    def decide(self):
        return reuse.decide_reuse(
            repository="Oteryn/Oteryn-Platform",
            pr_number=1012,
            workflow="edge-security-emulation.yml",
            job_name="validate",
            head_sha="a" * 40,
            token="token",
        )

    def test_material_change_runs_heavy(self) -> None:
        with (
            patch.object(reuse, "material_tree_digest", return_value="digest"),
            patch.object(reuse, "commit_parents", return_value=["b" * 40]),
            patch.object(reuse, "diff_paths", return_value=["app/Runtime.php"]),
            patch.object(reuse, "find_successful_heavy_run") as evidence,
        ):
            decision = self.decide()

        self.assertFalse(decision.reuse)
        self.assertIn("material", decision.reason)
        evidence.assert_not_called()

    def test_checkpoint_only_successor_reuses_exact_successful_heavy_run(self) -> None:
        digests = {"a" * 40: "digest", "b" * 40: "digest"}
        with (
            patch.object(reuse, "material_tree_digest", side_effect=lambda sha: digests[sha]),
            patch.object(reuse, "commit_parents", return_value=["b" * 40]),
            patch.object(
                reuse,
                "diff_paths",
                return_value=["docs/agents/tasks/active/checkpoint.md"],
            ),
            patch.object(reuse, "find_successful_heavy_run", return_value=4242),
        ):
            decision = self.decide()

        self.assertTrue(decision.reuse)
        self.assertEqual("b" * 40, decision.reused_head_sha)
        self.assertEqual(4242, decision.reused_run_id)
        self.assertEqual("digest", decision.material_tree_digest)

    def test_consecutive_checkpoint_commits_walk_back_to_material_evidence(self) -> None:
        head = "a" * 40
        docs_parent = "b" * 40
        material_head = "c" * 40
        parents = {
            head: [docs_parent],
            docs_parent: [material_head],
        }
        paths = {
            (docs_parent, head): ["docs/agents/tasks/active/checkpoint.md"],
            (material_head, docs_parent): ["docs/agents/evidence/result.md"],
        }
        runs = {docs_parent: None, material_head: 5151}

        with (
            patch.object(reuse, "material_tree_digest", return_value="same-digest"),
            patch.object(reuse, "commit_parents", side_effect=lambda sha: parents[sha]),
            patch.object(reuse, "diff_paths", side_effect=lambda base, child: paths[(base, child)]),
            patch.object(
                reuse,
                "find_successful_heavy_run",
                side_effect=lambda **kwargs: runs[kwargs["head_sha"]],
            ),
        ):
            decision = self.decide()

        self.assertTrue(decision.reuse)
        self.assertEqual(material_head, decision.reused_head_sha)
        self.assertEqual(5151, decision.reused_run_id)

    def test_material_change_after_checkpoint_runs_heavy_again(self) -> None:
        with (
            patch.object(reuse, "material_tree_digest", return_value="digest"),
            patch.object(reuse, "commit_parents", return_value=["b" * 40]),
            patch.object(
                reuse,
                "diff_paths",
                return_value=[
                    "docs/agents/tasks/active/checkpoint.md",
                    "routes/web.php",
                ],
            ),
            patch.object(reuse, "find_successful_heavy_run") as evidence,
        ):
            decision = self.decide()

        self.assertFalse(decision.reuse)
        evidence.assert_not_called()

    def test_ambiguous_history_fails_closed_to_heavy(self) -> None:
        with (
            patch.object(reuse, "material_tree_digest", return_value="digest"),
            patch.object(
                reuse,
                "commit_parents",
                return_value=["b" * 40, "c" * 40],
            ),
        ):
            decision = self.decide()

        self.assertFalse(decision.reuse)
        self.assertIn("ambiguous", decision.reason)

    def test_material_digest_mismatch_fails_closed(self) -> None:
        digests = {"a" * 40: "current", "b" * 40: "parent"}
        with (
            patch.object(reuse, "material_tree_digest", side_effect=lambda sha: digests[sha]),
            patch.object(reuse, "commit_parents", return_value=["b" * 40]),
            patch.object(reuse, "diff_paths", return_value=["docs/agents/checkpoint.md"]),
            patch.object(reuse, "find_successful_heavy_run") as evidence,
        ):
            decision = self.decide()

        self.assertFalse(decision.reuse)
        self.assertIn("digest changed", decision.reason)
        evidence.assert_not_called()

    def test_missing_token_fails_closed(self) -> None:
        decision = reuse.decide_reuse(
            repository="Oteryn/Oteryn-Platform",
            pr_number=1012,
            workflow="edge-security-emulation.yml",
            job_name="validate",
            head_sha="a" * 40,
            token="",
        )
        self.assertFalse(decision.reuse)
        self.assertIn("token unavailable", decision.reason)

    def test_empty_latest_diff_is_ambiguous_not_reusable(self) -> None:
        with (
            patch.object(reuse, "material_tree_digest", return_value="digest"),
            patch.object(reuse, "commit_parents", return_value=["b" * 40]),
            patch.object(reuse, "diff_paths", return_value=[]),
        ):
            decision = self.decide()

        self.assertFalse(decision.reuse)

    def test_reusable_path_contract_matches_classifier(self) -> None:
        self.assertTrue(
            reuse.reusable_successor_paths(
                [
                    "docs/agents/tasks/active/checkpoint.md",
                    "docs/maintenance/evidence.md",
                ]
            )
        )
        self.assertFalse(
            reuse.reusable_successor_paths(
                [
                    "docs/agents/tasks/active/checkpoint.md",
                    "docs/contracts/runtime-contract.md",
                ]
            )
        )


if __name__ == "__main__":
    unittest.main()
