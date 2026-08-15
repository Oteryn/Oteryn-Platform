#!/usr/bin/env python3
from __future__ import annotations

import re
import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_ROOT = ROOT / ".github" / "workflows"

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

PORTAL_ACCEPTANCE_PATHS = (
    "'app/**'",
    "'bootstrap/**'",
    "'config/**'",
    "'database/**'",
    "'public/css/**'",
    "'public/js/**'",
    "'resources/views/**'",
    "'routes/**'",
    "'scripts/acceptance/**'",
    "'docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json'",
    "'docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json'",
    "'docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md'",
    "'docs/testing/product-completeness-benchmark.json'",
    "'docs/testing/product-backend-frontend-completeness.json'",
    "'.github/workflows/portal-acceptance-contract.yml'",
    "'composer.json'",
    "'composer.lock'",
)

RETIRED_WORKFLOWS = (
    "account-security-format-diagnostics.yml",
    "account-security-static-diagnostics.yml",
    "deep-system-validation.yml",
    "portal-exhaustive-acceptance.yml",
    "portal-exhaustive-audit.yml",
    "portal-exhaustive-trigger-coupling.yml",
)

GLOBAL_PULL_REQUEST_WORKFLOWS = {
    "ci.yml",
    "github-actions-storage-hygiene.yml",
}

GLOBAL_MAIN_PUSH_WORKFLOWS = {
    "ci.yml",
}


def trigger_prefix(text: str) -> str:
    return text.split("permissions:", 1)[0]


def event_block(trigger: str, event: str) -> str | None:
    lines = trigger.splitlines()
    header = f"  {event}:"
    try:
        start = lines.index(header)
    except ValueError:
        return None

    end = len(lines)
    for index in range(start + 1, len(lines)):
        line = lines[index]
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not line.startswith(" "):
            end = index
            break
        if re.match(r"^  [A-Za-z0-9_-]+:\s*$", line):
            end = index
            break
    return "\n".join(lines[start:end]) + "\n"


def has_path_filter(block: str | None) -> bool:
    if block is None:
        return False
    return "    paths:\n" in block or "    paths-ignore:\n" in block


def targets_main(block: str | None) -> bool:
    if block is None:
        return False
    return bool(re.search(r"(?m)^      - ['\"]?main['\"]?\s*$", block))


def has_superseded_run_cancellation(text: str) -> bool:
    match = re.search(r"(?m)^  cancel-in-progress:\s*(.+)$", text)
    if match is None:
        return False
    value = match.group(1).strip()
    if value == "true":
        return True
    # A workflow that also runs on push/manual events may cancel only the PR
    # generation. This is equivalent to literal true for the supersedable PR
    # path while deliberately preserving independent non-PR executions.
    return "github.event_name == 'pull_request'" in value


for filename in HEAVY_WORKFLOWS:
    path = WORKFLOW_ROOT / filename
    text = path.read_text(encoding="utf-8")
    trigger = trigger_prefix(text)

    assert "pull_request:" in trigger, f"{filename}: missing pull_request trigger"
    assert "    paths-ignore:\n" in trigger, (
        f"{filename}: governance/checkpoint changes would still create a workflow run"
    )
    for ignored in REQUIRED_IGNORES:
        assert ignored in trigger, f"{filename}: missing trigger ignore {ignored}"

    assert has_superseded_run_cancellation(text), (
        f"{filename}: superseded same-PR runs are not cancelled"
    )
    assert "scripts/ci/classify_changes.py" in text, (
        f"{filename}: internal fail-closed path classification was removed"
    )

for filename in RETIRED_WORKFLOWS:
    assert not (WORKFLOW_ROOT / filename).exists(), (
        f"{filename}: retired task/diagnostic workflow was reintroduced"
    )

for path in sorted([*WORKFLOW_ROOT.glob("*.yml"), *WORKFLOW_ROOT.glob("*.yaml")]):
    text = path.read_text(encoding="utf-8")
    trigger = trigger_prefix(text)
    pull_request = event_block(trigger, "pull_request")
    push = event_block(trigger, "push")

    if pull_request is not None and path.name not in GLOBAL_PULL_REQUEST_WORKFLOWS:
        assert has_path_filter(pull_request), (
            f"{path.name}: pull_request must use paths/paths-ignore or be an "
            "explicit repository-wide control-plane workflow"
        )
        assert has_superseded_run_cancellation(text), (
            f"{path.name}: supersedable pull_request workflow lacks cancel-in-progress"
        )

    if (
        push is not None
        and targets_main(push)
        and path.name not in GLOBAL_MAIN_PUSH_WORKFLOWS
    ):
        assert has_path_filter(push), (
            f"{path.name}: push to main must use paths/paths-ignore or be an "
            "explicit repository-wide control-plane workflow"
        )

