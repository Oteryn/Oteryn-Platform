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
  - PR #859
optional_reads: []
---

# OTERYN-20260808-agent-governance-required-gate-repair

## Goal

Repair Issue #858 so invalid active-task checkpoint state cannot remain nominally mergeable through the protected `test` gate, while preserving the blocked Native Character Portfolio architecture decision and changing no runtime/product/production behavior.

## Acceptance criteria

- [ ] The current Native Character Portfolio active record satisfies checkpoint contract version 1 without accepting Option A/B/C.
- [ ] The protected `test` context fails closed when active task checkpoints violate the canonical contract.
- [ ] CI regression coverage proves the protected workflow retains that governance gate.
- [ ] Agent Governance and repository-required exact-head CI pass.
- [ ] E2E is recorded as `NOT_APPLICABLE` with a concrete non-runtime reason.
- [ ] Exact-head self-review reports zero material findings.
- [ ] Issue #858 is closed and this task is archived after merge.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-agent-governance-required-gate-repair.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-portfolio-context.md
  - .github/workflows/ci.yml
  - tests/ci/test_required_test_gate.py
modules:
  - agent-governance
  - ci-repair
dependencies:
  - issue #858
  - PR #859 retains architecture-decision ownership and must rebase after this repair
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T22:52:25Z
head: UNKNOWN
branch: fix/OTERYN-20260808-agent-governance-required-gate
pr: none
status: implementing
context_routes:
  - agent-governance
  - ci-repair
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-agent-governance-required-gate-repair.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-portfolio-context.md
  - .github/workflows/ci.yml
  - tests/ci/test_required_test_gate.py
proven:
  - Issue #858 records current main Agent Governance failure caused first by an invalid Native Character Portfolio active-task checkpoint.
  - Protected main currently requires only classify-changes and test.
  - PR #859 remains draft and blocked on an explicit owner architecture decision.
derived:
  - The minimal fail-closed repair is to restore the active checkpoint schema and make the already-required test context validate all active task checkpoints.
unknown: []
conflicts: []
first_failure:
  marker: active task checkpoint rejected by Agent Governance
  evidence: Issue #858; main Agent Governance run 31223532847 and PR #859 follow-up run 31224204311
rejected_hypotheses:
  - Normal CI being green is sufficient: branch protection does not currently require Agent Governance and allowed the invalid task state to merge.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-agent-governance-required-gate-repair.md
validation:
  - command: exact-head GitHub Actions
    result: NOT_RUN
    evidence: implementation is not yet complete
blockers:
  - none
next_action: Add the canonical checkpoint to the Native Character Portfolio task and enforce checkpoint validation inside the protected test context.
```

## Notes

Risk gate: `HEIGHTENED` because this changes CI/governance merge enforcement. It does not modify application runtime, data, authentication, payments, external repositories, deployment or production state.
