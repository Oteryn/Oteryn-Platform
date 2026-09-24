#!/usr/bin/env python3
"""Focused regressions for the Platform consumer of central META policy."""

from __future__ import annotations

import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest
from urllib.parse import urlparse

from policy_consistency import (
    BINDING_PATH,
    CATALOG_PATH,
    REPO_ROOT,
    PolicyConsistencyError,
    authenticate_meta_checkout,
    validate_policy,
)

PUBLICATION_INTEGRITY_AUTHORITY = "21bc49bccef4874b037aabcbde9732b904187c32"


class PolicyConsistencyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        configured = os.environ.get("OTERYN_META_POLICY_ROOT")
        if not configured:
            raise RuntimeError("OTERYN_META_POLICY_ROOT must name the exact bound META checkout")
        cls.meta_root = Path(configured).resolve()
        cls.binding = json.loads((REPO_ROOT / BINDING_PATH).read_text(encoding="utf-8"))
        cls.authority = authenticate_meta_checkout(
            cls.meta_root,
            cls.binding,
            github_json=cls._fake_github(cls.meta_root, cls.binding),
        )

    @staticmethod
    def _fake_github(meta_root: Path, binding: dict[str, object], *, corrupt_path: str | None = None):
        commit = str(binding["authority_commit"])
        tree = []
        output = subprocess.run(
            ["git", "ls-tree", "-r", "HEAD"],
            cwd=meta_root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        for line in output.splitlines():
            metadata, path = line.split("\t", 1)
            _mode, kind, sha = metadata.split()
            if path == corrupt_path:
                sha = "0" * 40
            tree.append({"path": path, "type": kind, "sha": sha})
        main = "f" * 40

        def read(url: str) -> object:
            path = urlparse(url).path
            if path.endswith(f"/commits/{commit}"):
                return {"sha": commit}
            if path.endswith("/branches/main"):
                return {"protected": True, "commit": {"sha": main}}
            if "/compare/" in path:
                return {
                    "status": "ahead",
                    "base_commit": {"sha": commit},
                    "merge_base_commit": {"sha": commit},
                }
            if "/git/trees/" in path:
                return {"truncated": False, "tree": tree}
            raise AssertionError(f"unexpected fake GitHub URL: {url}")

        return read

    def _fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        for relative in (Path("AGENTS.md"), BINDING_PATH, CATALOG_PATH):
            target = root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(REPO_ROOT / relative, target)
        shutil.copytree(REPO_ROOT / "docs/agents/prompts", root / "docs/agents/prompts")
        return temporary, root

    def _findings(self, root: Path) -> str:
        return "\n".join(validate_policy(root, self.meta_root, self.authority))

    @staticmethod
    def _bound_meta_checkout_target(workflow: str) -> tuple[str, str]:
        match = re.search(
            r"(?ms)^\s*- name: Check out bound META policy\s*$"
            r"(?P<body>.*?)(?=^\s*- name:|\Z)",
            workflow,
        )
        if match is None:
            raise AssertionError("bound META checkout step is missing")
        active = "\n".join(
            line for line in match.group("body").splitlines()
            if not line.lstrip().startswith("#")
        )
        repository = re.search(r"(?m)^\s*repository:\s*([^\s#]+)\s*$", active)
        ref = re.search(r"(?m)^\s*ref:\s*([0-9a-f]{40})\s*$", active)
        if repository is None or ref is None:
            raise AssertionError("bound META checkout step lacks one active repository/ref")
        return repository.group(1), ref.group(1)

    @staticmethod
    def _publication_fallback_clause(bootstrap: str) -> str:
        match = re.search(
            r"(?ms)^If an already-prepared material candidate cannot use the normal "
            r"authorized publication path, .*?(?=\n\n)",
            bootstrap,
        )
        if match is None:
            raise AssertionError("operative publication fallback clause is missing")
        return match.group(0)

    @staticmethod
    def _publication_fallback_clause_is_bound(clause: str) -> bool:
        required = (
            "do not silently reconstruct or relabel that selected candidate",
            "Preserve its custody and return publication control to the active control plane",
            "The control plane may select only an API-native **new candidate** route permitted by the bound META policy",
            "bounded connector-compatible Git Data mode only under its exact one-writer/predecessor/one-commit/non-force/post-readback conditions",
            "Ad-hoc raw Git Data reconstruction",
            "sequential per-file API publication",
            "force/ref replacement",
            "reset and rebase remain forbidden",
        )
        contradictions = (
            r"\bselected candidate\b[^.]{0,100}\b(?:may|can)\s+be\s+(?:reconstructed|relabelled|relabeled)\b",
            r"\bpublication control\b[^.]{0,100}\b(?:may|can)\s+(?:remain|stay)\s+with\s+(?:the\s+)?(?:worker|agent)\b",
            r"\bAd-hoc raw Git Data reconstruction\b[^.]{0,100}\b(?:is|are|be|remain)?\s*(?:allowed|permitted|authorized)\b",
            r"\bsequential per-file API publication\b[^.]{0,100}\b(?:is|are|be|remain)?\s*(?:allowed|permitted|authorized)\b",
            r"\bforce/ref replacement\b[^.]{0,100}\b(?:is|are|be|remain)?\s*(?:allowed|permitted|authorized)\b",
            r"\breset and rebase\b[^.]{0,100}\b(?:is|are|be|remain)?\s*(?:allowed|permitted|authorized)\b",
            r"\b(?:may|can|should)\s+(?:use|perform)\b[^.]{0,100}\b(?:raw Git Data|sequential per-file|force/ref replacement|reset|rebase)\b",
        )
        return (
            all(value in clause for value in required)
            and not any(re.search(pattern, clause, re.IGNORECASE) for pattern in contradictions)
        )

    def test_current_repository_adopts_authenticated_central_policy(self) -> None:
        self.assertEqual([], validate_policy(REPO_ROOT, self.meta_root, self.authority))

    def test_publication_integrity_authority_and_fallback_are_bound(self) -> None:
        self.assertEqual(self.binding["authority_commit"], PUBLICATION_INTEGRITY_AUTHORITY)

        workflow = (REPO_ROOT / ".github/workflows/agent-governance.yml").read_text(encoding="utf-8")
        self.assertEqual(
            ("Oteryn/Oteryn", PUBLICATION_INTEGRITY_AUTHORITY),
            self._bound_meta_checkout_target(workflow),
        )
        stale_workflow = workflow.replace(
            f"          ref: {PUBLICATION_INTEGRITY_AUTHORITY}",
            "          # stale evidence only: ref: "
            f"{PUBLICATION_INTEGRITY_AUTHORITY}\n"
            "          ref: 33b212e652c680bd4047be3b414c9a358b8bf26f",
            1,
        )
        self.assertEqual(
            ("Oteryn/Oteryn", "33b212e652c680bd4047be3b414c9a358b8bf26f"),
            self._bound_meta_checkout_target(stale_workflow),
        )

        bootstrap = (REPO_ROOT / "docs/agents/PLATFORM_AGENT_BOOTSTRAP.md").read_text(encoding="utf-8")
        clause = self._publication_fallback_clause(bootstrap)
        self.assertTrue(self._publication_fallback_clause_is_bound(clause))
        for required in (
            "do not silently reconstruct or relabel that selected candidate",
            "Preserve its custody and return publication control to the active control plane",
            "The control plane may select only an API-native **new candidate** route permitted by the bound META policy",
            "bounded connector-compatible Git Data mode only under its exact one-writer/predecessor/one-commit/non-force/post-readback conditions",
            "Ad-hoc raw Git Data reconstruction",
            "sequential per-file API publication",
            "force/ref replacement",
            "reset and rebase remain forbidden",
        ):
            self.assertFalse(
                self._publication_fallback_clause_is_bound(clause.replace(required, "", 1)),
                required,
            )
        for additive_contradiction in (
            " Selected candidate may be reconstructed for recovery.",
            " Publication control may remain with the worker during recovery.",
            " Ad-hoc raw Git Data reconstruction is allowed for recovery.",
            " Sequential per-file API publication is permitted for recovery.",
            " Force/ref replacement is authorized for recovery.",
            " Reset and rebase are allowed for recovery.",
            " The worker may use reset when recovery is difficult.",
        ):
            self.assertFalse(
                self._publication_fallback_clause_is_bound(
                    clause + additive_contradiction
                ),
                additive_contradiction,
            )
        contradictory = bootstrap.replace(
            clause,
            clause + " Reset and rebase are allowed for recovery.",
            1,
        ) + "\n\nHistorical note: reset and rebase remain forbidden.\n"
        self.assertFalse(
            self._publication_fallback_clause_is_bound(
                self._publication_fallback_clause(contradictory)
            )
        )

        contract = (self.meta_root / "docs/agents/contracts/PUBLICATION_INTEGRITY_POLICY.md").read_text(encoding="utf-8")
        for value in (
            "bounded connector-compatible Git Data route",
            "Either API route creates a **new candidate**",
            "GitHub Git-refs `force=false` proves only ancestry",
            "sequential per-file commits",
            "partial/mixed reconstruction remain forbidden",
        ):
            self.assertIn(value, contract)

    def test_authentication_rejects_dirty_executable_policy_checkout(self) -> None:
        temporary = tempfile.TemporaryDirectory(); self.addCleanup(temporary.cleanup)
        clone = Path(temporary.name) / "meta"
        subprocess.run(["git", "clone", "--quiet", "--no-hardlinks", str(self.meta_root), str(clone)], check=True)
        path = clone / "tools/governance/central_agent_policy.py"
        path.write_text(path.read_text(encoding="utf-8") + "\n# tampered\n", encoding="utf-8")
        with self.assertRaisesRegex(PolicyConsistencyError, "must be clean"):
            authenticate_meta_checkout(clone, self.binding, github_json=self._fake_github(clone, self.binding))

    def test_authentication_rejects_blob_mismatch_before_code_load(self) -> None:
        with self.assertRaisesRegex(PolicyConsistencyError, "does not match authenticated GitHub blob"):
            authenticate_meta_checkout(
                self.meta_root,
                self.binding,
                github_json=self._fake_github(
                    self.meta_root,
                    self.binding,
                    corrupt_path="tools/governance/central_agent_policy.py",
                ),
            )

    def test_missing_binding_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        (root / BINDING_PATH).unlink()
        self.assertIn("cannot read valid JSON", self._findings(root))

    def test_binding_version_drift_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / BINDING_PATH
        binding = json.loads(path.read_text(encoding="utf-8"))
        binding["policy_version"] = "999.0.0"
        path.write_text(json.dumps(binding), encoding="utf-8")
        self.assertIn("policy_version", self._findings(root))

    def test_binding_commit_must_match_authenticated_authority(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / BINDING_PATH
        binding = json.loads(path.read_text(encoding="utf-8"))
        binding["authority_commit"] = "0" * 40
        path.write_text(json.dumps(binding), encoding="utf-8")
        self.assertIn("could not be resolved", self._findings(root))

    def test_root_must_resolve_binding(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8").replace("Resolve `docs/agents/META_AGENT_POLICY_BINDING.json`", "Mention `docs/agents/META_AGENT_POLICY_BINDING.json`", 1), encoding="utf-8")
        self.assertIn("provider overlay must resolve", self._findings(root))

    def test_root_parallel_first_policy_is_rejected_by_central_validator(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8") + "\nAgents must use parallel-first execution.\n", encoding="utf-8")
        self.assertIn("parallel-first execution wording is forbidden", self._findings(root))

    def test_active_prompt_cannot_embed_global_execution_policy(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        catalog = json.loads((root / CATALOG_PATH).read_text(encoding="utf-8"))
        active = next(entry for entry in catalog["prompts"] if entry["executable"] is True)
        path = root / active["path"]
        path.write_text(path.read_text(encoding="utf-8") + "\n## GitHub-first execution\n", encoding="utf-8")
        self.assertIn("task prompt must not copy organization-wide policy sections", self._findings(root))

    def test_historical_prompt_body_is_not_an_active_policy_consumer(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        catalog = json.loads((root / CATALOG_PATH).read_text(encoding="utf-8"))
        historical = next(entry for entry in catalog["prompts"] if entry["executable"] is False)
        path = root / historical["path"]
        path.write_text(path.read_text(encoding="utf-8") + "\n## GitHub-first execution\n", encoding="utf-8")
        self.assertEqual([], validate_policy(root, self.meta_root, self.authority))

    def test_executable_prompt_requires_active_reusable_classification(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / CATALOG_PATH
        catalog = json.loads(path.read_text(encoding="utf-8"))
        active = next(entry for entry in catalog["prompts"] if entry["executable"] is True)
        active["status"] = "historical_do_not_run"
        path.write_text(json.dumps(catalog), encoding="utf-8")
        self.assertIn("executable prompt must be reusable/active_reusable", self._findings(root))

    def test_historical_prompt_requires_inert_lifecycle(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / CATALOG_PATH
        catalog = json.loads(path.read_text(encoding="utf-8"))
        historical = next(entry for entry in catalog["prompts"] if entry["executable"] is False)
        historical["classification"] = "reusable"
        path.write_text(json.dumps(catalog), encoding="utf-8")
        self.assertIn("inert prompt must be one_shot_historical/historical_do_not_run", self._findings(root))

    def test_duplicate_prompt_path_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / CATALOG_PATH
        catalog = json.loads(path.read_text(encoding="utf-8"))
        catalog["prompts"].append(dict(catalog["prompts"][0]))
        path.write_text(json.dumps(catalog), encoding="utf-8")
        self.assertIn("duplicate prompt path", self._findings(root))


if __name__ == "__main__":
    unittest.main(verbosity=2)
