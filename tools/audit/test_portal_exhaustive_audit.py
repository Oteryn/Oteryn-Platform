#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("portal_exhaustive_audit.py")
SPEC = importlib.util.spec_from_file_location("portal_exhaustive_audit", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


class PortalExhaustiveAuditTests(unittest.TestCase):
    def test_complete_allows_not_applicable(self) -> None:
        verdict = AUDIT.final_classification(
            {
                "exists": "PASS",
                "functional": "PASS",
                "content_complete": "NOT_APPLICABLE",
                "production_complete": "PASS",
            }
        )
        self.assertEqual("COMPLETE", verdict)

    def test_missing_and_unknown_are_fail_closed(self) -> None:
        self.assertEqual(
            "MISSING",
            AUDIT.final_classification(
                {
                    "exists": "MISSING",
                    "functional": "UNKNOWN",
                    "content_complete": "UNKNOWN",
                    "production_complete": "UNKNOWN",
                }
            ),
        )
        self.assertEqual(
            "BLOCKED",
            AUDIT.final_classification(
                {
                    "exists": "PASS",
                    "functional": "UNKNOWN",
                    "content_complete": "PARTIAL",
                    "production_complete": "PARTIAL",
                }
            ),
        )

    def test_surface_module_mapping_preserves_specific_prefixes(self) -> None:
        self.assertEqual("characters", AUDIT.module_for_surface("identity.character-profile-preferences"))
        self.assertEqual("identity", AUDIT.module_for_surface("identity.password-lifecycle"))
        self.assertEqual("game_catalog", AUDIT.module_for_surface("game-catalog.public-items-creatures-and-loot"))
        self.assertEqual("quality_e2e", AUDIT.module_for_surface("unknown.surface"))

    def test_capability_module_mapping_separates_products_and_payments(self) -> None:
        self.assertEqual("products_entitlements", AUDIT.module_for_capability("commerce.products-ready-to-use"))
        self.assertEqual("payments", AUDIT.module_for_capability("commerce.provider-webhook-refunds-chargebacks"))
        self.assertEqual("characters", AUDIT.module_for_capability("character.rename"))

    def test_state_categories_are_explicit(self) -> None:
        categories = AUDIT.state_categories(
            ["validation-error", "permission-denied", "dependency-unavailable", "dependency-restored"]
        )
        self.assertIn("validation", categories)
        self.assertIn("authorization", categories)
        self.assertIn("server_failure", categories)
        self.assertIn("recovery", categories)

    def test_finding_ids_are_stable_and_subject_specific(self) -> None:
        first = AUDIT.finding_id("CONTENT", "wiki")
        second = AUDIT.finding_id("CONTENT", "wiki")
        other = AUDIT.finding_id("CONTENT", "game_catalog")
        self.assertEqual(first, second)
        self.assertNotEqual(first, other)
        self.assertTrue(first.startswith("OTERYN-AUDIT-CURRENT-CONTENT-"))

    def test_canonical_workflow_gates_wiki_inventory_before_audit_generation(self) -> None:
        repo_root = MODULE_PATH.parents[2]
        workflow = (repo_root / ".github/workflows/portal-exhaustive-audit.yml").read_text(encoding="utf-8")

        gate = workflow.index("- name: Validate Wiki expected-content inventory")
        command = workflow.index("php artisan wiki:launch-content:validate --json")
        generation = workflow.index("- name: Generate exhaustive current-main audit")

        self.assertLess(gate, command)
        self.assertLess(command, generation)
        self.assertIn("assert report['status'] == 'PASS'", workflow)
        self.assertIn("wiki-expected-content-validation.json", workflow)


if __name__ == "__main__":
    unittest.main()
