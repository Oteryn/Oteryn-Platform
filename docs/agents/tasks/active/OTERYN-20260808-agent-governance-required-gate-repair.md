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

Repair Issue #858 so invalid active-task checkpoint state cannot remain nominally mergeable through the protected required CI gates, while preserving the blocked Native Character Portfolio architecture decision and changing no runtime/product/production behavior.

## Acceptance criteria

- [x] The current Native Character Portfolio active record satisfies checkpoint contract version 1 without accepting Option A/B/C.
- [x] A branch-protection-required CI context fails closed when active task checkpoints violate the canonical contract.
- [x] CI regression coverage proves the protected workflow retains that governance gate.
- [ ] Agent Governance and repository-required exact-head CI pass.
- [x] E2E is recorded as `NOT_APPLICABLE` with a concrete non-runtime reason.
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

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: implementation owner
  classified_at: 2026-08-07T22:55:20Z
  risk: high
  triggers:
    - CI merge enforcement
    - active-task governance
  unknown_or_conflict: []
  rationale: The repair changes a branch-protection-required CI path and governance validity enforcement, but no runtime/product/production behavior.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T22:55:20Z
head: e1989e49bdecd6d02dd41cb4b01b820fd2fb489c
branch: fix/OTERYN-20260808-agent-governance-required-gate
pr: 861
status: validating
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
  - Protected main requires classify-changes and test; classify-changes now validates all active task checkpoints with the canonical validator.
  - PR #859 remains draft and blocked on an explicit owner architecture decision; repair coordination was recorded on that PR without selecting an option.
derived:
  - The repaired required classifier prevents the same invalid checkpoint state from remaining nominally mergeable even if the separate path-filtered Agent Governance workflow is not itself a required status.
unknown: []
conflicts: []
first_failure:
  marker: active task checkpoint rejected by Agent Governance
  evidence: Issue #858; main Agent Governance run 31223532847 and PR #859 follow-up run 31224204311
rejected_hypotheses:
  - Normal CI being green is sufficient: branch protection did not previously validate active checkpoint structure and allowed the invalid task state to merge.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-agent-governance-required-gate-repair.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-portfolio-context.md
  - .github/workflows/ci.yml
  - tests/ci/test_required_test_gate.py
validation:
  - command: exact-head GitHub Actions on PR #861
    result: NOT_RUN
    evidence: final required checks are pending inspection on the current repair head
  - command: repair E2E
    result: NOT_APPLICABLE
    evidence: governance/task/CI-only repair creates no executable user or integration journey
blockers:
  - none
next_action: Inspect PR #861 exact-head Agent Governance and required CI results, then repair only evidence-backed failures before final self-review.
```

## Notes

No Laravel runtime, application data, authentication, payment, external repository, deployment or production state is changed. The architecture decision in PR #859 remains owner-blocked and Proposed.
