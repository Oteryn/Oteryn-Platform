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
updated_at: 2026-08-11T19:20:00+02:00
head: 6e9bf6a67f0907f45f127f3063bdb0dc63b2d37d
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
  - Clean restack onto protected main remains bounded to exactly the six declared task-owned paths and carries no stale lifecycle history.
  - Prior final-generation Agent Governance 31516504584, CI 31516504583, Phase 7 31516504600, Platform DB Outage 31516504602, Edge Security 31516504585 and Game Auth Ticket Concurrency 31516504607 passed before the fresh review repair.
  - Fresh Codex review on f3e9cc27252719244a67e03ed894ce9f40edcc82 exposed four additional fail-closed variants: asserted rather than conditional authorization, contracted read-only negation, grants outside the allowlist section, and Markdown-formatted conflicting budget values.
  - tools/agents/policy_consistency.py now requires only-when/unless conditional authorization, recognizes contracted read-only negation, scans the whole authoritative root policy for foreign write grants, and normalizes inline Markdown before numeric budget comparison.
  - tools/agents/test_policy_consistency.py adds focused regressions for all four fresh variants while retaining the prior positive current-task authorization case.
  - Pull-request-template directory routing remains governance-only and regression-covered.
derived:
  - All currently known material review findings now have direct implementation repairs and focused regression coverage; merge readiness requires a fresh unchanged-head generation.
unknown:
  - Terminal result of required checks generated from this final review-repair checkpoint revision.
  - Fresh Codex review result for the final branch head.
conflicts: []
first_failure:
  marker: codex-final-fail-closed-variants
  evidence: Codex review on f3e9cc27252719244a67e03ed894ce9f40edcc82 reported four material variants that remained accepted; implementation commit 6e9bf6a67f0907f45f127f3063bdb0dc63b2d37d repairs each and adds regressions.
rejected_hypotheses:
  - Merely stating that the user explicitly authorizes a foreign repository is a valid exception; the policy must make the grant conditional on user authorization with only-when/unless semantics.
  - Checking only not/never/no-longer covers read-only negation; common contractions such as isn't must also fail closed.
  - Conflicting grants matter only inside the allowlist section; the entire authoritative root policy can affect later agents and must be scanned.
  - Raw-digit-only budget patterns are sufficient; Markdown emphasis can otherwise hide contradictory duplicate values.
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: prior final-generation exact-head checks on f3e9cc27252719244a67e03ed894ce9f40edcc82
    result: PASS
    evidence: Agent Governance, CI, Phase 7, Platform DB Outage, Edge Security and Game Auth Ticket Concurrency passed; Deep System was still running when fresh review required implementation changes.
  - command: final exact-head required checks and fresh review after four-variant repair
    result: NOT_RUN
    evidence: this metadata-only checkpoint revision creates the final validation generation; no implementation write should follow unless a gate or fresh review proves another defect.
blockers:
  - none
next_action: Validate the unchanged final branch head with repository-required CI and fresh Codex review; resolve repaired review threads, squash-merge PR #992 with expected-head protection, verify resulting main, close Issue #991 and archive this task.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 12
  session_id: agent-20260811-final-closeout
  session_started_at: 2026-08-11T18:59:00+02:00
  checkpointed_at: 2026-08-11T19:20:00+02:00
  last_progress_at: 2026-08-11T19:20:00+02:00
  phase: final-review-repair-validation
  exact_head: 6e9bf6a67f0907f45f127f3063bdb0dc63b2d37d
  pull_request: 992
  active_operation: validate final four-variant fail-closed repair
  external_run_ids: []
  operation_started_at: 2026-08-11T19:20:00+02:00
  wait_deadline_at: 2026-08-11T20:05:00+02:00
  check_generation: final-review-repair
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: exact-head checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect final validation and review, then merge only if every gate is green
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` with executable governance and routing tests as the outcome proof. Generic workflow and classifier changes remain fail closed with all heavy gates.
