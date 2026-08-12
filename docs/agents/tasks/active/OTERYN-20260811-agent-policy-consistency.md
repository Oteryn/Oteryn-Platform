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
updated_at: 2026-08-12T09:53:00+02:00
head: 639e7afcb807d09f3df4b6d9a55bbf7786c6bd00
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
  - Earlier review generations drove fail-closed coverage for repository-scope grants, authorization scoping and polarity, Markdown wrapping/emphasis/fences, slash-shaped prose, active/passive/mandatory mutation grammar, duplicate declarations, nested YAML examples and checkpoint freshness.
  - Fresh Codex review on checkpoint head 5535311de640f91a6706a1b4583b63a6ed01e1bd exposed four material findings: write-access grants after conditional conjunctions were not split; delete/remove mutations were missing; full/unrestricted write-access modifiers bypassed positive authority; and do-not-allow / never-authorize / never-permit denials were false positives.
  - Repair commits 5f9e472aa44bba7c8185d07fff5f4044fbd5882b and 639e7afcb807d09f3df4b6d9a55bbf7786c6bd00 close those findings by extending mutation vocabulary, write-access modifiers, grant splitting, negative authorization forms and removal association through the from preposition.
  - Test head be7bec7d48ddb3d07a690fbd83a1112e35265548 added focused regressions for conditional write-access splitting, delete/remove, full/unrestricted write access, do-not-allow, never-authorize and never-permit; its first Agent Governance attempt correctly exposed one remaining remove-from association failure, which 639e7afcb807d09f3df4b6d9a55bbf7786c6bd00 repairs.
  - Agent Governance run 31575678582 passed on 639e7afcb807d09f3df4b6d9a55bbf7786c6bd00. Job 94047133313 executed 77 of 77 policy-consistency regressions successfully, then passed the live policy validator, 3 active checkpoint validations, live liveness for 3 active tasks with 0 advisory findings, and Control Room validation.
  - All four material review threads from 5535311de640f91a6706a1b4583b63a6ed01e1bd were replied to with exact-head regression evidence and resolved only after the 77-test proof.
derived:
  - All currently known material parser findings have implementation repairs and explicit focused regressions; this task-record-only refresh creates a new final checkpoint generation without changing parser/runtime behavior.
  - Recording 639e7afcb807d09f3df4b6d9a55bbf7786c6bd00 as the latest material implementation/test head avoids self-referential checkpoint SHA churn; live GitHub state remains authoritative for the actual branch head produced by this checkpoint commit.
unknown:
  - Terminal repository-required CI result on the checkpoint-only final head created by this update.
  - Fresh Codex review result for the checkpoint-only final head created by this update.
conflicts: []
first_failure:
  marker: codex-final-grant-grammar-generation
  evidence: Fresh Codex review on 5535311de640f91a6706a1b4583b63a6ed01e1bd identified four grant/denial bypasses. Initial repair head be7bec7d48ddb3d07a690fbd83a1112e35265548 passed 76/77 regressions and exposed only remove-files-from association; material head 639e7afcb807d09f3df4b6d9a55bbf7786c6bd00 repairs that final case and Agent Governance 31575678582 proves 77/77 focused regressions plus the live validator pass.
rejected_hypotheses:
  - Direct have/has write-access support is sufficient without adding the same grammar to conjunction splitting; independent RHS write-access grants must be split from prior conditional authorization.
  - Edit/push/commit/merge cover all meaningful repository mutation vocabulary; delete/remove are direct mutations and must fail closed on foreign repositories.
  - Only explicit can modify write-access wording; common full/unrestricted/direct/autonomous modifiers also represent affirmative authority.
  - Negating only grant is sufficient; allow/authorize/permit require corresponding do-not/never/not denial recognition to avoid false positives.
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: Agent Governance run 31575678582 on implementation/test head 639e7afcb807d09f3df4b6d9a55bbf7786c6bd00
    result: PASS
    evidence: job 94047133313 completed 77 of 77 policy-consistency regressions, live policy validation, 3 checkpoint validations, live liveness validation for 3 active tasks with 0 advisory findings and Control Room validation successfully.
  - command: first repair Agent Governance run 31575370050 on be7bec7d48ddb3d07a690fbd83a1112e35265548
    result: FAIL
    evidence: job 94046169494 ran 77 tests with exactly one failure, test_remove_foreign_repository_grant_fails_closed; the first actionable failure was the missing from preposition in mutation-to-repository association and is repaired on 639e7afcb807d09f3df4b6d9a55bbf7786c6bd00.
  - command: checkpoint-only final-head required CI, Agent Governance and fresh Codex review
    result: NOT_RUN
    evidence: this task-record-only update creates the final validation generation.
blockers:
  - none
next_action: Validate the checkpoint-only final branch head created by this update with repository-required CI, Agent Governance and a fresh Codex review; if every required check passes with zero unresolved material threads, squash-merge PR #992 with expected-head protection, verify main, close Issue #991 and archive this task through the repository-mandated lifecycle.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 23
  session_id: agent-20260812-0953-final-grant-grammar-checkpoint
  session_started_at: 2026-08-12T09:40:00+02:00
  checkpointed_at: 2026-08-12T09:53:00+02:00
  last_progress_at: 2026-08-12T09:53:00+02:00
  phase: final-checkpoint-validation
  exact_head: 639e7afcb807d09f3df4b6d9a55bbf7786c6bd00
  pull_request: 992
  active_operation: create refreshed final checkpoint after final grant-grammar repairs and 77-test Agent Governance proof
  external_run_ids:
    - 31575678582
    - 31575678567
    - 31575678595
    - 31575678600
    - 31575678639
    - 31575678841
    - 31575679149
  operation_started_at: 2026-08-12T09:53:00+02:00
  wait_deadline_at: 2026-08-12T10:38:00+02:00
  check_generation: final-grant-grammar-checkpoint
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: checkpoint-only exact-head required checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect final exact-head CI and fresh review, then squash-merge only if every merge gate remains satisfied
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` because this task changes repository governance, validation and routing only. Executable outcome proof is the focused Python regression suite, live policy validator, routing tests and repository-required exact-head checks. Generic workflow and classifier changes remain fail closed with all applicable gates.