#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

HEAVY_WORKFLOWS = (
    "edge-security-emulation.yml",
    "platform-db-outage-validation.yml",
    "phase7-production-like-validation.yml",
    "game-auth-ticket-concurrency.yml",
)

REQUIRED_IGNORES = (
    "'AGENTS.md'",
    "'AGENTS.override.md'",
    "'docs/agents/**'",
    "'.github/ISSUE_TEMPLATE/**'",
    "'.github/PULL_REQUEST_TEMPLATE.md'",
)

for filename in HEAVY_WORKFLOWS:
    path = ROOT / ".github" / "workflows" / filename
    text = path.read_text(encoding="utf-8")
    trigger = text.split("permissions:", 1)[0]

    assert "pull_request:" in trigger, f"{filename}: missing pull_request trigger"
    assert "    paths-ignore:\n" in trigger, (
        f"{filename}: governance/checkpoint changes would still create a workflow run"
    )
    for ignored in REQUIRED_IGNORES:
        assert ignored in trigger, f"{filename}: missing trigger ignore {ignored}"

    assert "cancel-in-progress: true" in text, (
        f"{filename}: superseded same-PR runs are not cancelled"
    )
    assert "scripts/ci/classify_changes.py" in text, (
        f"{filename}: internal fail-closed path classification was removed"
    )

agent_governance = (
    ROOT / ".github" / "workflows" / "agent-governance.yml"
).read_text(encoding="utf-8")
assert "group: agent-governance-${{ github.event.pull_request.number || github.ref }}" in agent_governance
assert "cancel-in-progress: true" in agent_governance

ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
assert "cancel-in-progress: true" in ci
assert "scripts/ci/classify_changes.py" in ci

print("workflow trigger economy contract: PASS")
