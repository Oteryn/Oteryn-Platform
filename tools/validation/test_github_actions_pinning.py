#!/usr/bin/env python3
from __future__ import annotations

import base64
import json
import tempfile
import unittest
from pathlib import Path

import github_actions_pinning as pinning

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "ci" / "fixtures" / "github-actions-pinning"


class GitHubActionsPinningTests(unittest.TestCase):
    def test_classification_cases(self) -> None:
        cases = json.loads((FIXTURES / "cases.json").read_text(encoding="utf-8"))
        for case in cases:
            with self.subTest(case=case["name"]):
                value = base64.b64decode(case["uses_b64"]).decode("utf-8")
                kind, _ = pinning.classify_uses(value)
                self.assertEqual(case["expected"], kind)

    def test_valid_sha_workflow_fixture_passes(self) -> None:
        inventory, findings = pinning.scan_files([FIXTURES / "valid-sha.yml"], ROOT)
        self.assertEqual([], findings)
        self.assertEqual(2, len(inventory))
        self.assertEqual({"external_sha"}, {entry["kind"] for entry in inventory})

    def test_local_and_docker_forms_are_technically_distinct(self) -> None:
        self.assertEqual("local", pinning.classify_uses("./.github/actions/example")[0])
        self.assertEqual("docker", pinning.classify_uses("docker://alpine:3.20")[0])

    def test_malformed_empty_uses_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "bad.yml"
            path.write_text("jobs:\n  x:\n    uses:\n", encoding="utf-8")
            _, findings = pinning.scan_files([path])
            self.assertEqual(1, len(findings))
            self.assertEqual("malformed", findings[0].kind)

    def test_quoted_sha_and_trailing_comment_pass(self) -> None:
        value = "actions/checkout@0123456789abcdef0123456789abcdef01234567"
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "quoted.yml"
            path.write_text(f'jobs:\n  x:\n    uses: "{value}" # v7.0.1\n', encoding="utf-8")
            _, findings = pinning.scan_files([path])
            self.assertEqual([], findings)


if __name__ == "__main__":
    unittest.main()