agent_governance = (WORKFLOW_ROOT / "agent-governance.yml").read_text(encoding="utf-8")
assert (
    "group: agent-governance-${{ github.event.pull_request.number || github.ref }}"
    in agent_governance
)
assert has_superseded_run_cancellation(agent_governance)

ci = (WORKFLOW_ROOT / "ci.yml").read_text(encoding="utf-8")
assert (
    "group: ci-${{ github.workflow }}-${{ github.event.pull_request.number || github.sha }}"
    in ci
), "ci.yml: CI generations are not isolated by PR identity or push commit SHA"
assert "cancel-in-progress: ${{ github.event_name == 'pull_request' }}" in ci, (
    "ci.yml: main-push generations may still be cancelled by a later main push"
)
assert "group: ci-${{ github.workflow }}-${{ github.ref }}" not in ci, (
    "ci.yml: shared main ref concurrency can preempt required runtime validation"
)
assert "scripts/ci/classify_changes.py" in ci
assert "scripts/ci/classify_push_changes.py" in ci
assert "github.event.pull_request.base.sha || github.event.before || ''" in ci
assert "python tests/ci/test_push_change_routing.py" in ci

acceptance = (WORKFLOW_ROOT / "acceptance-validation.yml").read_text(encoding="utf-8")
acceptance_trigger = trigger_prefix(acceptance)
assert event_block(acceptance_trigger, "pull_request") is not None
assert event_block(acceptance_trigger, "push") is not None
assert event_block(acceptance_trigger, "workflow_dispatch") is not None
assert event_block(acceptance_trigger, "workflow_call") is not None
assert has_path_filter(event_block(acceptance_trigger, "pull_request"))
assert has_path_filter(event_block(acceptance_trigger, "push"))
for product_path in ACCEPTANCE_PRODUCT_PATHS:
    assert product_path in event_block(acceptance_trigger, "pull_request"), (
        f"acceptance-validation.yml: pull_request missing product path {product_path}"
    )
    assert product_path in event_block(acceptance_trigger, "push"), (
        f"acceptance-validation.yml: push main missing product path {product_path}"
    )
assert (
    "group: acceptance-e2e-${{ github.workflow }}-${{ github.ref }}-${{ inputs.run_suffix || 'direct' }}"
    in acceptance
)
assert has_superseded_run_cancellation(acceptance)

portal_acceptance = (
    WORKFLOW_ROOT / "portal-acceptance-contract.yml"
).read_text(encoding="utf-8")
portal_trigger = trigger_prefix(portal_acceptance)
portal_pr = event_block(portal_trigger, "pull_request")
portal_push = event_block(portal_trigger, "push")
assert has_path_filter(portal_pr), "Portal Acceptance PR trigger must be path-scoped"
assert has_path_filter(portal_push), "Portal Acceptance main push must be path-scoped"
for product_path in PORTAL_ACCEPTANCE_PATHS:
    assert product_path in portal_pr, (
        f"portal-acceptance-contract.yml: pull_request missing product path {product_path}"
    )
    assert product_path in portal_push, (
        f"portal-acceptance-contract.yml: push main missing product path {product_path}"
    )

storage = (
    WORKFLOW_ROOT / "github-actions-storage-hygiene.yml"
).read_text(encoding="utf-8")
storage_trigger = trigger_prefix(storage)
assert event_block(storage_trigger, "pull_request") is not None
assert event_block(storage_trigger, "pull_request_target") is not None
assert "      - closed\n" in storage_trigger
assert event_block(storage_trigger, "schedule") is not None
assert "    - cron: '23 3,15 * * *'\n" in storage_trigger
assert event_block(storage_trigger, "workflow_dispatch") is not None
assert event_block(storage_trigger, "push") is None
assert "'docs/agents/tasks/**'" not in storage_trigger
assert "'build-synology-staging-images.yml'" not in storage_trigger
assert (
    "startsWith(github.event.head_commit.message, "
    "'chore(ci): clean GitHub Actions storage safely (#980)')"
    not in storage
)
assert (
    "group: actions-storage-hygiene-${{ github.event_name }}-${{ github.event.pull_request.number || github.ref }}"
    in storage
)
assert "cancel-in-progress: ${{ github.event_name == 'pull_request' }}" in storage
assert (
    "if: ${{ github.event_name == 'pull_request_target' && github.event.action == 'closed' }}"
    in storage
)
assert "if: ${{ github.event_name == 'schedule' || github.event_name == 'workflow_dispatch' }}" in storage
assert "ref: refs/heads/main" in storage
assert "persist-credentials: false" in storage
assert "actions: write" in storage
assert "--mode closed-pr" in storage
assert "--mode '${{ steps.mode.outputs.mode }}'" in storage
assert "CLEAN_ACTIONS_STORAGE" in storage
assert "if: ${{ always() }}" in storage

runpy.run_path(
    str(ROOT / "tests" / "ci" / "test_github_actions_storage_hygiene.py"),
    run_name="__main__",
)

print("workflow trigger economy contract: PASS")
