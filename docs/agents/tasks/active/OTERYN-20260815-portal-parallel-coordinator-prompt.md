---
task_id: OTERYN-20260815-portal-parallel-coordinator-prompt
project_lane: oteryn-platform-core
implementation_authorized: true
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - Issue #1082
  - PR #1076 portal completion prompt hardening ownership
  - PR #1061 federated-search reverse-edge work
  - PR #1073 client distribution Platform work
optional_reads: []
---

# OTERYN-20260815-portal-parallel-coordinator-prompt

## Goal

Add a standalone repository-owned prompt and dedicated evaluation inventory for a parallel `OTERYN_PORTAL_COMPLETION` coordinator/auditor/integrator without modifying the canonical prompt or shared eval surfaces currently owned by PR #1076.

## Acceptance criteria

- [x] The prompt allows parallel execution only for genuinely independent tasks with separate Issues/tasks/branches/PRs and non-overlapping `owned_paths`.
- [x] Existing live task/PR ownership is reused rather than duplicated.
- [x] Worker handoff requires a durable candidate-ready state and stops branch writes before coordinator audit/takeover.
- [x] Coordinator audit distrusts worker summaries, verifies exact-head outcome, remediates or returns findings, and integrates in dependency-safe order.
- [x] Canonical selector ordering remains authoritative and is rerun at synchronization barriers.
- [x] External/server repositories, production/protected environments, credentials, signing, payments and owner-funded AI remain unauthorized absent separate exact permission.
- [x] The new prompt has a versioned `PROMPT_EVAL_STANDARD.md` contract and dedicated balanced evaluation inventory with explicit no-model-trial limitation.
- [x] No file owned by PR #1076 is changed.
- [ ] Documentation/governance validation and exact-head required CI pass; runtime/browser E2E is `NOT_APPLICABLE` with reason.
- [ ] Issue/PR/task lifecycle is terminal and ownership is released after merge.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-parallel-coordinator-prompt.md
  - docs/agents/tasks/archive/OTERYN-20260815-portal-parallel-coordinator-prompt.md
modules:
  - agent-governance
  - portal-completion
dependencies:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
blockers:
  - none
cross_repository_tasks:
  - none
forbidden_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-15T07:18:30Z
head: ac757da25daadf652f06dc5afe16356a549bba7b
branch: docs/portal-parallel-coordinator-prompt
pr: 1083
status: validating
phase: validate
project_lane: oteryn-platform-core
session_id: chatgpt-20260815-portal-parallel-prompt
session_role: implementation_owner
execution_mode: chat_github
execution_reason: additive prompt/eval/task documentation with connector-first repository writes
context_routes:
  - agent-governance
  - portal-completion
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: focused
owned_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-parallel-coordinator-prompt.md
proven:
  - protected main at task start is 3c3499f38100ec15ba76f958558444c87d644c15
  - Issue #1082 owns this additive prompt package
  - draft PR #1083 owns branch docs/portal-parallel-coordinator-prompt
  - PR #1076 owns the canonical portal-completion prompt and shared prompt-contract eval; this task excludes both paths
  - repository compare from task-start main through ac757da25daadf652f06dc5afe16356a549bba7b changes exactly the standalone prompt, dedicated eval and this task record
  - no existing parallel-coordinator prompt was found by repository search
  - repository policy permits parallel workers only across independent tasks and forbids concurrent branch/worktree sharing
  - prompt requires canonical selector precedence, existing-work reuse, one branch writer, candidate handoff, dependency-safe merge and barrier reselection
  - dedicated eval contains positive, negative, boundary, authority, injection, validation-routing, integration and closeout cases
  - repeated model-behaviour trials are explicitly not claimed
derived:
  - a standalone prompt plus dedicated eval avoids ownership conflict with PR #1076
unknown:
  - exact final required CI outcome on the final task-checkpoint head
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - modifying the canonical execution prompt in parallel with PR #1076 is safe
  - several workers should be allowed to write one shared branch before integrator review
  - board OPEN or ARCHITECTURE_READY state alone is sufficient to dispatch a new parallel worker
changed_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-parallel-coordinator-prompt.md
validation:
  - command: repository overlap search against PR #1076 and existing parallel-coordinator prompt
    result: PASS
    evidence: owned paths are additive and distinct; no existing matching prompt found
  - command: compare task-start protected main 3c3499f38100ec15ba76f958558444c87d644c15 to ac757da25daadf652f06dc5afe16356a549bba7b
    result: PASS
    evidence: exactly three declared additive documentation/governance paths changed; forbidden #1076 paths are absent
  - command: dedicated evaluation inventory static review
    result: PASS
    evidence: balanced positive/negative/boundary/authority/injection/integration/closeout cases are present; model trials explicitly not claimed
  - command: full prompt specification review against PROMPTING_STANDARD, PROMPT_EVAL_STANDARD and EXECUTION_PROTOCOL
    result: PASS
    evidence: no identified authority, ownership, selector, audit-routing, E2E or closeout weakening
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: task changes agent-governance Markdown/JSON only and no executable product route, API, persistence or frontend behavior
blockers:
  - none
next_action: verify exact final branch head, mark PR ready when coherent, then require repository CI and governance checks before merge
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository documentation task branch
source_branch_evidence: pending merge and source-ref verification
```

## Notes

Issue: #1082. PR: #1083. The package is deliberately additive so it can proceed without touching PR #1076 ownership.
