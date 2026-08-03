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


class PortalExhaustiveStrictnessTests(unittest.TestCase):
    def test_locales_require_explicit_en_and_pl(self) -> None:
        self.assertEqual({"en", "pl"}, STRICT.declared_locales(["localized-en", "localized-pl"]))
        self.assertEqual({"pl"}, STRICT.declared_locales(["Polish responsive state"]))
        self.assertEqual(set(), STRICT.declared_locales(["localization-covered"]))

    def test_failure_without_recovery_emits_state_finding(self) -> None:
        records = [
            {
                "kind": "rendered_screen",
                "surface_id": "sample.surface",
                "module": "cms_content",
                "states": ["dependency-unavailable", "localized-en", "localized-pl"],
            }
        ]
        findings, identifiers = STRICT.strict_surface_findings(records)
        state_id = STRICT.AUDIT.finding_id("STATE", "sample.surface")
        self.assertIn(state_id, identifiers["sample.surface"])
        state_finding = next(row for row in findings if row["id"] == state_id)
        self.assertEqual(["recovery"], state_finding["evidence"]["missing_categories"])

    def test_failure_and_recovery_do_not_emit_state_finding(self) -> None:
        records = [
            {
                "kind": "rendered_screen",
                "surface_id": "sample.surface",
                "module": "cms_content",
                "states": ["dependency-unavailable", "dependency-restored", "localized-en", "localized-pl"],
            }
        ]
        findings, _ = STRICT.strict_surface_findings(records)
        state_id = STRICT.AUDIT.finding_id("STATE", "sample.surface")
        self.assertNotIn(state_id, {row["id"] for row in findings})

    def test_missing_polish_emits_locale_finding(self) -> None:
        records = [
            {
                "kind": "rendered_screen",
                "surface_id": "sample.surface",
                "module": "cms_content",
                "states": ["dependency-unavailable", "dependency-restored", "localized-en"],
            }
        ]
        findings, identifiers = STRICT.strict_surface_findings(records)
        locale_id = STRICT.AUDIT.finding_id("LOCALE", "sample.surface")
        self.assertIn(locale_id, identifiers["sample.surface"])
        locale_finding = next(row for row in findings if row["id"] == locale_id)
        self.assertEqual(["pl"], locale_finding["evidence"]["missing_locales"])

    def test_generic_localization_marker_is_fail_closed(self) -> None:
        records = [
            {
                "kind": "rendered_screen",
                "surface_id": "sample.surface",
                "module": "cms_content",
                "states": ["dependency-unavailable", "dependency-restored", "localization-covered"],
            }
        ]
        findings, _ = STRICT.strict_surface_findings(records)
        locale_finding = next(row for row in findings if row["id"] == STRICT.AUDIT.finding_id("LOCALE", "sample.surface"))
        self.assertEqual(["en", "pl"], locale_finding["evidence"]["missing_locales"])


if __name__ == "__main__":
    unittest.main()
