#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("portal_exhaustive_strictness.py")
SPEC = importlib.util.spec_from_file_location("portal_exhaustive_strictness", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
STRICT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(STRICT)


def rendered(states: list[str], markers: list[str] | None = None) -> dict:
    return {
        "kind": "rendered_screen",
        "surface_id": "sample.surface",
        "module": "cms_content",
        "states": states,
        "evidence_layers": [],
        "evidence": [{"file": "sample.spec.mjs", "markers": markers or []}],
    }


class PortalExhaustiveStrictnessTests(unittest.TestCase):
    def test_locales_require_explicit_en_and_pl(self) -> None:
        self.assertEqual({"en", "pl"}, STRICT.declared_locales(["localized-en", "localized-pl"]))
        self.assertEqual({"pl"}, STRICT.declared_locales(["Polish responsive state"]))
        self.assertEqual(set(), STRICT.declared_locales(["localization-covered"]))

    def test_validation_error_is_not_server_failure(self) -> None:
        categories = STRICT.strict_state_categories(["validation-error"])
        self.assertNotIn("server_failure", categories)

    def test_plain_missing_is_not_http_not_found(self) -> None:
        categories = STRICT.strict_state_categories(["missing"])
        self.assertNotIn("not_found", categories)

    def test_precise_http_markers_classify_every_required_category(self) -> None:
        categories = STRICT.strict_state_categories(
            [
                "not-found",
                "csrf-419",
                "rate-limit",
                "dependency-unavailable",
                "dependency-restored",
            ]
        )
        self.assertEqual(STRICT.REQUIRED_STATE_CATEGORIES, categories)

    def test_failure_without_recovery_emits_state_finding(self) -> None:
        records = [rendered(["dependency-unavailable", "localized-en", "localized-pl"])]
        findings, identifiers = STRICT.strict_surface_findings(records)
        state_id = STRICT.AUDIT.finding_id("STATE", "sample.surface")
        self.assertIn(state_id, identifiers["sample.surface"])
        state_finding = next(row for row in findings if row["id"] == state_id)
        self.assertIn("recovery", state_finding["evidence"]["missing_categories"])
        self.assertIn("csrf_expiry", state_finding["evidence"]["missing_categories"])

    def test_complete_http_matrix_does_not_emit_state_finding(self) -> None:
        records = [
            rendered(
                [
                    "not-found",
                    "csrf-419",
                    "rate-limit",
                    "dependency-unavailable",
                    "dependency-restored",
                    "localized-en",
                    "localized-pl",
                ],
                ["accessibility axe check", "no horizontal overflow"],
            )
        ]
        findings, _ = STRICT.strict_surface_findings(records)
        state_id = STRICT.AUDIT.finding_id("STATE", "sample.surface")
        self.assertNotIn(state_id, {row["id"] for row in findings})

    def test_missing_polish_emits_locale_finding(self) -> None:
        records = [rendered(["dependency-unavailable", "dependency-restored", "localized-en"])]
        findings, identifiers = STRICT.strict_surface_findings(records)
        locale_id = STRICT.AUDIT.finding_id("LOCALE", "sample.surface")
        self.assertIn(locale_id, identifiers["sample.surface"])
        locale_finding = next(row for row in findings if row["id"] == locale_id)
        self.assertEqual(["pl"], locale_finding["evidence"]["missing_locales"])

    def test_generic_localization_marker_is_fail_closed(self) -> None:
        records = [rendered(["dependency-unavailable", "dependency-restored", "localization-covered"])]
        findings, _ = STRICT.strict_surface_findings(records)
        locale_finding = next(row for row in findings if row["id"] == STRICT.AUDIT.finding_id("LOCALE", "sample.surface"))
        self.assertEqual(["en", "pl"], locale_finding["evidence"]["missing_locales"])

    def test_accessibility_and_overflow_need_explicit_markers(self) -> None:
        findings, _ = STRICT.strict_surface_findings([rendered(["localized-en", "localized-pl"])])
        identifiers = {row["id"] for row in findings}
        self.assertIn(STRICT.AUDIT.finding_id("ACCESSIBILITY", "sample.surface"), identifiers)
        self.assertIn(STRICT.AUDIT.finding_id("OVERFLOW", "sample.surface"), identifiers)

    def test_accessibility_and_overflow_markers_close_findings(self) -> None:
        records = [rendered(["localized-en", "localized-pl"], ["WCAG accessibility", "no horizontal overflow"])]
        findings, _ = STRICT.strict_surface_findings(records)
        identifiers = {row["id"] for row in findings}
        self.assertNotIn(STRICT.AUDIT.finding_id("ACCESSIBILITY", "sample.surface"), identifiers)
        self.assertNotIn(STRICT.AUDIT.finding_id("OVERFLOW", "sample.surface"), identifiers)


if __name__ == "__main__":
    unittest.main()
