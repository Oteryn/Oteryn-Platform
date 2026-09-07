#!/usr/bin/env python3
from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

import policy_consistency as policy


class FakeCentral(SimpleNamespace):
    def __init__(self) -> None:
        super().__init__()
        self.bundle_errors: list[str] = []
        self.binding_errors: list[str] = []
        self.overlay_errors: list[str] = []
        self.seen_provider: str | None = None
        self.seen_overlay: str | None = None

    @staticmethod
    def load_policy(_root: Path) -> dict[str, object]:
        return {"policy_id": "OTERYN_ORGANIZATION_AGENT_POLICY", "policy_version": "3.0.0"}

    def validate_meta_bundle(self, _root: Path, _policy: dict[str, object]) -> list[str]:
        return list(self.bundle_errors)

    def validate_provider_binding(self, _binding, *, policy, authority_resolver) -> list[str]:
        self.resolved = authority_resolver("Oteryn/Oteryn", _binding.get("authority_commit"))
        return list(self.binding_errors)

    def validate_provider_overlay(self, provider: str, text: str, *, policy) -> list[str]:
        self.seen_provider = provider
        self.seen_overlay = text
        return list(self.overlay_errors)

    @staticmethod
    def resolve_meta_authority_via_github(repository: str, commit: str) -> dict[str, object]:
        return {"repository": repository, "commit": commit, "merged_to_protected_main": True}


