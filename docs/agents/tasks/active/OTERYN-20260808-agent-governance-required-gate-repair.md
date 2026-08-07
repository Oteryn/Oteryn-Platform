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
  - PR #861
optional_reads: []
---

# OTERYN-20260808-agent-governance-required-gate-repair

## Goal

Repair Issue #858 so invalid active-task governance cannot remain nominally mergeable through the protected required CI gates, and restore resulting-main Agent Governance after the now-merged Native Character Portfolio architecture task reaches terminal closeout.

## Acceptance criteria

- [x] The Native Character Portfolio task uses canonical checkpoint contract version 1 without re-opening the accepted Option A decision.
- [x] The now-terminal Native Character Portfolio task is moved from active state to archive state.
- [x] A branch-protection-required CI context fails closed when active task checkpoints violate the canonical contract.
- [x] CI regression coverage proves the protected workflow retains that governance gate.
- [ ] Agent Governance and repository-required exact-head CI pass on PR #861.
- [x] E2E is recorded as `NOT_APPLICABLE` with a concrete non-runtime reason.
- [ ] Exact-head self-review reports zero material findings.
- [ ] Issue #858 is closed and this repair task is archived after merge.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-agent-governance-required-gate-repair.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-portfolio-context.md
  - docs/agents/tasks/archive/OTERYN-20260808-native-character-portfolio-context.md
  - .github/workflows/ci.yml
  - tests/ci/test_required_test_gate.py
modules:
  - agent-governance
  - ci-repair
dependencies:
  - issue #858
  - PR #859 merged as 73c2426b37cfd5028fe9fbcec8254cc8aab3bc80
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
  classified_at: 2026-08-07T22:59:53Z
  risk: high
  triggers:
    - CI merge enforcement
    - active-task governance
    - terminal task closeout
  unknown_or_conflict: []
  rationale: The repair changes a branch-protection-required CI path and governance lifecycle state, but no runtime/product/production behavior.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Baseline drift disposition

During repair execution, `main` advanced from `b3788c3414b716743baa0500903b02f2e64cca7f` to `73c2426b37cfd5028fe9fbcec8254cc8aab3bc80` because PR #859 was owner-approved and merged. That merge already repaired the malformed Native Character Portfolio checkpoint and accepted Option A. This repair therefore preserves the new canonical architecture state, archives the now-terminal task, and retains only the independent merge-gate/regression fix from #858.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T22:59:53Z
head: 4a2bb58d13d850c332db5b508e6d22dc0434d792
branch: fix/OTERYN-20260808-agent-governance-required-gate
pr: 861
status: validating
context_routes:
  - agent-governance
  - ci-repair
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-agent-governance-required-gate-repair.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-portfolio-context.md
  - docs/agents/tasks/archive/OTERYN-20260808-native-character-portfolio-context.md
  - .github/workflows/ci.yml
  - tests/ci/test_required_test_gate.py
proven:
  - PR #859 merged as main commit 73c2426b37cfd5028fe9fbcec8254cc8aab3bc80 after exact-head Agent Governance and CI passed and Issue #857 was closed.
  - Main Agent Governance run 31225419202 fails only because the merged PR #859 task remains active with a stale merge next action and no explicit terminal archive transition.
  - Protected main requires classify-changes and test; this repair makes classify-changes validate all active task checkpoints with the canonical validator.
  - tests/ci/test_required_test_gate.py now asserts that the required classifier retains active checkpoint validation.
derived:
  - The repair must be rebased onto current main, preserve the accepted architecture, archive the terminal PR #859 task, and keep the independent required-CI checkpoint gate.
unknown: []
conflicts: []
first_failure:
  marker: terminal_pr_stale_next_action on resulting main
  evidence: Agent Governance run 31225419202 reports terminal_pr_stale_next_action and terminal_pr_active_task for PR #859.
rejected_hypotheses:
  - The repair should overwrite the Native Character Portfolio architecture checkpoint: PR #859 already supplied the canonical accepted Option A state on current main.
  - Normal CI being green is sufficient: branch protection did not previously validate active checkpoint structure and allowed the #858 failure mode to merge.
changed_paths:
  - .github/workflows/ci.yml
  - docs/agents/tasks/active/OTERYN-20260808-agent-governance-required-gate-repair.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-portfolio-context.md
  - docs/agents/tasks/archive/OTERYN-20260808-native-character-portfolio-context.md
  - tests/ci/test_required_test_gate.py
validation:
  - command: PR #859 exact-head Agent Governance and required CI
    result: PASS
    evidence: PR #859 validation records exact head b3e08b2251a755baddacfe709504227b8534dfb5 with Agent Governance and CI PASS.
  - command: resulting-main Agent Governance run 31225419202
    result: FAIL
    evidence: only terminal task liveness remains invalid after PR #859 merge; structural checkpoint validation passes.
  - command: PR #861 exact-head GitHub Actions
    result: NOT_RUN
    evidence: current repair branch must first reconcile baseline drift to main 73c2426b37cfd5028fe9fbcec8254cc8aab3bc80.
  - command: repair E2E
    result: NOT_APPLICABLE
    evidence: governance/task/CI-only repair creates no executable user or integration journey.
blockers:
  - none
next_action: Reconcile PR #861 onto current main with the terminal architecture task archived, then validate exact-head Agent Governance and required CI before self-review.
```

## Notes

No Laravel runtime, application data, authentication, payment, external repository, deployment or production state is changed. ADR 0030 remains Accepted and Option A is not re-opened.
