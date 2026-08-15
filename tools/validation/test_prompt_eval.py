#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from prompt_eval import PromptEvalError, REQUIRED_CATEGORIES, validate_suite


class PromptEvalTest(unittest.TestCase):
    def make_repository(self, *, mutate=None) -> tuple[Path, Path]:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        source = root / "docs/prompt.md"
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_text("ALLOW\nSAFE\n", encoding="utf-8")

        cases = []
        categories = sorted(REQUIRED_CATEGORIES)
        for index, category in enumerate(categories):
            case = {
                "id": f"case-{index}-{category}",
                "category": category,
                "source": "docs/prompt.md",
                "must_contain": ["ALLOW"],
            }
            if category in {"boundary_refusal", "authority_stop", "prompt_injection"}:
                case["safety_critical"] = True
            cases.append(case)

        suite = {
            "schema_version": 1,
            "id": "test-suite",
            "mode": "deterministic_text_contract",
            "eval_policy": {
                "minimum_model_trials_when_nondeterminism_matters": 3,
                "deterministic_checks": 1,
                "maximum_regression_on_safety_critical_cases": 0,
            },
            "limitations": (
                "This does not execute an LLM; it is deterministic and does not prove stochastic "
                "adherence, so model/runtime trials remain required when behaviour changes."
            ),
            "required_categories": categories,
            "cases": cases,
        }
        if mutate is not None:
            mutate(root, suite)

        suite_path = root / "suite.json"
        suite_path.write_text(json.dumps(suite, indent=2) + "\n", encoding="utf-8")
        return root, Path("suite.json")

    def test_balanced_suite_passes_without_claiming_model_trials(self) -> None:
        root, suite = self.make_repository()
        result = validate_suite(root, suite)
        self.assertEqual(len(REQUIRED_CATEGORIES), result["categories"])
        self.assertEqual(0, result["model_trials_executed"])
        self.assertEqual(3, result["safety_critical_cases"])

    def test_missing_required_marker_fails(self) -> None:
        def mutate(_root, suite):
            suite["cases"][0]["must_contain"] = ["MISSING"]

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "missing required marker"):
            validate_suite(root, suite)

    def test_forbidden_marker_fails(self) -> None:
        def mutate(_root, suite):
            suite["cases"][0]["must_not_contain"] = ["SAFE"]

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "contains forbidden marker"):
            validate_suite(root, suite)

    def test_missing_category_fails(self) -> None:
        def mutate(_root, suite):
            removed = suite["required_categories"].pop()
            suite["cases"] = [case for case in suite["cases"] if case["category"] != removed]

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "required_categories drift"):
            validate_suite(root, suite)

    def test_fewer_than_three_safety_cases_fails(self) -> None:
        def mutate(_root, suite):
            safety = [case for case in suite["cases"] if case.get("safety_critical")]
            safety[0].pop("safety_critical")

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "at least three"):
            validate_suite(root, suite)

    def test_source_path_escape_fails(self) -> None:
        def mutate(_root, suite):
            suite["cases"][0]["source"] = "../outside.md"

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "repository-relative"):
            validate_suite(root, suite)

    def test_limitations_must_disclaim_model_execution(self) -> None:
        def mutate(_root, suite):
            suite["limitations"] = "Automated prompt eval."

        root, suite = self.make_repository(mutate=mutate)
        with self.assertRaisesRegex(PromptEvalError, "deterministic scope"):
            validate_suite(root, suite)

    def test_portal_completion_scope_manifest_contract(self) -> None:
        root = Path(__file__).resolve().parents[2]
        scope_path = root / "docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json"
        scope = json.loads(scope_path.read_text(encoding="utf-8"))

        self.assertEqual(1, scope["schema_version"])
        self.assertEqual("OTERYN_PORTAL_COMPLETION", scope["programme"])
        self.assertEqual("non_scheduling_completion_scope", scope["mode"])
        self.assertEqual(
            "docs/agents/programs/OTERYN_PORTAL_COMPLETION.md",
            scope["selection_authority"],
        )

        rules = scope["rules"]
        for key in ("selects_work", "claims_ownership", "proves_live_state", "can_promote_ready"):
            self.assertIs(False, rules[key], f"{key} must stay fail-closed/non-scheduling")
        self.assertIs(True, rules["live_selector_state_required"])
        self.assertIs(True, rules["higher_authority_overrides_projection"])

        allowed = {"REQUIRED", "CONDITIONAL", "DEFERRED", "REJECTED"}
        self.assertEqual(allowed, set(scope["allowed_dispositions"]))

        workstreams = scope["workstreams"]
        self.assertTrue(workstreams)
        ids = [item["id"] for item in workstreams]
        self.assertEqual(len(ids), len(set(ids)), "portal completion workstream ids must be unique")

        forbidden_live_fields = {
            "status",
            "current_status",
            "selector_state",
            "ready",
            "owned",
            "branch",
            "pull_request",
            "head",
        }
        for item in workstreams:
            self.assertIn(item["disposition"], allowed)
            self.assertIsInstance(item["boundary"], str)
            self.assertTrue(item["boundary"].strip())
            selector_entry = item["selector_entry"]
            self.assertTrue(
                selector_entry is None or (isinstance(selector_entry, int) and 1 <= selector_entry <= 12),
                f"invalid selector_entry for {item['id']}",
            )
            if item["disposition"] == "CONDITIONAL":
                self.assertIsInstance(item["activation_trigger"], str)
                self.assertTrue(item["activation_trigger"].strip())
            self.assertFalse(
                forbidden_live_fields.intersection(item),
                f"{item['id']} must not persist mutable live selector/ownership state",
            )

        by_id = {item["id"]: item for item in workstreams}
        expected = {
            "portal_control_plane": ("REQUIRED", 2),
            "launch_critical_remediation": ("CONDITIONAL", 3),
            "production_public_edge_proof": ("REQUIRED", 4),
            "core_account_character_portfolio": ("REQUIRED", 5),
            "liveops": ("REQUIRED", 6),
            "public_today": ("REQUIRED", 6),
            "private_today": ("CONDITIONAL", 6),
            "federated_search": ("REQUIRED", 7),
            "client_distribution_platform": ("CONDITIONAL", 8),
            "wiki_expected_inventory": ("REQUIRED", 9),
            "game_catalog_expected_inventory": ("REQUIRED", 9),
            "multi_world_ruleset_season_dimensions": ("CONDITIONAL", None),
            "player_companion_foundation": ("REQUIRED", 10),
            "player_companion_followups": ("CONDITIONAL", 10),
            "platform_api": ("DEFERRED", None),
            "public_game_data_richer_community_reads": ("DEFERRED", 11),
            "forum": ("DEFERRED", 11),
            "read_scaling_dedicated_index": ("DEFERRED", 11),
            "world_hub": ("DEFERRED", 11),
            "commerce_capability": ("REQUIRED", 12),
            "commerce_production_activation": ("DEFERRED", 12),
        }
        self.assertEqual(set(expected), set(by_id), "portal completion canonical workstream inventory drift")
        for workstream_id, (disposition, selector_entry) in expected.items():
            self.assertEqual(disposition, by_id[workstream_id]["disposition"])
            self.assertEqual(selector_entry, by_id[workstream_id]["selector_entry"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
