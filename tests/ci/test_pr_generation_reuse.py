#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts" / "ci"
FIXTURE = Path(__file__).parent / "fixtures" / "pr-generation-reuse-cases.json"

sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location(
    "pr_generation_reuse", SCRIPTS / "pr_generation_reuse.py"
)
if spec is None or spec.loader is None:
    raise RuntimeError("unable to load pr_generation_reuse.py")
reuse = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = reuse
spec.loader.exec_module(reuse)


class FakeClient:
    def __init__(self, runs: list[dict[str, object]], jobs: dict[int, list[dict[str, object]]]):
        self._runs = runs
        self._jobs = jobs

    def successful_pr_runs(self, workflow_file: str) -> list[dict[str, object]]:
        return list(self._runs)

    def jobs(self, run_id: int) -> list[dict[str, object]]:
        return list(self._jobs.get(run_id, []))


class PullRequestGenerationReuseTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))

    def test_required_acceptance_cases(self) -> None:
        names = {case["name"] for case in self.fixture["cases"]}
        self.assertTrue(
            {
                "material_change_runs",
                "checkpoint_successor_reuses",
                "material_after_checkpoint_runs",
                "ambiguous_generation_runs",
            }.issubset(names)
        )

        for case in self.fixture["cases"]:
            before = case["before"]
            head = case["head"]

            def changed_paths_fn(_before: str, _head: str) -> list[str]:
                self.assertEqual(before, _before)
                self.assertEqual(head, _head)
                return list(case["generation_paths"])

            def digest_fn(commit: str, _gate: str) -> str:
                if commit == before:
                    return str(case["before_digest"])
                if commit == head:
                    return str(case["head_digest"])
                return str(case["head_digest"])

            def evidence_lookup_fn(**kwargs: object) -> reuse.ReuseEvidence | None:
                self.assertEqual("ci", kwargs["gate"])
                value = case["prior_success"]
                if value == "error":
                    raise reuse.GitHubApiError("synthetic API failure")
                if value:
                    return reuse.ReuseEvidence(
                        run_id=4242,
                        head_sha="cccccccccccccccccccccccccccccccccccccccc",
                        job_name="runtime-tests",
                    )
                return None

            decision = reuse.resolve_decision(
                gate="ci",
                workflow_file="ci.yml",
                evidence_job="runtime-tests",
                event_name="pull_request",
                event_action=case["event_action"],
                accumulated_required=case["accumulated_required"],
                before=before,
                after=case["after"],
                head=head,
                pr_number="1012",
                repository="Oteryn/Oteryn-Platform",
                changed_paths_fn=changed_paths_fn,
                digest_fn=digest_fn,
                ancestor_fn=lambda _a, _b: True,
                evidence_lookup_fn=evidence_lookup_fn,
            )
            with self.subTest(case=case["name"]):
                self.assertEqual(case["expected_required"], decision.required)
                self.assertEqual(case["expected_decision"], decision.decision)
                self.assertEqual(case["expected_reason"], decision.reason)
                if decision.decision == "reuse":
                    self.assertEqual(4242, decision.reused_run_id)

    def test_non_synchronize_material_scope_runs(self) -> None:
        decision = reuse.resolve_decision(
            gate="edge",
            workflow_file="edge-security-emulation.yml",
            evidence_job="validate",
            event_name="pull_request",
            event_action="opened",
            accumulated_required="true",
            before="",
            after="",
            head="b" * 40,
            pr_number="1012",
            repository="Oteryn/Oteryn-Platform",
        )
        self.assertTrue(decision.required)
        self.assertEqual("non_synchronize_pull_request", decision.reason)

    def test_force_push_or_unrelated_generation_fails_closed(self) -> None:
        decision = reuse.resolve_decision(
            gate="phase7",
            workflow_file="phase7-production-like-validation.yml",
            evidence_job="validate",
            event_name="pull_request",
            event_action="synchronize",
            accumulated_required="true",
            before="a" * 40,
            after="b" * 40,
            head="b" * 40,
            pr_number="1012",
            repository="Oteryn/Oteryn-Platform",
            ancestor_fn=lambda _a, _b: False,
        )
        self.assertTrue(decision.required)
        self.assertEqual("generation_not_fast_forward", decision.reason)

    def test_source_evidence_requires_successful_heavy_job(self) -> None:
        candidate = "c" * 40
        current = "d" * 40
        client = FakeClient(
            runs=[
                {
                    "id": 44,
                    "head_sha": candidate,
                    "pull_requests": [{"number": 1012}],
                }
            ],
            jobs={44: [{"name": "validate", "conclusion": "skipped"}]},
        )
        evidence = reuse.find_reusable_evidence(
            client=client,
            workflow_file="edge-security-emulation.yml",
            evidence_job="validate",
            pr_number=1012,
            gate="edge",
            current_head=current,
            current_digest="sha256:same",
            digest_fn=lambda _sha, _gate: "sha256:same",
            ancestor_fn=lambda _a, _b: True,
        )
        self.assertIsNone(evidence)

    def test_source_evidence_is_same_pr_same_digest_success(self) -> None:
        good = "c" * 40
        wrong_pr = "e" * 40
        current = "d" * 40
        client = FakeClient(
            runs=[
                {
                    "id": 45,
                    "head_sha": wrong_pr,
                    "pull_requests": [{"number": 9999}],
                },
                {
                    "id": 44,
                    "head_sha": good,
                    "pull_requests": [{"number": 1012}],
                },
            ],
            jobs={44: [{"name": "validate", "conclusion": "success"}]},
        )
        evidence = reuse.find_reusable_evidence(
            client=client,
            workflow_file="edge-security-emulation.yml",
            evidence_job="validate",
            pr_number=1012,
            gate="edge",
            current_head=current,
            current_digest="sha256:same",
            digest_fn=lambda sha, _gate: (
                "sha256:same" if sha == good else "sha256:different"
            ),
            ancestor_fn=lambda _a, _b: True,
        )
        self.assertIsNotNone(evidence)
        assert evidence is not None
        self.assertEqual(44, evidence.run_id)
        self.assertEqual(good, evidence.head_sha)

    def test_material_tree_digest_ignores_docs_but_changes_for_runtime(self) -> None:
        old_cwd = os.getcwd()
        try:
            with tempfile.TemporaryDirectory() as directory:
                os.chdir(directory)
                subprocess.run(["git", "init", "-q"], check=True)
                subprocess.run(["git", "config", "user.email", "ci@example.test"], check=True)
                subprocess.run(["git", "config", "user.name", "CI"], check=True)
                Path("app").mkdir()
                Path("docs").mkdir()
                Path("app/Runtime.php").write_text("<?php\nreturn 1;\n", encoding="utf-8")
                Path("docs/note.md").write_text("one\n", encoding="utf-8")
                subprocess.run(["git", "add", "."], check=True)
                subprocess.run(["git", "commit", "-qm", "base"], check=True)
                base = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
                base_digest = reuse.material_tree_digest(base, "ci")

                Path("docs/note.md").write_text("two\n", encoding="utf-8")
                subprocess.run(["git", "add", "."], check=True)
                subprocess.run(["git", "commit", "-qm", "docs"], check=True)
                docs_head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
                self.assertEqual(base_digest, reuse.material_tree_digest(docs_head, "ci"))

                Path("app/Runtime.php").write_text("<?php\nreturn 2;\n", encoding="utf-8")
                subprocess.run(["git", "add", "."], check=True)
                subprocess.run(["git", "commit", "-qm", "runtime"], check=True)
                runtime_head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
                self.assertNotEqual(
                    base_digest,
                    reuse.material_tree_digest(runtime_head, "ci"),
                )
        finally:
            os.chdir(old_cwd)

    def test_output_records_reuse_provenance(self) -> None:
        decision = reuse.ReuseDecision(
            required=False,
            decision="reuse",
            reason="prior_success_same_material_tree",
            material_digest="sha256:abc",
            generation_paths=("docs/agents/task.md",),
            reused_run_id=12345,
            reused_head_sha="a" * 40,
            reused_job_name="runtime-tests",
        )
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "output"
            summary = Path(directory) / "summary"
            reuse.write_github_output(output, decision)
            reuse.write_summary(
                summary,
                gate="ci",
                workflow_file="ci.yml",
                evidence_job="runtime-tests",
                head="b" * 40,
                decision=decision,
            )
            output_text = output.read_text(encoding="utf-8")
            summary_text = summary.read_text(encoding="utf-8")
        self.assertIn("required=false", output_text)
        self.assertIn("reused_run_id=12345", output_text)
        self.assertIn("reused_head_sha=" + "a" * 40, output_text)
        self.assertIn("reused workflow run: `12345`", summary_text)
        self.assertIn("ambiguity", summary_text)


if __name__ == "__main__":
    unittest.main()
