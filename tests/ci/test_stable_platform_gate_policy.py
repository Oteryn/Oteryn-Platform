from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / "docs/operations/github-repository-policy.json"
CI = ROOT / ".github/workflows/ci.yml"


def test_policy_requires_only_stable_platform_gate() -> None:
    policy = json.loads(POLICY.read_text(encoding="utf-8"))
    checks = policy["branch_protection"]["required_status_checks"]
    assert checks["strict"] is False
    assert checks["contexts"] == ["platform-gate"]


def test_platform_gate_fails_closed_over_legacy_aggregates() -> None:
    workflow = CI.read_text(encoding="utf-8")
    required = (
        "  platform_gate:\n",
        "    name: platform-gate\n",
        "      - classify_changes\n",
        "      - test\n",
        '          CLASSIFY_CHANGES_RESULT: ${{ needs.classify_changes.result }}\n',
        '          TEST_RESULT: ${{ needs.test.result }}\n',
        '          test "$CLASSIFY_CHANGES_RESULT" = success\n',
        '          test "$TEST_RESULT" = success\n',
    )
    for fragment in required:
        assert fragment in workflow, fragment
