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

ACCEPTANCE_PRODUCT_PATHS = (
    "'app/**'",
    "'bootstrap/**'",
    "'config/**'",
    "'database/**'",
    "'public/**'",
    "'resources/views/**'",
    "'routes/**'",
    "'scripts/acceptance/**'",
    "'.github/workflows/acceptance-validation.yml'",
    "'composer.json'",
    "'composer.lock'",
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
assert "scripts/ci/classify_push_changes.py" in ci
assert "github.event.pull_request.base.sha || github.event.before || ''" in ci
assert "python tests/ci/test_push_change_routing.py" in ci

acceptance = (
    ROOT / ".github" / "workflows" / "acceptance-validation.yml"
).read_text(encoding="utf-8")
acceptance_trigger = acceptance.split("permissions:", 1)[0]
assert "  pull_request:\n" in acceptance_trigger
assert "  push:\n" in acceptance_trigger
assert "  workflow_dispatch:\n" in acceptance_trigger
assert "  workflow_call:\n" in acceptance_trigger

pull_request_block, after_pull_request = acceptance_trigger.split("  push:\n", 1)
push_block = after_pull_request.split("  workflow_dispatch:\n", 1)[0]
assert "    branches:\n      - main\n    paths:\n" in push_block, (
    "acceptance-validation.yml: main pushes are not product-path filtered"
)
for product_path in ACCEPTANCE_PRODUCT_PATHS:
    assert product_path in pull_request_block, (
        f"acceptance-validation.yml: pull_request missing product path {product_path}"
    )
    assert product_path in push_block, (
        f"acceptance-validation.yml: push main missing product path {product_path}"
    )

assert "group: acceptance-e2e-${{ github.workflow }}-${{ github.ref }}-${{ inputs.run_suffix || 'direct' }}" in acceptance
assert "cancel-in-progress: true" in acceptance

print("workflow trigger economy contract: PASS")
