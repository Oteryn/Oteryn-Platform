#!/usr/bin/env python3

from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEPENDABOT_CONFIG = ROOT / ".github" / "dependabot.yml"
WORKFLOWS_DIR = ROOT / ".github" / "workflows"


def _dependabot_ecosystem_block(config: str, ecosystem: str) -> str:
    entries = list(
        re.finditer(
            r"(?m)^  - package-ecosystem:\s*([^\s#]+)\s*$",
            config,
        )
    )
    for index, match in enumerate(entries):
        if match.group(1).strip('"\'') != ecosystem:
            continue
        end = entries[index + 1].start() if index + 1 < len(entries) else len(config)
        return config[match.start() : end]
    raise AssertionError(f"Dependabot ecosystem {ecosystem!r} is not configured")


def _codeql_action_refs() -> list[tuple[str, str, str]]:
    pattern = re.compile(
        r"(?m)^\s*uses:\s*github/codeql-action/([^@\s]+)@([0-9a-f]{40})(?:\s+#.*)?$"
    )
    refs: list[tuple[str, str, str]] = []
    for workflow in sorted(WORKFLOWS_DIR.glob("*.yml")):
        text = workflow.read_text(encoding="utf-8")
        for component, sha in pattern.findall(text):
            refs.append((workflow.name, component, sha))
    return refs


class DependencyUpdateContractTest(unittest.TestCase):
    def test_dependabot_groups_coupled_codeql_actions(self) -> None:
        config = DEPENDABOT_CONFIG.read_text(encoding="utf-8")
        github_actions = _dependabot_ecosystem_block(config, "github-actions")

        self.assertRegex(
            github_actions,
            r"(?m)^    groups:\s*$",
            "github-actions Dependabot updates must define groups",
        )
        self.assertRegex(
            github_actions,
            r"(?ms)^      codeql-actions:\s*$.*?^        patterns:\s*$.*?^          - [\"']github/codeql-action/\*[\"']\s*$",
            "github/codeql-action/* must be updated as one atomic Dependabot group",
        )

    def test_all_codeql_steps_share_one_immutable_revision(self) -> None:
        refs = _codeql_action_refs()
        self.assertGreaterEqual(len(refs), 2, "expected coupled CodeQL action steps")

        components = {component for _, component, _ in refs}
        self.assertIn("init", components, "CodeQL init step is missing")
        self.assertIn("analyze", components, "CodeQL analyze step is missing")

        revisions = {sha for _, _, sha in refs}
        self.assertEqual(
            1,
            len(revisions),
            "all github/codeql-action steps must use the same immutable commit SHA: "
            + ", ".join(f"{workflow}:{component}@{sha}" for workflow, component, sha in refs),
        )


if __name__ == "__main__":
    unittest.main()
