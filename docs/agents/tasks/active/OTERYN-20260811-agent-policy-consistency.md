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

Tracks Issue #991 and PR #992.

## Acceptance criteria

- [x] Deterministic validator covers current trusted governance and contradictory scope declarations.
- [x] Focused negative fixtures prove task-status, budget, repository-scope and closeout drift are rejected.
- [x] Agent Governance CI executes the focused tests and live validator.
- [x] Governance-only changes, including directory-based PR templates, route without unrelated heavy runtime gates by design.
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
updated_at: 2026-08-12T13:41:14+02:00
head: 77ee0c2e4c75ca65045b99e772c8679b3b82083f
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
  - PR #992 remains limited to the six declared changed paths and is based on current main without known ownership conflict.
  - Earlier review generations produced fail-closed coverage for repository grant grammar, authorization scoping and polarity, Markdown/fence parsing, duplicate status/budget declarations, completion weakening and governance-only routing.
  - Material repair b027ffe1ff9d78173e01f9e31f9f0cc4287426bb binds authorization to the actual repository/write target, compares the canonical repository case-insensitively, and rejects temporal/even-if completion weakening; e62bb1aa470d80fb78791ebcb5e1b70548f0d6f8 checkpointed those repairs.
  - Fresh review on e62bb1aa470d80fb78791ebcb5e1b70548f0d6f8 identified one still-material parser gap: repository/file/branch creation was absent from mutation grammar; the other target-binding and completion findings were already covered by b027ffe1 regressions.
  - Material head aded29c619b24c9f98f21d9febbc07f457447519 extends mutation grammar to create/creation and branch forms and adds regressions for create-files, create-branches and direct branch grants while retaining slash-prose suppression.
  - Agent Governance run 31586901594 passed on material head aded29c619b24c9f98f21d9febbc07f457447519. Job 94082713486 ran 92 of 92 policy-consistency regressions successfully, then passed the live validator, checkpoint validation, live task liveness and Control Room.
  - Review threads for target-binding, completion weakening and create/branch mutation coverage were resolved only after implementation or exact test evidence existed.
  - Fresh review on checkpoint head 9597211528d20f4edb8ca58c88cc22335c5cbfbf identified two remaining P1 variants: generic write permission for another repository and completion when exact-final-head CI has not passed or fails.
  - Repair d2170c7ffc7c55a6e8b2873d516da947c61668db binds generic write permission to the detected target and rejects negative exact-head CI completion predicates; regression head 5d89a8783809610713aca96200cd136153e1d0e6 adds all three adversarial examples.
  - Local focused validation on 5d89a8783809610713aca96200cd136153e1d0e6 passed 95 of 95 policy-consistency regressions, the live validator, 12 of 12 CI routing tests, all 3 active checkpoint validations and git diff checks.
  - Final checkpoint candidate bb9422e9a4d012e434c47152034d629e36c9d276 passed all 13 required checks and fresh full-diff review with zero material P1/P2, but branch protection rejected merge after main advanced.
  - Merge head 77ee0c2e4c75ca65045b99e772c8679b3b82083f incorporates protected main 66443e1d4bec220d45e74ead9fbc445d0abbca6e without conflicts; focused validation still passes 95/95 policy regressions, live validator, 12/12 routing tests and 3 checkpoint validations.
derived:
  - All currently known material review findings have implementation repairs and focused regression coverage on 5d89a8783809610713aca96200cd136153e1d0e6.
  - This task-record-only update creates a new metadata head; 77ee0c2e4c75ca65045b99e772c8679b3b82083f remains the latest main-synchronized material head.
unknown:
  - Terminal repository-required workflow results on the metadata-only final head created by this update.
  - Fresh Codex review result on the metadata-only final head created by this update.
conflicts: []
first_failure:
  marker: none-known-after-final-authorization-gate-regressions
  evidence: Local focused validation on 5d89a8783809610713aca96200cd136153e1d0e6 passed 95/95 policy regressions, live policy validation, 12/12 routing tests and checkpoint validation.
rejected_hypotheses:
  - Repository creation and branch creation are non-mutating operations; both modify repository state and must be covered by the foreign-repository boundary.
  - Any nearby owner approval is sufficient authorization for a foreign-repository mutation; authorization must bind to writing/mutating the actual target.
  - Presence of required closeout markers alone prevents policy weakening; contradictory authoritative completion declarations must fail closed.
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: python tools/agents/test_policy_consistency.py && python tools/agents/policy_consistency.py && python tests/ci/test_classify_changes.py && python tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint on 5d89a8783809610713aca96200cd136153e1d0e6
    result: PASS
    evidence: 95/95 policy-consistency regressions, live validator, 12/12 routing tests and 3 active checkpoint validations passed locally.
  - command: Agent Governance run 31586901594 on material head aded29c619b24c9f98f21d9febbc07f457447519
    result: PASS
    evidence: job 94082713486 completed 92/92 policy-consistency regressions, live policy validator, checkpoint validation, live task liveness and Control Room successfully.
  - command: final metadata-head repository-required CI and fresh Codex review
    result: NOT_RUN
    evidence: this checkpoint-only update creates a new final validation generation after mandatory main synchronization.
blockers:
  - none
next_action: Freeze the branch, validate every repository-required workflow and fresh Codex review on the resulting exact metadata head, resolve the procedural checkpoint thread with final-head evidence, then squash-merge PR #992 with expected-head protection and perform lifecycle archival before closing Issue #991.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 28
  session_id: codex-20260812-main-synchronized-final-gate
  session_started_at: 2026-08-12T13:05:00+02:00
  checkpointed_at: 2026-08-12T13:41:14+02:00
  last_progress_at: 2026-08-12T13:41:14+02:00
  phase: final-checkpoint-validation
  exact_head: 77ee0c2e4c75ca65045b99e772c8679b3b82083f
  pull_request: 992
  active_operation: checkpoint mandatory main synchronization after branch protection rejected stale-base merge
  external_run_ids: []
  operation_started_at: 2026-08-12T13:41:14+02:00
  wait_deadline_at: 2026-08-12T14:26:14+02:00
  check_generation: main-synchronized-final-checkpoint
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: final checkpoint head required checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect exact-head required CI and fresh review; merge only with zero unresolved material findings
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` because this task changes repository governance, validation and routing only. Executable outcome proof is the focused Python regression suite, live policy validator, routing tests and repository-required exact-head checks. Generic workflow and classifier changes remain fail closed with all applicable gates.
