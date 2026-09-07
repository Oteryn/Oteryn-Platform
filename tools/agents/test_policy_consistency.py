#!/usr/bin/env python3
"""Focused regressions for the Platform consumer of central META policy."""

from __future__ import annotations

import json
import os
from pathlib import Path
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

    def test_current_repository_adopts_authenticated_central_policy(self) -> None:
        self.assertEqual([], validate_policy(REPO_ROOT, self.meta_root, self.authority))

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
