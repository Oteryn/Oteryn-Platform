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
updated_at: 2026-08-11T23:42:00+02:00
head: 430f3386e0ddde43f0210971502ad65046988f03
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
  - Protected main ab43c4b47173e7208d34851c4091f79051379f7a is incorporated and the PR remains limited to the six declared task-owned paths.
  - Fresh Codex review on checkpoint head 5225ee858d26ea5ad1a930081fbdde013af41f81 exposed three additional fail-closed variants: denied/withheld authorization predicates, mutation objects between operation and repository, and status declarations nested inside an outer fenced example.
  - Implementation head 430f3386e0ddde43f0210971502ad65046988f03 requires an affirmative active/passive authorization predicate, associates common repository mutation object/preposition phrases, and extracts status/terminal declarations only outside outer fenced examples.
  - Focused regressions cover denied, withheld and not-granted authorization; edit-files-in and push-changes-to repository grants; and nested outer-fence declaration suppression.
  - Agent Governance run 31538306125 passed on implementation head 430f3386e0ddde43f0210971502ad65046988f03.
  - CI 31538306185, Phase 7 31538306132, Edge Security 31538306181, Platform DB Outage 31538306238 and Game Auth Ticket Concurrency 31538306158 passed on implementation head 430f3386e0ddde43f0210971502ad65046988f03; Deep System Validation 31538306217 was still running at checkpoint time.
  - The three fresh review threads from 5225ee858d26ea5ad1a930081fbdde013af41f81 are resolved after implementation and passing focused governance validation.
derived:
  - All currently known material review findings have implementation repairs and regression coverage; this task-record-only commit creates the final checkpoint generation.
unknown:
  - Terminal repository-required check result on the checkpoint-only final head created by this update.
  - Fresh Codex review result for the checkpoint-only final head created by this update.
conflicts: []
first_failure:
  marker: codex-affirmative-auth-object-association-and-outer-fence
  evidence: Fresh Codex review on 5225ee858d26ea5ad1a930081fbdde013af41f81 identified three material variants; implementation head 430f3386e0ddde43f0210971502ad65046988f03 repairs each and Agent Governance 31538306125 passes.
rejected_hypotheses:
  - Presence of permission/authorization vocabulary is enough to prove authorization; a valid exception requires an affirmative authorization predicate.
  - Repository association requires the owner/name token to nearly immediately follow a mutation verb; common mutation objects and prepositions must remain associated.
  - Status declarations can be regex-scanned after emphasis normalization without outer-fence state; non-normative nested fenced examples must be excluded.
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: Agent Governance run 31538306125 on implementation head 430f3386e0ddde43f0210971502ad65046988f03
    result: PASS
    evidence: focused policy-consistency regressions, live validator, checkpoint validation, liveness and Control Room completed successfully.
  - command: completed repository workflows on implementation head 430f3386e0ddde43f0210971502ad65046988f03
    result: PASS
    evidence: CI, Phase 7, Edge Security, Platform DB Outage and Game Auth Ticket Concurrency each completed successfully; Deep System Validation status is tracked separately in checkpoint evidence because no terminal result had been observed at checkpoint time.
  - command: final checkpoint-head required checks and fresh Codex review
    result: NOT_RUN
    evidence: this task-record-only update creates the final validation generation.
blockers:
  - none
next_action: Validate the unchanged checkpoint-only final head with repository-required CI and fresh Codex review; if every required check passes with zero unresolved material threads, squash-merge PR #992 with expected-head protection, verify main, close Issue #991 and archive this task through the repository-mandated lifecycle.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 19
  session_id: agent-20260811-2342-final-closeout
  session_started_at: 2026-08-11T23:42:00+02:00
  checkpointed_at: 2026-08-11T23:42:00+02:00
  last_progress_at: 2026-08-11T23:42:00+02:00
  phase: final-checkpoint-validation
  exact_head: 430f3386e0ddde43f0210971502ad65046988f03
  pull_request: 992
  active_operation: create final checkpoint generation after latest Codex-review repair
  external_run_ids:
    - 31538306125
    - 31538306185
    - 31538306132
    - 31538306181
    - 31538306238
    - 31538306158
    - 31538306217
  operation_started_at: 2026-08-11T23:42:00+02:00
  wait_deadline_at: 2026-08-12T00:27:00+02:00
  check_generation: final-checkpoint
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: exact-head checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect final exact-head CI and fresh review, then squash-merge only if every merge gate remains satisfied
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` because this task changes repository governance, validation and routing only. Executable outcome proof is the focused Python regression suite, live policy validator, routing tests and repository-required exact-head checks. Generic workflow and classifier changes remain fail closed with all applicable gates.
