---
task_id: OTERYN-20260807-main-push-ci-routing
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
search_first:
  - issue #783 and related open PRs
  - overlapping active tasks and repair branches
optional_reads: []
---

# OTERYN-20260807-main-push-ci-routing

## Goal

Repair Issue #783 so documentation/governance-only pushes to `main` do not run or preempt runtime-heavy CI/Acceptance work, while product-affecting and ambiguous pushes remain fail-closed and required pull-request/manual behavior is preserved.

## Acceptance criteria

- [ ] Main-push CI classifies the exact pushed base/head range instead of unconditionally forcing `--all`.
- [ ] Missing, zero, empty or unusable push ranges fail closed to all heavy CI gates.
- [ ] Documentation/governance-only pushes to `main` do not emit full Acceptance E2E runs.
- [ ] Existing product-path Acceptance triggers, pull-request routing and manual `workflow_dispatch` remain intact.
- [ ] Deterministic tests cover docs-only/product main routing, zero/ambiguous range fallback, Acceptance path filtering and concurrency/preemption safety.
- [ ] Exact-head self-review, focused validation and repository-required CI pass with zero unresolved material findings.

## Ownership

```yaml
owned_paths:
  - .github/workflows/ci.yml
  - .github/workflows/acceptance-validation.yml
  - scripts/ci/classify_push_changes.py
  - tests/ci/test_push_change_routing.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20260807-main-push-ci-routing.md
modules:
  - ci-build-test
  - architecture-governance
dependencies:
  - Issue #783
  - docs/agents/BUILD_TEST_MATRIX.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T09:25:00Z
head: 3fd8c7acec864777e4876a36eaf8d2ef648cee47
branch: repair/issue-783
pr: 786
status: validating
context_routes:
  - ci-repair
  - testing
owned_paths:
  - .github/workflows/ci.yml
  - .github/workflows/acceptance-validation.yml
  - scripts/ci/classify_push_changes.py
  - tests/ci/test_push_change_routing.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20260807-main-push-ci-routing.md
proven:
  - Issue #783 is implementation-authorized and PR #786 is the active repair delivery.
  - CI push routing uses the GitHub push before/head range through a dedicated fail-closed adapter while pull-request routing remains on the canonical classifier.
  - Acceptance push routing mirrors its existing product-path pull-request filter.
  - PR #786 is maintained as one repair commit over the current main base.
  - The compact checkpoint correction passes checkpoint schema validation and live ownership evaluation for this task.
derived:
  - Filtering Acceptance at the push trigger prevents documentation-only main pushes from creating a run that could enter the shared main concurrency generation.
  - Reusing the canonical path classifier through a push adapter keeps the path policy single-sourced while adding push-range failure handling.
unknown: []
conflicts: []
first_failure:
  marker: Agent Governance run 31165291464 failed active task checkpoint validation.
  evidence: The original task checkpoint contained unsupported nested validation_gate metadata; replacing it with the version-1 compact contract made active checkpoint validation pass on the next exact head.
rejected_hypotheses:
  - Keep unconditional main --all and only tune concurrency; documentation-only pushes would still allocate heavy CI.
  - Add only an in-job Acceptance guard; the workflow would still be created and could preempt an in-progress main run before the guard executes.
changed_paths:
  - .github/workflows/ci.yml
  - .github/workflows/acceptance-validation.yml
  - scripts/ci/classify_push_changes.py
  - tests/ci/test_push_change_routing.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20260807-main-push-ci-routing.md
validation:
  - command: exact base/head compare review
    result: PASS
    evidence: The repair remains one commit and is limited to the six owned paths.
  - command: GitHub workflow parse/load
    result: PASS
    evidence: GitHub created exact-head CI, Acceptance and governed validation workflow runs for the changed workflow definitions.
  - command: Agent Governance checkpoint validation on 3fd8c7acec864777e4876a36eaf8d2ef648cee47
    result: PASS
    evidence: Checkpoint schema validation and live active-task ownership both completed successfully for the corrected checkpoint generation.
  - command: Agent Governance full job on 3fd8c7acec864777e4876a36eaf8d2ef648cee47
    result: FAIL
    evidence: An inherited terminal active task from merged audit PR #790 caused live-state enforcement failure; its owner merged closeout PR #791 into main as fed4ea954cd34d8b1b3a06a3de6f7307f619c747.
  - command: repository-required exact-head checks after rebasing onto closeout main
    result: NOT_RUN
    evidence: The rebased exact head has not been created yet.
blockers: []
next_action: Rebase the single repair commit onto main at fed4ea954cd34d8b1b3a06a3de6f7307f619c747, then require Agent Governance and CI to pass on the new exact head before evaluating the remaining required workflows.
```

## Notes

Issue #784 recorded the finding and its audit task has already been archived on `main`; it does not own or implement these repair paths.