class PolicyConsistencyTest(unittest.TestCase):
    def setUp(self) -> None:
        self.source_root = Path(__file__).resolve().parents[2]
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "provider"
        self.meta_root = Path(self.temp.name) / "meta"
        self.meta_root.mkdir(parents=True)
        for relative in (
            policy.ROOT_AGENTS_PATH,
            policy.BOOTSTRAP_PATH,
            policy.BINDING_PATH,
            policy.GOVERNANCE_CONTRACT_PATH,
        ):
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((self.source_root / relative).read_bytes())
        self.central = FakeCentral()
        self.resolved = {
            "repository": "Oteryn/Oteryn",
            "commit": "5ed3f14400af450b5875c091e443da70f2d67ab9",
            "merged_to_protected_main": True,
            "branch_protected": True,
        }
        self.resolver = lambda repository, commit: {
            **self.resolved,
            "repository": repository,
            "commit": commit,
        }

    def validate(self) -> list[str]:
        return policy.validate_policy(
            self.root,
            meta_root=self.meta_root,
            meta_module=self.central,
            authority_resolver=self.resolver,
            meta_head_resolver=lambda _root: "5ed3f14400af450b5875c091e443da70f2d67ab9",
            meta_clean_resolver=lambda _root: True,
        )

    def replace(self, relative: Path, old: str, new: str) -> None:
        path = self.root / relative
        text = path.read_text(encoding="utf-8")
        self.assertIn(old, text)
        path.write_text(text.replace(old, new, 1), encoding="utf-8")

    def test_current_provider_contract_passes_and_uses_meta_consumer(self) -> None:
        self.assertEqual([], self.validate())
        self.assertEqual(policy.PLATFORM_REPOSITORY, self.central.seen_provider)
        self.assertIn("META_AGENT_POLICY_BINDING.json", self.central.seen_overlay or "")

    def test_meta_bundle_error_is_not_hidden(self) -> None:
        self.central.bundle_errors = ["machine authority missing"]
        self.assertIn("META source: machine authority missing", self.validate())

    def test_binding_error_is_not_hidden(self) -> None:
        self.central.binding_errors = ["authority commit is not protected-main ancestry"]
        self.assertIn(
            "META binding: authority commit is not protected-main ancestry",
            self.validate(),
        )

    def test_provider_overlay_error_is_not_hidden(self) -> None:
        self.central.overlay_errors = ["parallel-first execution wording is forbidden"]
        self.assertIn(
            "Platform overlay: parallel-first execution wording is forbidden",
            self.validate(),
        )

    def test_each_platform_invariant_is_required(self) -> None:
        for name, marker in policy.PLATFORM_INVARIANTS.items():
            with self.subTest(name=name):
                original = (self.root / policy.ROOT_AGENTS_PATH).read_text(encoding="utf-8")
                self.replace(policy.ROOT_AGENTS_PATH, marker, f"removed-{name}")
                self.assertIn(f"AGENTS.md lacks Platform invariant {name}", self.validate())
                (self.root / policy.ROOT_AGENTS_PATH).write_text(original, encoding="utf-8")

    def test_bootstrap_keeps_server_read_and_candidate_authority_boundaries(self) -> None:
        marker = "The current task candidate cannot authorize itself."
        self.replace(policy.BOOTSTRAP_PATH, marker, "Candidate authority omitted.")
        findings = self.validate()
        self.assertTrue(any(marker in finding for finding in findings), findings)

    def test_machine_contract_sections_remain_required(self) -> None:
        path = self.root / policy.GOVERNANCE_CONTRACT_PATH
        contract = json.loads(path.read_text(encoding="utf-8"))
        del contract["live_task_liveness"]
        path.write_text(json.dumps(contract), encoding="utf-8")
        self.assertIn("GOVERNANCE_CONTRACT.json lacks live_task_liveness", self.validate())

    def test_meta_checkout_head_must_match_binding(self) -> None:
        findings = policy.validate_policy(
            self.root,
            meta_root=self.meta_root,
            meta_module=self.central,
            authority_resolver=self.resolver,
            meta_head_resolver=lambda _root: "0123456789abcdef0123456789abcdef01234567",
            meta_clean_resolver=lambda _root: True,
        )
        self.assertIn(
            "META trust bootstrap: checkout HEAD does not equal authority_commit",
            findings,
        )

    def test_untrusted_checkout_is_rejected_before_validator_import(self) -> None:
        sentinel = Path(self.temp.name) / "sentinel"
        target = self.meta_root / policy.META_VALIDATOR_PATH
        target.parent.mkdir(parents=True)
        target.write_text(
            f"from pathlib import Path\nPath({str(sentinel)!r}).write_text('executed')\n",
            encoding="utf-8",
        )
        findings = policy.validate_policy(
            self.root,
            meta_root=self.meta_root,
            authority_resolver=self.resolver,
            meta_head_resolver=lambda _root: "0123456789abcdef0123456789abcdef01234567",
            meta_clean_resolver=lambda _root: True,
        )
        self.assertIn("META trust bootstrap: checkout HEAD does not equal authority_commit", findings)
        self.assertFalse(sentinel.exists(), findings)

    def test_modified_checkout_is_rejected_before_validator_import(self) -> None:
        sentinel = Path(self.temp.name) / "dirty-sentinel"
        target = self.meta_root / policy.META_VALIDATOR_PATH
        target.parent.mkdir(parents=True)
        target.write_text(
            f"from pathlib import Path\nPath({str(sentinel)!r}).write_text('executed')\n",
            encoding="utf-8",
        )
        findings = policy.validate_policy(
            self.root,
            meta_root=self.meta_root,
            authority_resolver=self.resolver,
            meta_head_resolver=lambda _root: "5ed3f14400af450b5875c091e443da70f2d67ab9",
            meta_clean_resolver=lambda _root: False,
        )
        self.assertIn("META trust bootstrap: checkout has modified tracked files", findings)
        self.assertFalse(sentinel.exists(), findings)

    def test_loader_rejects_missing_bound_validator(self) -> None:
        with self.assertRaisesRegex(policy.PolicyConsistencyError, "missing bound META validator"):
            policy.load_meta_validator(self.meta_root)

    def test_loader_rejects_incomplete_validator_interface(self) -> None:
        target = self.meta_root / policy.META_VALIDATOR_PATH
        target.parent.mkdir(parents=True)
        target.write_text("def load_policy(root): return {}\n", encoding="utf-8")
        with self.assertRaisesRegex(policy.PolicyConsistencyError, "lacks callable"):
            policy.load_meta_validator(self.meta_root)


if __name__ == "__main__":
    unittest.main()
