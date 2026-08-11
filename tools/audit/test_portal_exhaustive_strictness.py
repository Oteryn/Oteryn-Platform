#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import tempfile
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
        "route_name": "sample.index",
        "methods": ["GET", "HEAD"],
        "uri": "sample",
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
        findings, identifiers, _ = STRICT.strict_surface_findings(records)
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
        findings, _, _ = STRICT.strict_surface_findings(records)
        state_id = STRICT.AUDIT.finding_id("STATE", "sample.surface")
        self.assertNotIn(state_id, {row["id"] for row in findings})

    def test_missing_polish_emits_locale_finding(self) -> None:
        records = [rendered(["dependency-unavailable", "dependency-restored", "localized-en"])]
        findings, identifiers, _ = STRICT.strict_surface_findings(records)
        locale_id = STRICT.AUDIT.finding_id("LOCALE", "sample.surface")
        self.assertIn(locale_id, identifiers["sample.surface"])
        locale_finding = next(row for row in findings if row["id"] == locale_id)
        self.assertEqual(["pl"], locale_finding["evidence"]["missing_locales"])

    def test_generic_localization_marker_is_fail_closed(self) -> None:
        records = [rendered(["dependency-unavailable", "dependency-restored", "localization-covered"])]
        findings, _, _ = STRICT.strict_surface_findings(records)
        locale_finding = next(row for row in findings if row["id"] == STRICT.AUDIT.finding_id("LOCALE", "sample.surface"))
        self.assertEqual(["en", "pl"], locale_finding["evidence"]["missing_locales"])

    def test_accessibility_and_overflow_need_explicit_markers(self) -> None:
        findings, _, _ = STRICT.strict_surface_findings([rendered(["localized-en", "localized-pl"])])
        identifiers = {row["id"] for row in findings}
        self.assertIn(STRICT.AUDIT.finding_id("ACCESSIBILITY", "sample.surface"), identifiers)
        self.assertIn(STRICT.AUDIT.finding_id("OVERFLOW", "sample.surface"), identifiers)

    def test_accessibility_and_overflow_markers_close_findings(self) -> None:
        records = [rendered(["localized-en", "localized-pl"], ["WCAG accessibility", "no horizontal overflow"])]
        findings, _, _ = STRICT.strict_surface_findings(records)
        identifiers = {row["id"] for row in findings}
        self.assertNotIn(STRICT.AUDIT.finding_id("ACCESSIBILITY", "sample.surface"), identifiers)
        self.assertNotIn(STRICT.AUDIT.finding_id("OVERFLOW", "sample.surface"), identifiers)

    def test_covered_contract_requires_exact_source_markers(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            evidence = root / "sample.spec.mjs"
            evidence.write_text("marker one\nmarker two\n", encoding="utf-8")
            contract = {
                "schema_version": 1,
                "surfaces": {
                    "sample.surface": {
                        "state_categories": {
                            "server_failure": {
                                "status": "covered",
                                "evidence": {"file": "sample.spec.mjs", "markers": ["missing marker"]},
                            }
                        }
                    }
                },
            }
            with self.assertRaisesRegex(RuntimeError, "evidence marker is missing"):
                STRICT.strict_surface_findings([rendered(["localized-en", "localized-pl"])], contract, root)

    def test_read_only_surface_can_prove_csrf_non_applicability(self) -> None:
        contract = {
            "schema_version": 1,
            "surfaces": {
                "sample.surface": {
                    "state_categories": {
                        "csrf_expiry": {
                            "status": "not_applicable",
                            "rule": "read_only_surface",
                            "reason": "This sample surface contains only GET and HEAD routes, so a CSRF-expiry response cannot apply to its read-only browser requests.",
                        }
                    }
                }
            },
        }
        findings, _, summaries = STRICT.strict_surface_findings(
            [rendered(["not-found", "rate-limit", "dependency-unavailable", "dependency-restored", "localized-en", "localized-pl"], ["accessibility", "overflow"])],
            contract,
        )
        state_id = STRICT.AUDIT.finding_id("STATE", "sample.surface")
        self.assertNotIn(state_id, {row["id"] for row in findings})
        self.assertEqual(["csrf_expiry"], summaries["sample.surface"]["not_applicable_state_categories"])

    def test_read_only_non_applicability_fails_when_surface_has_post(self) -> None:
        post = rendered(["not-found", "rate-limit", "dependency-unavailable", "dependency-restored", "localized-en", "localized-pl"], ["accessibility", "overflow"])
        post["route_name"] = "sample.store"
        post["kind"] = "form_action"
        post["methods"] = ["POST"]
        contract = {
            "schema_version": 1,
            "surfaces": {
                "sample.surface": {
                    "state_categories": {
                        "csrf_expiry": {
                            "status": "not_applicable",
                            "rule": "read_only_surface",
                            "reason": "This deliberately invalid test explanation is long enough to reach the validation boundary but the route topology disproves it.",
                        }
                    }
                }
            },
        }
        with self.assertRaisesRegex(RuntimeError, "read_only_surface is false"):
            STRICT.strict_surface_findings([rendered([]), post], contract)


if __name__ == "__main__":
    unittest.main()
