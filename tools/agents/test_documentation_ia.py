#!/usr/bin/env python3
from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import documentation_ia as ia


def base_catalog():
    return {
        "schema_version": 1,
        "catalog_id": "fixture",
        "inventory_head": "a" * 40,
        "inventory_scope": {
            "prompt_root": "docs/agents/prompts",
            "handover_root": "docs/agents/handovers",
            "instruction_chain": {
                "effective_paths": ["AGENTS.md", "docs/agents/AGENTS.md"],
                "measured_absent_overrides": ["docs/agents/prompts/AGENTS.md"],
            },
        },
        "lifecycle_authority": "docs/agents/DOCUMENTATION_IA_LIFECYCLE.md",
        "prompts": [
            {
                "id": "REUSABLE",
                "path": "docs/agents/prompts/reusable.md",
                "version": "1.0",
                "classification": "reusable",
                "status": "active_reusable",
                "owner": "fixture",
                "scope": "fixture",
                "executable": True,
                "supersession": {"target": None, "reason": "current"},
                "provenance": ["fixture"],
            },
            {
                "id": "HISTORICAL",
                "path": "docs/agents/prompts/historical.md",
                "version": "legacy",
                "classification": "one_shot_historical",
                "status": "historical_do_not_run",
                "owner": "fixture",
                "scope": "fixture",
                "executable": False,
                "supersession": {"target": "REUSABLE", "reason": "superseded"},
                "provenance": ["fixture"],
            },
        ],
        "handovers": [
            {
                "id": "HANDOVER",
                "path": "docs/agents/handovers/handover.md",
                "status": "historical_snapshot",
                "authoritative": False,
                "lifecycle": "expired",
                "superseded_by": "live state",
                "provenance": ["fixture"],
            }
        ],
    }


class DocumentationIATests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        for path in ("docs/agents/prompts", "docs/agents/handovers"):
            (self.root / path).mkdir(parents=True)
        (self.root / "AGENTS.md").write_text("root", encoding="utf-8")
        (self.root / "docs/agents/AGENTS.md").write_text("nested", encoding="utf-8")
        (self.root / "docs/agents/DOCUMENTATION_IA_LIFECYCLE.md").write_text("authority", encoding="utf-8")
        (self.root / "docs/agents/prompts/reusable.md").write_text("x", encoding="utf-8")
        (self.root / "docs/agents/prompts/historical.md").write_text("x", encoding="utf-8")
        (self.root / "docs/agents/handovers/handover.md").write_text("x", encoding="utf-8")
        self.catalog = self.root / "docs/agents/DOCUMENTATION_IA_CATALOG.json"

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, value):
        self.catalog.write_text(json.dumps(value), encoding="utf-8")

    def test_valid_catalog(self):
        self.write(base_catalog())
        self.assertEqual(ia.validate_catalog(self.root, self.catalog), [])

    def test_uncatalogued_prompt_fails(self):
        value = base_catalog()
        value["prompts"] = value["prompts"][:1]
        self.write(value)
        errors = ia.validate_catalog(self.root, self.catalog)
        self.assertTrue(any("prompt inventory mismatch" in item for item in errors))

    def test_historical_prompt_cannot_be_executable(self):
        value = base_catalog()
        value["prompts"][1]["executable"] = True
        self.write(value)
        errors = ia.validate_catalog(self.root, self.catalog)
        self.assertTrue(any("executable contradicts classification" in item for item in errors))

    def test_handover_cannot_be_authoritative(self):
        value = base_catalog()
        value["handovers"][0]["authoritative"] = True
        self.write(value)
        errors = ia.validate_catalog(self.root, self.catalog)
        self.assertTrue(any("authoritative=false" in item for item in errors))

    def test_new_nearer_instruction_override_requires_remeasurement(self):
        value = base_catalog()
        self.write(value)
        (self.root / "docs/agents/prompts/AGENTS.md").write_text("new", encoding="utf-8")
        errors = ia.validate_catalog(self.root, self.catalog)
        self.assertTrue(any("absent override now exists" in item for item in errors))


if __name__ == "__main__":
    unittest.main()
