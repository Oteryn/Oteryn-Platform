#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "tools/validation/php_coverage_policy.py"
spec = importlib.util.spec_from_file_location("php_coverage_policy", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"Unable to load {MODULE_PATH}")
coverage = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = coverage
spec.loader.exec_module(coverage)


CLOVER = """<?xml version="1.0" encoding="UTF-8"?>
<coverage generated="1">
  <project timestamp="1">
    <metrics statements="100" coveredstatements="83" methods="20" coveredmethods="15"/>
  </project>
</coverage>
"""


class CoveragePolicyTest(unittest.TestCase):
    def test_parse_clover_reports_statement_and_method_percent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "clover.xml"
            path.write_text(CLOVER, encoding="utf-8")
            metrics = coverage.parse_clover(path)
        self.assertEqual(83.0, metrics["statement_percent"])
        self.assertEqual(75.0, metrics["method_percent"])

    def test_report_only_never_invents_threshold(self) -> None:
        result = coverage.evaluate(
            {
                "mode": "report_only",
                "scope": "app/**",
                "minimum_statement_percent": None,
            },
            {
                "statement_percent": 12.34,
                "method_percent": 10.0,
                "statements": 100,
                "covered_statements": 12,
                "methods": 10,
                "covered_methods": 1,
            },
        )
        self.assertTrue(result["passed"])
        self.assertIsNone(result["minimum_statement_percent"])

    def test_enforce_fails_below_reviewed_floor(self) -> None:
        result = coverage.evaluate(
            {
                "mode": "enforce",
                "scope": "app/**",
                "minimum_statement_percent": 80.0,
            },
            {
                "statement_percent": 79.99,
                "method_percent": 70.0,
                "statements": 100,
                "covered_statements": 79,
                "methods": 10,
                "covered_methods": 7,
            },
        )
        self.assertFalse(result["passed"])

    def test_enforce_passes_at_reviewed_floor(self) -> None:
        result = coverage.evaluate(
            {
                "mode": "enforce",
                "scope": "app/**",
                "minimum_statement_percent": 80.0,
            },
            {
                "statement_percent": 80.0,
                "method_percent": 70.0,
                "statements": 100,
                "covered_statements": 80,
                "methods": 10,
                "covered_methods": 7,
            },
        )
        self.assertTrue(result["passed"])

    def test_policy_enforce_requires_threshold(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "policy.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "mode": "enforce",
                        "minimum_statement_percent": None,
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                coverage.CoveragePolicyError,
                "requires numeric minimum_statement_percent",
            ):
                coverage._load_policy(path)

    def test_clover_rejects_impossible_metrics(self) -> None:
        impossible = CLOVER.replace(
            'statements="100" coveredstatements="83"',
            'statements="100" coveredstatements="101"',
        )
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "clover.xml"
            path.write_text(impossible, encoding="utf-8")
            with self.assertRaisesRegex(
                coverage.CoveragePolicyError,
                "covered statements exceed total statements",
            ):
                coverage.parse_clover(path)


if __name__ == "__main__":
    unittest.main(verbosity=2)
