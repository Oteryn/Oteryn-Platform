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

        expected_root_keys = {
            "schema_version",
            "programme",
            "mode",
            "authority",
            "delivery_plan",
            "selection_authority",
            "rules",
            "allowed_dispositions",
            "capability_disposition_contract",
            "workstreams",
        }
        self.assertEqual(expected_root_keys, set(scope), "portal completion scope root schema drift")
        forbidden_root_live_fields = {
            "status",
            "current_status",
            "selector_state",
            "ready",
            "owned",
            "branch",
            "pull_request",
            "head",
        }
        self.assertFalse(
            forbidden_root_live_fields.intersection(scope),
            "portal completion scope root must not persist mutable live selector/ownership state",
        )

        self.assertEqual(1, scope["schema_version"])
        self.assertEqual("OTERYN_PORTAL_COMPLETION", scope["programme"])
        self.assertEqual("non_scheduling_completion_scope", scope["mode"])
        self.assertEqual(
            "docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md",
            scope["authority"],
        )
        self.assertEqual(
            "docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md",
            scope["delivery_plan"],
        )
        self.assertEqual(
            "docs/agents/programs/OTERYN_PORTAL_COMPLETION.md",
            scope["selection_authority"],
        )

        expected_rules = {
            "selects_work": False,
            "claims_ownership": False,
            "proves_live_state": False,
            "can_promote_ready": False,
            "live_selector_state_required": True,
            "higher_authority_overrides_projection": True,
            "required_meaning": (
                "A terminal implement, defer or reject disposition is required before global portal "
                "completion; REQUIRED does not mean live READY."
            ),
            "conditional_meaning": (
                "The workstream participates only when its named accepted activation trigger is proven; "
                "CONDITIONAL does not self-activate."
            ),
            "deferred_meaning": (
                "Outside current launch scope until an accepted reactivation trigger is satisfied."
            ),
            "rejected_meaning": "Explicitly excluded by accepted authority.",
        }
        self.assertEqual(expected_rules, scope["rules"], "portal completion scope rule semantics drift")

        allowed_list = ["REQUIRED", "CONDITIONAL", "DEFERRED", "REJECTED"]
        self.assertEqual(allowed_list, scope["allowed_dispositions"])
        allowed = set(allowed_list)

        expected_capability_disposition_contract = {
            "required_for_global_completion": True,
            "inventory_authority": [
                "docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md",
                "focused canonical architecture owners for included capability families",
                "accepted owner decisions for the named release scope",
            ],
            "inventory_resolution": (
                "Enumerate every named benchmark capability for the exact release scope with a stable "
                "capability_id; workstream or family grouping must not replace member capabilities."
            ),
            "allowed_outcomes": ["IMPLEMENT", "DEFER", "REJECT"],
            "required_record_fields": [
                "capability_id",
                "owner",
                "rationale",
                "outcome",
                "authority_evidence",
            ],
            "one_record_per_capability": True,
            "owner_approved": True,
            "stable_capability_id_required": True,
            "workstream_disposition_is_not_capability_proof": True,
            "scope_manifest_is_not_live_proof": True,
            "missing_or_ambiguous_record_state": "DECISION_REQUIRED",
        }
        self.assertEqual(
            expected_capability_disposition_contract,
            scope["capability_disposition_contract"],
            "global per-capability disposition proof contract drift",
        )

        programme_text = (
            root / "docs/agents/programs/OTERYN_PORTAL_COMPLETION.md"
        ).read_text(encoding="utf-8")
        for marker in (
            "## Per-capability disposition proof",
            "a broad workstream/family row must not replace or collapse its member capabilities;",
            "every record must contain `capability_id`, `owner`, `rationale`, `outcome` and `authority_evidence`;",
            "missing, duplicate, conflicting or ambiguous capability-disposition evidence is `DECISION_REQUIRED` and keeps global Portal Completion false.",
            "no broad workstream/family disposition is being used as a substitute for the required per-capability records;",
        ):
            self.assertIn(marker, programme_text)

        completion_markers = (
            "the exact canonical per-capability inventory",
            "every capability has exactly one owner-approved `IMPLEMENT | DEFER | REJECT` record containing stable `capability_id`, `owner`, `rationale`, `outcome` and `authority_evidence`;",
            "no broad workstream/family disposition substitutes for the required per-capability records;",
            "`DECISION_REQUIRED` and global completion is false",
        )
        for relative in (
            "docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md",
            "docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md",
        ):
            text = (root / relative).read_text(encoding="utf-8")
            for marker in completion_markers:
                self.assertIn(marker, text, f"{relative} missing completion marker: {marker}")

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
            self.assertIsInstance(item["activation_trigger"], str)
            self.assertTrue(item["activation_trigger"].strip())
            self.assertIsInstance(item["boundary"], str)
            self.assertTrue(item["boundary"].strip())
            selector_entry = item["selector_entry"]
            self.assertTrue(
                selector_entry is None or (isinstance(selector_entry, int) and 1 <= selector_entry <= 12),
                f"invalid selector_entry for {item['id']}",
            )
            self.assertFalse(
                forbidden_live_fields.intersection(item),
                f"{item['id']} must not persist mutable live selector/ownership state",
            )

        expected = {
            "portal_control_plane": {
                "disposition": "REQUIRED",
                "selector_entry": 2,
                "activation_trigger": "current routing drift materially prevents reliable Portal Completion selection or closeout",
                "boundary": "current portal routing only; historical Git-ref lifecycle belongs to repository governance",
            },
            "launch_critical_remediation": {
                "disposition": "CONDITIONAL",
                "selector_entry": 3,
                "activation_trigger": "a currently open implementation-authorized launch-critical or high-risk Platform repair Issue exists",
                "boundary": "route through OTERYN_PLATFORM_REMEDIATION; historical closed findings never self-reactivate",
            },
            "production_public_edge_proof": {
                "disposition": "REQUIRED",
                "selector_entry": 4,
                "activation_trigger": "global portal go-live completion is being claimed",
                "boundary": "repository readiness is not production proof; direct protected-environment evidence and authority remain separate",
            },
            "core_account_character_portfolio": {
                "disposition": "REQUIRED",
                "selector_entry": 5,
                "activation_trigger": "current launch scope",
                "boundary": "runtime actions remain gated by accepted Character Authority contracts and external-repository authority",
            },
            "liveops": {
                "disposition": "REQUIRED",
                "selector_entry": 6,
                "activation_trigger": "current launch scope",
                "boundary": "runtime current-state facts require an authoritative producer; unavailable evidence is never fabricated",
            },
            "public_today": {
                "disposition": "REQUIRED",
                "selector_entry": 6,
                "activation_trigger": "current launch scope",
                "boundary": "source owners retain fact, freshness, publication and privacy authority",
            },
            "private_today": {
                "disposition": "CONDITIONAL",
                "selector_entry": 6,
                "activation_trigger": "accepted launch scope includes authenticated personalization and the identity/privacy/cache gates are satisfied",
                "boundary": "private-influenced representations remain owner-private and non-shareable",
            },
            "federated_search": {
                "disposition": "REQUIRED",
                "selector_entry": 7,
                "activation_trigger": "current portal completion scope",
                "boundary": "Announcements/Events reverse-edge cleanup precedes provider onboarding",
            },
            "client_distribution_platform": {
                "disposition": "CONDITIONAL",
                "selector_entry": 8,
                "activation_trigger": "the launcher/client-distribution path is part of the accepted launch scope or an accepted live implementation handoff exists",
                "boundary": "Platform implementation does not authorize external updater, signing infrastructure or production activation",
            },
            "wiki_expected_inventory": {
                "disposition": "REQUIRED",
                "selector_entry": 9,
                "activation_trigger": "current launch scope",
                "boundary": "content-completeness claims must remain machine-checkable",
            },
            "game_catalog_expected_inventory": {
                "disposition": "REQUIRED",
                "selector_entry": 9,
                "activation_trigger": "current launch scope",
                "boundary": "Platform-only work follows current repository authority; server/game evidence requires separate owner authorization",
            },
            "multi_world_ruleset_season_dimensions": {
                "disposition": "CONDITIONAL",
                "selector_entry": None,
                "activation_trigger": "a selected slice would otherwise introduce an unresolved irreversible world/profile/ruleset/catalog/season assumption",
                "boundary": "cross-cutting invariant, not an independent standing queue item",
            },
            "player_companion_foundation": {
                "disposition": "REQUIRED",
                "selector_entry": 10,
                "activation_trigger": "current launch scope",
                "boundary": "the Hunt Session Analyzer v1 satisfies the accepted first complete workflow foundation; current terminal state is resolved live, not stored here",
            },
            "player_companion_capability_inventory_disposition": {
                "disposition": "REQUIRED",
                "selector_entry": 10,
                "activation_trigger": "global portal completion is being claimed and one or more canonical PlayerCompanion capabilities lacks an owner-approved IMPLEMENT/DEFER/REJECT disposition",
                "boundary": "decision gate only; implementation is not implied; every listed capability needs an owner-approved disposition and rationale before this gate is terminal",
            },
            "player_companion_followups": {
                "disposition": "CONDITIONAL",
                "selector_entry": 10,
                "activation_trigger": "an individual follow-up capability has an owner-approved IMPLEMENT disposition and its authoritative dependencies are proven",
                "boundary": "only capabilities explicitly dispositioned IMPLEMENT become runtime candidates; DEFER/REJECT decisions remain terminal and do not self-reactivate",
            },
            "platform_api": {
                "disposition": "DEFERRED",
                "selector_entry": None,
                "activation_trigger": "an approved named consumer/use case satisfies ADR 0036 activation criteria",
                "boundary": "specialized game-auth/internal endpoints are not general PlatformAPI activation",
            },
            "public_game_data_richer_community_reads": {
                "disposition": "DEFERRED",
                "selector_entry": 11,
                "activation_trigger": "accepted product scope explicitly promotes a read-only community surface with authoritative inputs",
                "boundary": "product inventory inputs are not automatic launch requirements",
            },
            "forum": {
                "disposition": "DEFERRED",
                "selector_entry": 11,
                "activation_trigger": "a durable owned discussion/moderation need is accepted",
                "boundary": "Discord plus existing Support remains the default direction",
            },
            "read_scaling_dedicated_index": {
                "disposition": "DEFERRED",
                "selector_entry": 11,
                "activation_trigger": "measured telemetry crosses an accepted scaling/search threshold",
                "boundary": "derived infrastructure never becomes source truth",
            },
            "world_hub": {
                "disposition": "DEFERRED",
                "selector_entry": 11,
                "activation_trigger": "multiple worlds/profiles or authoritative status/history signals justify the composition",
                "boundary": "World Hub is presentation/composition, never routing or admission authority",
            },
            "commerce_capability": {
                "disposition": "REQUIRED",
                "selector_entry": 12,
                "activation_trigger": "global portal completion requires an explicit implement, defer or reject disposition for current commerce capability findings",
                "boundary": "REQUIRED here means terminal product disposition, not payment implementation or customer-payment activation",
            },
            "commerce_production_activation": {
                "disposition": "DEFERRED",
                "selector_entry": 12,
                "activation_trigger": "independent product, security, legal, provider, operational and protected-environment gates are accepted and satisfied",
                "boundary": "production payment/value authority is separately protected",
            },
        }
        by_id = {item["id"]: item for item in workstreams}
        self.assertEqual(set(expected), set(by_id), "portal completion canonical workstream inventory drift")

        common_keys = {"id", "selector_entry", "disposition", "activation_trigger", "boundary"}
        for workstream_id, contract in expected.items():
            item = by_id[workstream_id]
            expected_keys = common_keys | (
                {"required_capability_dispositions"}
                if workstream_id == "player_companion_capability_inventory_disposition"
                else set()
            )
            self.assertEqual(expected_keys, set(item), f"unexpected scope fields for {workstream_id}")
            for field, value in contract.items():
                self.assertEqual(value, item[field], f"{workstream_id}.{field} drift")

        expected_player_companion_capabilities = [
            "loot_split",
            "hunt_finder",
            "equipment_explorer_comparison",
            "character_build_planner",
            "charm_perk_proficiency_planner",
            "quest_access_tracker",
            "exp_training_calculators",
            "validated_shareable_builds",
            "bestiary_bosstiary_planner",
            "forge_upgrade_calculator",
            "imbuement_sustain_calculators",
            "team_hunt_composer",
            "weekly_task_personal_goal_planner",
            "owner_private_tracking_routines_change_signals",
            "equipment_set_comparison",
            "damage_sustain_resistance_simulation",
            "explainable_next_action_recommendations",
            "interactive_maps_route_planning",
            "raid_boss_scheduling_integrations",
            "market_price_trends_economy_analytics",
            "public_build_profiles_comparisons",
            "advanced_full_build_simulation",
            "community_contribution_workflows",
            "public_social_tracking_graphs_comparisons",
        ]
        self.assertEqual(
            expected_player_companion_capabilities,
            by_id["player_companion_capability_inventory_disposition"]["required_capability_dispositions"],
            "PlayerCompanion capability disposition inventory drift",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
