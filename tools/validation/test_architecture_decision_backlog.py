#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("architecture_decision_backlog.py")
SPEC = importlib.util.spec_from_file_location("architecture_decision_backlog", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = validator
SPEC.loader.exec_module(validator)


class ArchitectureDecisionBacklogTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        for path in (
            "docs/architecture/ARCHITECTURE_AUTHORITY.md",
            "docs/architecture/adr/README.md",
        ):
            target = self.root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("# fixture\n", encoding="utf-8")
        (self.root / "docs/agents/programs").mkdir(parents=True)
        self.data = self.fixture()
        self.write_programme(["ARCH-DEC-0001"])
        self.write()

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def fixture(self) -> dict:
        return {
            "authority": {
                "accepted_decision_authority": "docs/architecture/adr/README.md",
                "canonical_routing": "docs/architecture/ARCHITECTURE_AUTHORITY.md",
                "non_authority_statement": (
                    "This inventory grants no accepted-decision, implementation, "
                    "or activation authority."
                ),
                "role": "unresolved_decision_inventory",
            },
            "lifecycle": {
                "active_states": validator.ACTIVE_STATES,
                "terminal_handling": "Remove terminal records with their authority package.",
            },
            "records": [self.record()],
            "registry_name": validator.REGISTRY_NAME,
            "schema_version": 1,
        }

    def record(self) -> dict:
        return {
            "blocking_owner_question": "Does the owner accept option A or B?",
            "blockers": [],
            "canonical_owners": [
                "docs/architecture/ARCHITECTURE_AUTHORITY.md"
            ],
            "created_at": "2026-08-05",
            "decision_id": "ARCH-DEC-0001",
            "decision_owner": "repository owner",
            "decision_question": "Which policy should govern this concern?",
            "deferral_reason": None,
            "dependencies": {
                "decision_ids": [],
                "issues": [],
                "local_paths": [
                    "docs/architecture/ARCHITECTURE_AUTHORITY.md"
                ],
            },
            "evidence": {
                "CONFLICT": [],
                "DERIVED": ["A durable policy prevents inconsistent execution."],
                "PROVEN": ["Issue 1 records an unresolved policy choice."],
                "UNKNOWN": ["The owner-selected option is unknown."],
            },
            "implementation_authorized": False,
            "options": [
                {
                    "description": "Adopt strict enforcement.",
                    "option_id": "A",
                    "title": "Strict",
                    "trade_offs": ["More maintenance."],
                },
                {
                    "description": "Retain discretionary handling.",
                    "option_id": "B",
                    "title": "Discretionary",
                    "trade_offs": ["Weaker consistency."],
                },
            ],
            "problem_statement": "The repository lacks one durable policy.",
            "recommendation": "Adopt option A after owner acceptance.",
            "recommended_owner": "repository owner",
            "references": {
                "issue": 1,
                "proposed_adr": None,
                "related_prs": [],
            },
            "revisit_trigger": None,
            "severity": "medium",
            "state": "decision_required",
            "title": "Select repository policy",
            "updated_at": "2026-08-05",
        }

    def write(self, *, canonical: bool = True) -> None:
        path = self.root / validator.BACKLOG_PATH
        path.parent.mkdir(parents=True, exist_ok=True)
        content = (
            json.dumps(self.data, indent=2, sort_keys=True) + "\n"
            if canonical
            else json.dumps(self.data)
        )
        path.write_text(content, encoding="utf-8")

    def write_programme(self, ids: list[str], *, legacy: bool = False) -> None:
        projection = json.dumps(ids, separators=(",", ":"))
        legacy_text = "decision_backlog:\n  - id: ARCH-AUTH-001\n" if legacy else ""
        (self.root / validator.PROGRAMME_PATH).write_text(
            "# Programme\n```yaml\n"
            f"{legacy_text}active_architecture_decision_ids: {projection}\n"
            "next_action: Continue the bounded review.\n"
            "```\n",
            encoding="utf-8",
        )

    def errors(self) -> list[str]:
        return validator.validate_repository(self.root)

    def assert_error(self, text: str) -> None:
        errors = self.errors()
        self.assertTrue(
            any(text in error for error in errors),
            f"expected {text!r}, got {errors!r}",
        )

    def test_valid_registry(self) -> None:
        self.assertEqual([], self.errors())

    def test_unknown_fields_fail_closed(self) -> None:
        self.data["extra"] = True
        self.data["records"][0]["accepted"] = True
        self.write()
        errors = self.errors()
        self.assertTrue(any("root: unknown fields: extra" in e for e in errors))
        self.assertTrue(any("unknown fields: accepted" in e for e in errors))

    def test_schema_id_order_and_terminal_state(self) -> None:
        self.data["schema_version"] = 2
        self.data["records"][0]["decision_id"] = "DEC-1"
        self.data["records"][0]["state"] = "accepted"
        self.write_programme(["DEC-1"])
        self.write()
        errors = self.errors()
        self.assertTrue(any("supported value is 1" in e for e in errors))
        self.assertTrue(any("ARCH-DEC-NNNN" in e for e in errors))
        self.assertTrue(any("terminal lifecycle value" in e for e in errors))

    def test_duplicate_and_unsorted_ids(self) -> None:
        duplicate = copy.deepcopy(self.data["records"][0])
        self.data["records"].append(duplicate)
        self.write_programme(["ARCH-DEC-0001", "ARCH-DEC-0001"])
        self.write()
        self.assert_error("duplicate decision IDs")

        duplicate["decision_id"] = "ARCH-DEC-0000"
        duplicate["decision_question"] = "A separate question?"
        self.write_programme(["ARCH-DEC-0001", "ARCH-DEC-0000"])
        self.write()
        self.assert_error("records must be ordered by decision_id")

    def test_evidence_and_local_reference_integrity(self) -> None:
        record = self.data["records"][0]
        record["evidence"]["UNKNOWN"].append(
            "Issue 1 records an unresolved policy choice!"
        )
        record["canonical_owners"] = ["docs/architecture/MISSING.md"]
        self.write()
        errors = self.errors()
        self.assertTrue(any("normalized fact appears more than once" in e for e in errors))
        self.assertTrue(any("local path does not exist" in e for e in errors))

    def test_authority_and_decision_required_contract(self) -> None:
        record = self.data["records"][0]
        record["implementation_authorized"] = True
        record["options"] = []
        record["recommendation"] = None
        record["blocking_owner_question"] = None
        self.write()
        errors = self.errors()
        for expected in (
            "implementation_authorized: must be false",
            "requires at least two options",
            "requires a recommendation",
            "requires one blocking_owner_question",
        ):
            self.assertTrue(any(expected in e for e in errors), errors)

    def test_blocked_and_deferred_boundaries(self) -> None:
        record = self.data["records"][0]
        record["state"] = "blocked"
        record["blocking_owner_question"] = None
        record["options"] = []
        record["recommendation"] = None
        self.write()
        self.assert_error("blocked requires at least one blocker")

        record["state"] = "deferred"
        self.write()
        errors = self.errors()
        self.assertTrue(any("deferred requires deferral_reason" in e for e in errors))
        self.assertTrue(any("deferred requires revisit_trigger" in e for e in errors))

    def test_duplicate_obligation_and_dependency_integrity(self) -> None:
        second = copy.deepcopy(self.data["records"][0])
        second["decision_id"] = "ARCH-DEC-0002"
        second["references"]["issue"] = 2
        self.data["records"].append(second)
        self.data["records"][0]["dependencies"]["decision_ids"] = [
            "ARCH-DEC-0001",
            "ARCH-DEC-9999",
        ]
        self.write_programme(["ARCH-DEC-0001", "ARCH-DEC-0002"])
        self.write()
        errors = self.errors()
        self.assertTrue(any("duplicate unresolved obligation" in e for e in errors))
        self.assertTrue(any("cannot depend on itself" in e for e in errors))
        self.assertTrue(any("dependency decision ID does not exist" in e for e in errors))

    def test_programme_projection_and_legacy_backlog(self) -> None:
        self.write_programme([], legacy=True)
        errors = self.errors()
        self.assertTrue(any("full decision_backlog is forbidden" in e for e in errors))
        self.assertTrue(any("does not exactly match registry IDs" in e for e in errors))

    def test_noncanonical_json_and_adr_reference(self) -> None:
        self.data["records"][0]["references"]["proposed_adr"] = "ADR-1.md"
        self.write(canonical=False)
        errors = self.errors()
        self.assertTrue(any("canonical sorted-key" in e for e in errors))
        self.assertTrue(any("malformed ADR path" in e for e in errors))

        self.data["records"][0]["references"]["proposed_adr"] = (
            "docs/architecture/adr/0099-missing.md"
        )
        self.write()
        self.assert_error("local path does not exist")


if __name__ == "__main__":
    unittest.main()
