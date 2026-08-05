#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("portal_exhaustive_reconcile.py")
SPEC = importlib.util.spec_from_file_location("portal_exhaustive_reconcile", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
RECONCILE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RECONCILE)


class PortalExhaustiveReconcileTests(unittest.TestCase):
    def test_selectors_match_exact_or_prefix(self) -> None:
        self.assertTrue(RECONCILE.selector_matches("oauth.authorize", {"prefix": "oauth."}))
        self.assertTrue(RECONCILE.selector_matches("health", {"exact": "health"}))
        self.assertFalse(RECONCILE.selector_matches("oauth.authorize", {"exact": "oauth.token"}))

    def test_aggregate_is_fail_closed(self) -> None:
        records = [
            {"verdicts": {"functional": "PASS"}},
            {"verdicts": {"functional": "PARTIAL"}},
            {"verdicts": {"functional": "UNKNOWN"}},
        ]
        self.assertEqual("UNKNOWN", RECONCILE.aggregate_verdict(records, "functional"))

    def test_not_applicable_is_ignored_when_applicable_values_exist(self) -> None:
        records = [
            {"verdicts": {"content_complete": "NOT_APPLICABLE"}},
            {"verdicts": {"content_complete": "PASS"}},
        ]
        self.assertEqual("PASS", RECONCILE.aggregate_verdict(records, "content_complete"))

    def test_all_not_applicable_stays_not_applicable(self) -> None:
        records = [
            {"verdicts": {"content_complete": "NOT_APPLICABLE"}},
            {"verdicts": {"content_complete": "NOT_APPLICABLE"}},
        ]
        self.assertEqual("NOT_APPLICABLE", RECONCILE.aggregate_verdict(records, "content_complete"))


if __name__ == "__main__":
    unittest.main()
