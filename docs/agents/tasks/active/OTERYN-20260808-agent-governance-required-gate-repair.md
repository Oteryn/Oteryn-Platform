---
task_id: OTERYN-20260808-agent-governance-required-gate-repair
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
search_first:
  - issue #858
  - PR #861
optional_reads: []
---

# OTERYN-20260808-agent-governance-required-gate-repair

## Goal

Repair Issue #858 so invalid active-task checkpoint state cannot remain nominally mergeable through the protected required CI gates, without changing runtime, product, accepted architecture, deployment or production behavior.

## Acceptance criteria

- [x] The malformed Native Character Portfolio active-task state was repaired and the completed task was archived without re-opening accepted ADR 0030.
- [x] A branch-protection-required CI context fails closed when active task checkpoints violate the canonical contract.
- [x] CI regression coverage proves the protected workflow retains that governance gate.
- [x] Agent Governance and repository-required exact-head CI pass on PR #861.
- [x] E2E is recorded as `NOT_APPLICABLE` with a concrete non-runtime reason.
- [x] Exact-head full-diff self-review reports zero material findings.
- [ ] PR #861 has landed, resulting-main governance passes, Issue #858 is closed and this repair task is archived.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-agent-governance-required-gate-repair.md
  - .github/workflows/ci.yml
  - tests/ci/test_required_test_gate.py
modules:
  - agent-governance
  - ci-repair
dependencies:
  - issue #858
blockers:
  - none
cross_repository_tasks:
  - none
```

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: implementation owner
  classified_at: 2026-08-08T01:03:10+02:00
  risk: high
  triggers:
    - CI merge enforcement
    - active-task governance
  unknown_or_conflict: []
  rationale: The repair changes a branch-protection-required CI path and governance validity enforcement, but no runtime/product/production behavior.
  self_review:
    result: PASS_ZERO_MATERIAL_FINDINGS
    exact_head: a9a6a187fb6003645b3d9f4e102d76b321b6a049
    evidence:
      - PR #861 full-diff review submitted 2026-08-07T23:05:57Z; three bounded paths, zero material findings, zero review threads.
```

## Baseline reconciliation

PR #859 accepted ADR 0030 and PR #862 subsequently archived its completed task. PR #862 merged as `8571da7cb5748698352531565f47345c8d7e7672`. Current-main Agent Governance run `31225693878` passes. This repair therefore does not modify, recreate or own the Native Character Portfolio task or architecture programme state; it is bounded to the independent Issue #858 required-gate/regression fix.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T01:06:28+02:00
head: a9a6a187fb6003645b3d9f4e102d76b321b6a049
branch: fix/OTERYN-20260808-agent-governance-required-gate
pr: 861
status: ready
terminal_pr_policy: archive_pending
context_routes:
  - agent-governance
  - ci-repair
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-agent-governance-required-gate-repair.md
  - .github/workflows/ci.yml
  - tests/ci/test_required_test_gate.py
proven:
  - Issue #858 records the historical failure mode where an invalid active-task checkpoint reached protected main while normal required CI remained green.
  - PR #862 merged the completed Native Character Portfolio task archive as main commit 8571da7cb5748698352531565f47345c8d7e7672 and current-main Agent Governance run 31225693878 passes.
  - Protected main requires classify-changes and test.
  - PR #861 adds canonical active-task checkpoint validation inside required classify-changes.
  - tests/ci/test_required_test_gate.py asserts that the protected classifier retains the canonical checkpoint validation command.
  - PR #861 Agent Governance run 31225934401 passed on head a9a6a187fb6003645b3d9f4e102d76b321b6a049, including checkpoint, liveness and Control Room validation.
  - PR #861 CI run 31225933611 passed classify-changes, runtime-tests and required test on head a9a6a187fb6003645b3d9f4e102d76b321b6a049.
  - Exact-head full-diff self-review on a9a6a187fb6003645b3d9f4e102d76b321b6a049 reported PASS_ZERO_MATERIAL_FINDINGS with zero review threads.
derived:
  - The required classifier now fails closed on the structural checkpoint failure mode from Issue #858 even if the separate path-filtered Agent Governance workflow is not itself a required branch-protection context.
unknown: []
conflicts: []
first_failure:
  marker: active task checkpoint rejected by Agent Governance while required CI passed
  evidence: Issue #858; main Agent Governance run 31223532847 and corresponding successful normal CI recorded by the issue
rejected_hypotheses:
  - Normal CI being green was sufficient: branch protection previously omitted checkpoint validation and allowed the invalid task state to land.
  - PR #861 should keep Native Character Portfolio archive changes: PR #862 already completed that closeout on current main.
changed_paths:
  - .github/workflows/ci.yml
  - docs/agents/tasks/active/OTERYN-20260808-agent-governance-required-gate-repair.md
  - tests/ci/test_required_test_gate.py
validation:
  - command: Agent Governance run 31225934401 on PR #861 head a9a6a187fb6003645b3d9f4e102d76b321b6a049
    result: PASS
    evidence: checkpoint-validation completed successfully, including checkpoint contract, task liveness and Control Room validation.
  - command: CI run 31225933611 on PR #861 head a9a6a187fb6003645b3d9f4e102d76b321b6a049
    result: PASS
    evidence: classify-changes, runtime-tests and required test all completed successfully; the new active-task checkpoint step passed.
  - command: exact-head full-diff self-review on a9a6a187fb6003645b3d9f4e102d76b321b6a049
    result: PASS
    evidence: PASS_ZERO_MATERIAL_FINDINGS; exactly three bounded repair paths and zero review threads.
  - command: repair E2E
    result: NOT_APPLICABLE
    evidence: governance/task/CI-only repair creates no executable user or integration journey.
blockers:
  - none
next_action: After PR #861 lands, verify resulting-main Agent Governance and required CI, then archive this task record.
```

## Notes

No Laravel runtime, application data, authentication, payment, external repository, deployment, production state or accepted architecture decision is changed.
