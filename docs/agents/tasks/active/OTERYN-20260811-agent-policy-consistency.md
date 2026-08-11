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
updated_at: 2026-08-11T23:20:42+02:00
head: c4d9f12294400daf6c2bb426a8e2f37522c9c43f
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
  - Protected main ab43c4b47173e7208d34851c4091f79051379f7a is fully incorporated; compare remains behind_by=0 and exactly six declared task-owned paths.
  - Agent Governance run 31536712633 passed on 55c136e3e1bb4046870d3ed7b51dbe90ef27933f after the first six grammar-review repairs.
  - Fresh Codex review on 55c136e3e1bb4046870d3ed7b51dbe90ef27933f exposed three additional material parser variants: positive grants using unless, repositories preceding passive mutation verbs, and invalid Markdown fence closers with info strings.
  - Implementation head c4d9f12294400daf6c2bb426a8e2f37522c9c43f repairs all three variants and adds focused regressions for positive unless semantics, passive edit/modify grants and fence info-string closure.
  - Agent Governance run 31537301413 passed all policy-consistency tests, live validator, checkpoint validation and liveness on implementation head c4d9f12294400daf6c2bb426a8e2f37522c9c43f.
  - Earlier grammar findings covering quoted repository underscores, wrapped override declarations, negated authorization, negated grant verbs, dependent gerunds and fence-length semantics are regression-covered and their review threads are resolved.
derived:
  - Implementation and focused governance validation are complete; this checkpoint-only commit creates the final exact-head generation for repository-required checks and fresh review.
unknown:
  - Terminal result of repository-required checks on the checkpoint-only final head.
  - Fresh Codex review result for the checkpoint-only final head.
conflicts: []
first_failure:
  marker: codex-passive-and-fence-final-variants
  evidence: Fresh Codex review on 55c136e3e1bb4046870d3ed7b51dbe90ef27933f identified the last three known parser variants; c4d9f12294400daf6c2bb426a8e2f37522c9c43f repairs each with direct regression coverage.
rejected_hypotheses:
  - For a positive mutation grant, unless authorization is equivalent to only-when authorization; it is the opposite and must fail closed.
  - Repository association only needs mutation-before-token syntax; passive forms such as repo may be edited must also be recognized.
  - Any same-character fence token of sufficient length is a closer; a Markdown closer cannot carry an info-string suffix.
  - Slash-delimited prose should be globally excluded; real repository identifiers must instead be distinguished by mutation syntax and explicit quoting.
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: Agent Governance run 31537301413 on implementation head c4d9f12294400daf6c2bb426a8e2f37522c9c43f
    result: PASS
    evidence: Policy consistency tests, live validator, checkpoint validator, task liveness and Control Room all completed successfully.
  - command: exact-head full diff scope review
    result: PASS
    evidence: main..branch remains behind_by=0 and exactly six declared task-owned paths.
  - command: final checkpoint-head required checks and fresh Codex review
    result: NOT_RUN
    evidence: this task-record update creates the checkpoint-only final validation generation.
blockers:
  - none
next_action: Validate the unchanged checkpoint-only final head with repository-required CI and fresh Codex review; if green with zero unresolved material threads, squash-merge PR #992 with expected-head protection, verify main, close Issue #991 and archive this task through the repository-mandated lifecycle.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 18
  session_id: agent-20260811-final-closeout
  session_started_at: 2026-08-11T23:03:00+02:00
  checkpointed_at: 2026-08-11T23:20:42+02:00
  last_progress_at: 2026-08-11T23:20:42+02:00
  phase: checkpoint-only-final-validation
  exact_head: c4d9f12294400daf6c2bb426a8e2f37522c9c43f
  pull_request: 992
  active_operation: create final checkpoint generation after all known parser repairs
  external_run_ids:
    - 31537301413
  operation_started_at: 2026-08-11T23:20:42+02:00
  wait_deadline_at: 2026-08-12T00:05:42+02:00
  check_generation: final-checkpoint
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: exact-head checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect final validation and review, then squash-merge only if every gate is green
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` with executable governance and routing tests as the outcome proof. Generic workflow and classifier changes remain fail closed with all heavy gates.
