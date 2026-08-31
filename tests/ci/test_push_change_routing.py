#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
CI_SCRIPTS = REPOSITORY_ROOT / "scripts" / "ci"
PUSH_CLASSIFIER_PATH = CI_SCRIPTS / "classify_push_changes.py"

sys.path.insert(0, str(CI_SCRIPTS))
spec = importlib.util.spec_from_file_location("classify_push_changes", PUSH_CLASSIFIER_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"Unable to load push classifier from {PUSH_CLASSIFIER_PATH}")
push_classifier = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = push_classifier
spec.loader.exec_module(push_classifier)


class PushChangeRoutingTest(unittest.TestCase):
    def assert_all_gates(self, result: dict[str, object]) -> None:
        gates = result["gates"]
        self.assertIsInstance(gates, dict)
        self.assertTrue(all(gates.values()))

    def test_docs_only_push_uses_exact_range_and_skips_heavy_gates(self) -> None:
        with patch.object(
            push_classifier.classify_changes,
            "changed_paths",
            return_value=["docs/agents/checkpoint.md"],
        ) as changed_paths:
            result, fail_closed = push_classifier.classify_push_range("a" * 40, "b" * 40)

        changed_paths.assert_called_once_with("a" * 40, "b" * 40)
        self.assertFalse(fail_closed)
        self.assertEqual(["agent_governance"], result["classes"])
        self.assertFalse(any(result["gates"].values()))

    def test_product_push_uses_exact_range_and_enables_runtime_ci(self) -> None:
        with patch.object(
            push_classifier.classify_changes,
            "changed_paths",
            return_value=["app/Services/NewsPublisher.php"],
        ):
            result, fail_closed = push_classifier.classify_push_range("a" * 40, "b" * 40)

        self.assertFalse(fail_closed)
        self.assertTrue(result["gates"]["ci"])
        self.assertTrue(result["gates"]["phase7"])

    def test_zero_base_sha_fails_closed_without_diff(self) -> None:
        with patch.object(push_classifier.classify_changes, "changed_paths") as changed_paths:
            result, fail_closed = push_classifier.classify_push_range(
                push_classifier.ZERO_SHA, "b" * 40
            )

        changed_paths.assert_not_called()
        self.assertTrue(fail_closed)
        self.assert_all_gates(result)

    def test_merge_group_candidate_without_pr_range_fails_closed(self) -> None:
        """A merge-group event has no pull-request range and must run every gate."""
        with patch.object(push_classifier.classify_changes, "changed_paths") as changed_paths:
            result, fail_closed = push_classifier.classify_push_range("", "b" * 40)

        changed_paths.assert_not_called()
        self.assertTrue(fail_closed)
        self.assert_all_gates(result)

    def test_unusable_range_fails_closed(self) -> None:
        with patch.object(
            push_classifier.classify_changes,
            "changed_paths",
            side_effect=push_classifier.subprocess.CalledProcessError(128, ["git", "diff"]),
        ):
            result, fail_closed = push_classifier.classify_push_range("a" * 40, "b" * 40)

        self.assertTrue(fail_closed)
        self.assert_all_gates(result)

    def test_empty_range_fails_closed(self) -> None:
        with patch.object(
            push_classifier.classify_changes,
            "changed_paths",
            return_value=[],
        ):
            result, fail_closed = push_classifier.classify_push_range("a" * 40, "b" * 40)

        self.assertTrue(fail_closed)
        self.assert_all_gates(result)


if __name__ == "__main__":
    unittest.main()
