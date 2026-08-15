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

- [ ] The prompt allows parallel execution only for genuinely independent tasks with separate Issues/tasks/branches/PRs and non-overlapping `owned_paths`.
- [ ] Existing live task/PR ownership is reused rather than duplicated.
- [ ] Worker handoff requires a durable candidate-ready state and stops branch writes before coordinator audit/takeover.
- [ ] Coordinator audit distrusts worker summaries, verifies exact-head outcome, remediates or returns findings, and integrates in dependency-safe order.
- [ ] Canonical selector ordering remains authoritative and is rerun at synchronization barriers.
- [ ] External/server repositories, production/protected environments, credentials, signing, payments and owner-funded AI remain unauthorized absent separate exact permission.
- [ ] The new prompt has a versioned `PROMPT_EVAL_STANDARD.md` contract and dedicated balanced evaluation inventory with explicit no-model-trial limitation.
- [ ] No file owned by PR #1076 is changed.
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
updated_at: 2026-08-15T07:16:30Z
head: UNKNOWN
branch: docs/portal-parallel-coordinator-prompt
pr: none
status: implementing
phase: implement
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
  - PR #1076 owns the canonical portal-completion prompt and shared prompt-contract eval; this task excludes both paths
  - no existing parallel-coordinator prompt was found by repository search
  - repository policy permits parallel workers only across independent tasks and forbids concurrent branch/worktree sharing
derived:
  - a standalone prompt plus dedicated eval avoids ownership conflict with PR #1076
unknown:
  - exact final prompt/eval validation outcome
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - modifying the canonical execution prompt in parallel with PR #1076 is safe
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260815-portal-parallel-coordinator-prompt.md
validation:
  - command: repository overlap search against PR #1076 and existing parallel-coordinator prompt
    result: PASS
    evidence: owned paths are additive and distinct; no existing matching prompt found
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: task changes agent-governance Markdown/JSON only and no executable product route, API, persistence or frontend behavior
blockers:
  - none
next_action: create the standalone prompt and dedicated evaluation inventory, then validate the exact branch diff
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository documentation task branch
source_branch_evidence: pending merge and source-ref verification
```

## Notes

Issue: #1082. The package is deliberately additive so it can proceed without touching PR #1076 ownership.
