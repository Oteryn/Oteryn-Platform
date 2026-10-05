#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts" / "ci" / "heavy_evidence_reuse.py"
CI_SCRIPT_DIR = ROOT / "scripts" / "ci"
if str(CI_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(CI_SCRIPT_DIR))

spec = importlib.util.spec_from_file_location("heavy_evidence_reuse", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"Unable to load {MODULE_PATH}")
reuse = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = reuse
spec.loader.exec_module(reuse)


class GitRepoFixture:
    def __init__(self, root: Path) -> None:
        self.root = root

    def git(self, *args: str) -> str:
        return subprocess.check_output(
            ["git", *args],
            cwd=self.root,
            text=True,
            stderr=subprocess.STDOUT,
        ).strip()

    def write(self, path: str, content: str) -> None:
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")

    def commit(self, message: str) -> str:
        self.git("add", "-A")
        self.git("commit", "-m", message)
        return self.git("rev-parse", "HEAD")


class HeavyEvidenceReuseTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.repo_path = Path(self.temporary.name)
        subprocess.run(["git", "init", "-q"], cwd=self.repo_path, check=True)
        subprocess.run(
            ["git", "config", "user.email", "ci@example.test"],
            cwd=self.repo_path,
            check=True,
        )
        subprocess.run(
            ["git", "config", "user.name", "CI Test"],
            cwd=self.repo_path,
            check=True,
        )
        self.repo = GitRepoFixture(self.repo_path)
        self.repo.write("README.md", "base\n")
        self.base = self.repo.commit("base")
        self.old_cwd = Path.cwd()
        os.chdir(self.repo_path)

    def tearDown(self) -> None:
        os.chdir(self.old_cwd)
        self.temporary.cleanup()

    def _material_commit(self, value: str = "one") -> str:
        self.repo.write("app/Feature.php", f"<?php // {value}\n")
        return self.repo.commit(f"material {value}")

    def _checkpoint_commit(self, value: str = "one") -> str:
        self.repo.write(
            "docs/agents/tasks/active/example.md",
            f"checkpoint {value}\n",
        )
        return self.repo.commit(f"checkpoint {value}")

    def _successful_api(self, material_head: str, *, pr_number: int = 12, job: str = "validate"):
        def get_json(url: str, token: str):
            self.assertEqual("token", token)
            if "/runs?" in url:
                return {
                    "total_count": 1,
                    "workflow_runs": [
                        {
                            "id": 77,
                            "event": "pull_request",
                            "conclusion": "success",
                            "head_sha": material_head,
                            "pull_requests": [{"number": pr_number}],
                        }
                    ],
                }
            if "/actions/runs/77/jobs" in url:
                return {
                    "total_count": 1,
                    "jobs": [{"name": job, "conclusion": "success"}],
                }
            raise AssertionError(f"unexpected URL: {url}")

        return get_json

    def test_material_change_runs_heavy_validation(self) -> None:
        head = self._material_commit()
        plan = reuse.plan_reuse(
            event_name="pull_request",
            base=self.base,
            head=head,
            gate="phase7",
        )

        self.assertFalse(plan.candidate)
        self.assertEqual(head, plan.material_head)
        self.assertEqual(plan.material_digest, plan.current_digest)
        self.assertIn("final head contains a material change", plan.reason)

    def test_checkpoint_successor_reuses_exact_same_pr_heavy_evidence(self) -> None:
        material = self._material_commit()
        head = self._checkpoint_commit()
        decision = reuse.decide_reuse(
            event_name="pull_request",
            base=self.base,
            head=head,
            gate="phase7",
            repository="Oteryn/Oteryn-Platform",
            workflow="phase7-production-like-validation.yml",
            pr_number=12,
            evidence_job="validate",
            token="token",
            get_json=self._successful_api(material),
        )

        self.assertFalse(decision.validation_required)
        self.assertEqual("REUSE", decision.state)
        self.assertEqual(material, decision.material_head)
        self.assertEqual(77, decision.reused_run_id)
        self.assertEqual(decision.material_digest, decision.current_digest)

    def test_material_change_after_checkpoint_runs_heavy_validation_again(self) -> None:
        self._material_commit("one")
        self._checkpoint_commit()
        head = self._material_commit("two")
        plan = reuse.plan_reuse(
            event_name="pull_request",
            base=self.base,
            head=head,
            gate="phase7",
        )

        self.assertFalse(plan.candidate)
        self.assertEqual(head, plan.material_head)
        self.assertIn("final head contains a material change", plan.reason)

    def test_ambiguous_history_fails_closed_to_heavy_validation(self) -> None:
        head = self._material_commit()
        plan = reuse.plan_reuse(
            event_name="pull_request",
            base="0" * 40,
            head=head,
            gate="phase7",
        )

        self.assertFalse(plan.candidate)
        self.assertIsNone(plan.material_head)
        self.assertIn("validate fail-closed", plan.reason)

    def test_missing_same_pr_run_fails_closed(self) -> None:
        material = self._material_commit()
        head = self._checkpoint_commit()

        def wrong_pr(url: str, token: str):
            if "/runs?" in url:
                return {
                    "total_count": 1,
                    "workflow_runs": [
                        {
                            "id": 88,
                            "event": "pull_request",
                            "conclusion": "success",
                            "head_sha": material,
                            "pull_requests": [{"number": 999}],
                        }
                    ],
                }
            raise AssertionError("jobs must not be queried for a different PR")

        decision = reuse.decide_reuse(
            event_name="pull_request",
            base=self.base,
            head=head,
            gate="phase7",
            repository="Oteryn/Oteryn-Platform",
            workflow="phase7-production-like-validation.yml",
            pr_number=12,
            evidence_job="validate",
            token="token",
            get_json=wrong_pr,
        )

        self.assertTrue(decision.validation_required)
        self.assertEqual("RUN", decision.state)
        self.assertIsNone(decision.reused_run_id)

    def test_successful_workflow_without_successful_heavy_job_fails_closed(self) -> None:
        material = self._material_commit()
        head = self._checkpoint_commit()
        decision = reuse.decide_reuse(
            event_name="pull_request",
            base=self.base,
            head=head,
            gate="phase7",
            repository="Oteryn/Oteryn-Platform",
            workflow="phase7-production-like-validation.yml",
            pr_number=12,
            evidence_job="validate",
            token="token",
            get_json=self._successful_api(material, job="classify-changes"),
        )

        self.assertTrue(decision.validation_required)
        self.assertEqual("RUN", decision.state)
        self.assertIsNone(decision.reused_run_id)

    def test_non_pull_request_events_always_validate(self) -> None:
        head = self._material_commit()
        decision = reuse.decide_reuse(
            event_name="merge_group",
            base=self.base,
            head=head,
            gate="phase7",
            repository="Oteryn/Oteryn-Platform",
            workflow="phase7-production-like-validation.yml",
            pr_number=0,
            evidence_job="validate",
            token="",
        )

        self.assertTrue(decision.validation_required)
        self.assertEqual("RUN", decision.state)
        self.assertIn("non-pull-request", decision.reason)


if __name__ == "__main__":
    unittest.main()
