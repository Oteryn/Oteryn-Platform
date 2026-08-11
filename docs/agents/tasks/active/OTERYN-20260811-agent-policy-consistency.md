---
task_id: OTERYN-20260811-agent-policy-consistency
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
search_first:
  - agent governance consistency
  - open PR ownership for AGENTS and tools/agents
optional_reads: []
---

# OTERYN-20260811-agent-policy-consistency

## Goal

Fail closed when duplicated agent-governance status, execution-budget, repository-scope or completion rules drift across canonical Oteryn Platform policy documents.

Tracks Issue #991.

## Acceptance criteria

- [x] Deterministic validator covers current trusted governance and contradictory scope declarations.
- [x] Focused negative fixtures prove task-status, budget, repository-scope and closeout drift are rejected.
- [x] Agent Governance CI executes the focused tests and live validator.
- [x] Governance-only changes, including directory-based PR templates, route without unrelated heavy runtime gates.
- [ ] Exact-head required CI, Agent Governance and fresh review pass before merge.
- [ ] Issue #991 closes and this task archives after merge.

## Ownership

```yaml
owned_paths:
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
  - .github/workflows/agent-governance.yml
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - docs/agents/tasks/archive/OTERYN-20260811-agent-policy-consistency.md
modules:
  - agent-governance
  - testing
  - ci-routing
dependencies:
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T16:44:04Z
head: 58084c96884b775ec60fc1482aa0836638b09bb7
branch: test/agent-policy-consistency
pr: 992
status: validating
context_routes:
  - agent-governance
  - testing
  - ci-repair
owned_paths:
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
  - .github/workflows/agent-governance.yml
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - docs/agents/tasks/archive/OTERYN-20260811-agent-policy-consistency.md
proven:
  - Protected main advanced to a6943dca8622e43e0781786f49c93085cc1104df after PR #1001 squash-merged with exact-head CI and fresh Codex review clean.
  - Merge commit 58084c96884b775ec60fc1482aa0836638b09bb7 incorporates protected main a6943dca8622e43e0781786f49c93085cc1104df and implementation head 4b06796fe08caf1da45228da349b7ce6795a2aff without conflict.
  - Earlier exact head 27ffd7d6a9d95a1ca753899920207d4ad3074bdf passed all generated repository gates before fresh review raised one additional P1 about unscoped use of the word explicit.
  - tools/agents/policy_consistency.py now preserves the prior current-governance parser and accepts a foreign-repository write exception only when the line explicitly identifies the user, authorization, and current-task/write-task scope.
  - tools/agents/test_policy_consistency.py includes a negative regression for an unscoped explicitly-allowed foreign write grant and retains the positive current-task user-authorization case.
  - The directory-based .github/PULL_REQUEST_TEMPLATE/** governance-only classifier repair and regression remain present.
derived:
  - All currently known review findings have an implementation repair; merge readiness still requires a fresh unchanged-head generation.
unknown:
  - Terminal result of the final exact-head required checks after this checkpoint commit.
  - Fresh Codex review result for the final head.
conflicts: []
first_failure:
  marker: codex-review-current-task-authorization
  evidence: fresh review on 27ffd7d6a9d95a1ca753899920207d4ad3074bdf showed that a foreign write grant containing only the word explicit could bypass the contradiction detector.
rejected_hypotheses:
  - Any line containing explicit proves user authorization; the final parser requires explicit user authorization and current-task/write-task scope together.
  - The accidental reconstructed validator on 6409dd20 was compatible with current governance; Agent Governance 31512968713 disproved that, so the known-good parser baseline was restored before the focused P1 repair.
  - Prior green checks remain sufficient after the focused repair and restack; a new exact-head generation is required.
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: prior repository-required exact-head generation on 27ffd7d6a9d95a1ca753899920207d4ad3074bdf
    result: PASS
    evidence: Agent Governance, CI, Deep System, Phase 7, Platform DB Outage, Edge Security and Game Auth Ticket Concurrency all completed successfully.
  - command: final exact-head required checks after current-task authorization repair and main restack
    result: NOT_RUN
    evidence: this checkpoint creates the final validation generation and no implementation write should follow unless a gate or fresh review finds a defect.
blockers:
  - none
next_action: Require exact-head generated checks and fresh Codex review to pass on the unchanged final head; resolve repaired review threads, squash-merge PR #992, verify main, and perform terminal Issue #991/task archival closeout.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 10
  session_id: agent-20260811-final-closeout
  session_started_at: 2026-08-11T16:44:04Z
  checkpointed_at: 2026-08-11T16:44:04Z
  last_progress_at: 2026-08-11T16:44:04Z
  phase: final-exact-head-validation
  exact_head: 58084c96884b775ec60fc1482aa0836638b09bb7
  pull_request: 992
  active_operation: validate current-task authorization repair on refreshed main
  external_run_ids: []
  operation_started_at: 2026-08-11T16:44:04Z
  wait_deadline_at: 2026-08-11T17:29:04Z
  check_generation: current-task-authorization-final
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: exact-head checks and fresh review are terminal and head remains unchanged
  next_action: inspect final validation and review, then merge only if every gate is green
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` with executable governance and routing tests as the outcome proof. Generic workflow and classifier changes remain fail closed with all heavy gates.
