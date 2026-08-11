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
updated_at: 2026-08-11T18:59:00+02:00
head: 767b220335d7b7f07c670521dce2777c7d6e3d38
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
  - Protected main ab43c4b47173e7208d34851c4091f79051379f7a includes terminal Actions-major archive PR #1005 and releases its workflow ownership.
  - Implementation commit 767b220335d7b7f07c670521dce2777c7d6e3d38 has exactly one parent, protected main ab43c4b47173e7208d34851c4091f79051379f7a.
  - Compare main...767b2203 is behind_by=0 and changes exactly the six declared task-owned paths.
  - Repository-scope validation rejects unconditional foreign write grants whether repository identifiers are backticked or plain text, does not treat negated read-only wording as an exemption, and only accepts explicit user authorization scoped to the current task/write task.
  - Budget validation checks all matching declarations and rejects contradictory duplicate values.
  - Pull-request-template directory routing remains governance-only and regression-covered.
  - Focused regressions cover the fresh unquoted-repository, negated-read-only, duplicate-budget, explicit-current-task authorization and prior scope/status/closeout findings.
derived:
  - All currently known material review findings have implementation repairs; final merge readiness depends on unchanged-head CI and fresh review.
unknown:
  - Terminal result of the exact-head required checks generated from this metadata-only checkpoint revision.
  - Fresh Codex review result for the final branch head.
conflicts: []
first_failure:
  marker: historical-codex-policy-consistency-review-findings
  evidence: prior Codex reviews exposed repository-scope, template-routing and duplicate-budget fail-closed gaps; all are repaired in the current implementation and covered by regressions.
rejected_hypotheses:
  - A foreign grant is safe merely because it contains explicit or read-only wording; effective authorization must remain conditional on explicit user authorization for the current task.
  - Checking only the first matching budget declaration is fail closed; conflicting duplicate declarations must also be rejected.
  - Prior exact-head evidence remains sufficient after implementation changes; a fresh exact-head generation is required.
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: current-main clean restack comparison
    result: PASS
    evidence: ab43c4b47173e7208d34851c4091f79051379f7a...767b220335d7b7f07c670521dce2777c7d6e3d38 is behind_by=0, ahead_by=1 and exactly six task-owned files.
  - command: final exact-head required checks and fresh review
    result: NOT_RUN
    evidence: this metadata-only checkpoint revision creates the final validation generation; no implementation write should follow unless a gate or fresh review proves a defect.
blockers:
  - none
next_action: Validate the unchanged final branch head with repository-required CI and fresh Codex review; resolve all repaired review threads, squash-merge PR #992 with expected-head protection, verify resulting main, close Issue #991 and archive this task.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 11
  session_id: agent-20260811-final-closeout
  session_started_at: 2026-08-11T18:59:00+02:00
  checkpointed_at: 2026-08-11T18:59:00+02:00
  last_progress_at: 2026-08-11T18:59:00+02:00
  phase: final-exact-head-validation
  exact_head: 767b220335d7b7f07c670521dce2777c7d6e3d38
  pull_request: 992
  active_operation: validate clean current-main policy-consistency candidate
  external_run_ids: []
  operation_started_at: 2026-08-11T18:59:00+02:00
  wait_deadline_at: 2026-08-11T19:44:00+02:00
  check_generation: clean-restack-final
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: exact-head checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect final validation and review, then merge only if every gate is green
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` with executable governance and routing tests as the outcome proof. Generic workflow and classifier changes remain fail closed with all heavy gates.
