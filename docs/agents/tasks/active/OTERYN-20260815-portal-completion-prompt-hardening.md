---
task_id: OTERYN-20260815-portal-completion-prompt-hardening
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
search_first:
  - open Issues/PRs and branches owning OTERYN_PORTAL_COMPLETION prompt/governance paths
optional_reads:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
---

# OTERYN-20260815-portal-completion-prompt-hardening

## Goal

Harden the canonical `OTERYN_PORTAL_COMPLETION` execution prompt so it is deterministic, context-efficient and fully aligned with current prompting, anti-stall, validation and closeout governance without weakening repository or authority boundaries.

## Acceptance criteria

- [x] Delegate terminal reporting to the canonical anti-stall response instead of a weaker local duplicate.
- [x] Separate normal fresh independent validation routing from one-owner remediation self-review.
- [x] Use ordered selector short-circuiting only after every sibling in the current mixed entry is classified.
- [x] Preserve mandatory startup reads and move selected-slice context to just-in-time retrieval.
- [x] Preserve standard `execution_mode: chat` for coordination and resolve selected-slice mode only after selection.
- [x] Align autonomous wording with the at-most-one-additional-task anti-stall limit.
- [x] Put focused portal-v1.2 regression cases in canonical `docs/agents/evals/prompt-contract-v1.json`, which required CI executes.
- [x] Preserve connector-first, Platform-only, non-production, non-protected, non-payment, non-external-repository and no-standing-Codex boundaries.
- [x] Record runtime/browser E2E as `NOT_APPLICABLE` with the docs-only reason.
- [x] Repair and resolve both material P2 eval-integration review findings with exact-head required-CI evidence.
- [x] Synchronize the task branch with protected `main` without rewriting history or modifying unrelated archive content.
- [x] Repair the invalid checkpoint validation enum exposed by the synchronized checks.
- [x] Prove the checkpoint-fixed head with exact-head Agent Governance and CI.
- [ ] Complete the final task-record-only checkpoint generation and squash-merge PR #1076.
- [ ] Archive this task, verify source-branch cleanup, close/reconcile Issue #1075 and release ownership.

## Ownership

```yaml
project_lane: oteryn-platform-core
owned_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-completion-prompt-hardening.md
modules:
  - agent-governance
  - portal-completion-programme
dependencies:
  - Issue #1075
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T07:24:00Z
head: LIVE_PR_HEAD
branch: docs/issue-1075-portal-completion-prompt-hardening
pr: 1076
status: validating
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-completion-prompt-hardening.md
proven:
  - Issue #1075 and PR #1076 own this bounded task.
  - Prompt v1.2 keeps `OTERYN_PORTAL_COMPLETION.md` as the sole selector authority and short-circuits only after complete sibling classification in the current mixed entry.
  - Prompt v1.2 preserves standard execution-mode routing, connector-first GitHub routing, full vertical-slice semantics and all production/external/owner-funded-AI authority denials.
  - Runtime/browser E2E is NOT_APPLICABLE because this task changes only agent-governance Markdown/JSON.
  - Focused portal-v1.2 regression cases are integrated into schema-valid canonical `docs/agents/evals/prompt-contract-v1.json`, which both required workflows execute.
  - Repair head ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5 passed Agent Governance 31871134618 and CI 31871134611 after both P2 eval-integration findings were repaired.
  - Review threads PRRT_kwDOTcsYjs6Ze3O1 and PRRT_kwDOTcsYjs6Ze3O3 are resolved.
  - Protected main 3c3499f38100ec15ba76f958558444c87d644c15 was merged normally into this branch as d09fde29c5d8a9764e3e3dc6bb3f5a3a5712afda, preserving unrelated archive bytes without history rewrite.
  - The synchronized d09fde29c5d8a9764e3e3dc6bb3f5a3a5712afda failure was isolated to unsupported checkpoint validation result `PARTIAL`; deterministic prompt-contract steps passed first.
  - Checkpoint-fixed head afde56613db97ac1735d53aec64167e5c1c9f014 passed Agent Governance 31871662218 and CI 31871662221 on the exact unchanged head.
  - The delta from d09fde29c5d8a9764e3e3dc6bb3f5a3a5712afda to afde56613db97ac1735d53aec64167e5c1c9f014 modifies only this active task record.
  - Protected main later advanced independently to 860033172c8b4f1ba21d8d79263f04e2f0a49928 through PR #1081, modifying only `docs/agents/tasks/archive/OTERYN-20260814-public-today-architecture.md`, which does not overlap this task's three owned paths.
derived:
  - no further prompt or canonical-eval edit is warranted because the exact prompt/eval bytes already passed required deterministic validation and the latest main advancement is non-overlapping
unknown:
  - required checks and mergeability on the resulting task-record-only checkpoint head
conflicts: []
first_failure:
  marker: none
  evidence: all material prompt/eval/review/checkpoint defects identified during this task are repaired; remaining work is final checkpoint generation and merge closeout
rejected_hypotheses:
  - maintain a separate unregistered portal eval suite instead of using the canonical required suite
  - weaken or bypass prompt, checkpoint or source-branch validation to obtain green CI
  - change GOVERNANCE_CONTRACT.json to permit unsupported `PARTIAL` instead of correcting the task record
  - resynchronize solely because current main advanced through a non-overlapping archive-only closeout repair
changed_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-completion-prompt-hardening.md
validation:
  - command: canonical deterministic prompt contract after review repair
    result: PASS
    evidence: Agent Governance 31871134618 and CI 31871134611 on ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5
  - command: review repair exact-head self-review
    result: PASS
    evidence: PR #1076 comment 5301080752 and both resolved P2 review threads
  - command: synchronized-head checkpoint diagnosis
    result: PASS
    evidence: CI 31871442184 and Agent Governance 31871442254 both passed deterministic prompt-contract validation before exposing unsupported checkpoint result `PARTIAL`
  - command: checkpoint-fixed exact-head required CI
    result: PASS
    evidence: Agent Governance 31871662218 and CI 31871662221 both succeeded on afde56613db97ac1735d53aec64167e5c1c9f014
  - command: exact-head task-only delta review
    result: PASS
    evidence: compare d09fde29c5d8a9764e3e3dc6bb3f5a3a5712afda...afde56613db97ac1735d53aec64167e5c1c9f014 shows only this active task record changed
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: governance Markdown/JSON only; no executable browser or runtime journey exists for this task
  - command: final task-record-only checkpoint generation
    result: NOT_RUN
    evidence: this commit creates the final pre-merge checkpoint head and must pass repository-selected exact-head checks before merge
blockers:
  - none
next_action: verify required checks, review hygiene, current main/base and mergeability on LIVE_PR_HEAD; if green and non-overlapping, squash-merge PR #1076 with the expected exact head without another pre-merge branch commit
```

## Review findings

```yaml
findings:
  - id: PRRT_kwDOTcsYjs6Ze3O1
    severity: P2
    status: resolved
    repair: focused portal cases moved into schema-valid canonical prompt-contract-v1.json
    evidence: Agent Governance 31871134618 PASS; CI 31871134611 PASS
  - id: PRRT_kwDOTcsYjs6Ze3O3
    severity: P2
    status: resolved
    repair: prompt uses the canonical default suite already executed by both required workflows
    evidence: Agent Governance 31871134618 PASS; CI 31871134611 PASS
```

## Anti-stall state

```yaml
invocation_started_at: 2026-08-15T06:52:51Z
last_progress_at: 2026-08-15T07:24:00Z
ci_checks_for_current_head: 0
ci_check_generation: ready
terminal_ci_wait_started_at: 2026-08-15T07:24:00Z
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
context_reconstruction_attempts: 0
stall_warnings: 0
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 6
  session_id: portal-prompt-hardening-20260815T065251Z
  session_started_at: 2026-08-15T06:52:51Z
  checkpointed_at: 2026-08-15T07:24:00Z
  last_progress_at: 2026-08-15T07:24:00Z
  phase: final_ci
  exact_head: LIVE_PR_HEAD
  pull_request: 1076
  active_operation: bounded terminal CI wait for final task-record-only checkpoint head
  external_run_ids: []
  operation_started_at: 2026-08-15T07:24:00Z
  wait_deadline_at: 2026-08-15T07:45:40Z
  check_generation: ready
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: PR #1076 current exact head has terminal required checks and clean review/current-base/merge gates
  next_action: Observe one aggregate required-check snapshot for PR #1076 current exact head; if pending, wait at least three minutes before the next unchanged snapshot; if green, reverify review hygiene/current main/mergeability and squash-merge with expected head.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: this dedicated documentation/governance task branch has no continuing ownership or recovery purpose after PR #1076 merges and required archive closeout is persisted
source_branch_evidence: repository governance uses delete_branch_on_merge for merged same-repository task branches; final closeout must verify the source ref is absent after merge or reconcile it through Branch Lifecycle
```

## Notes

No Codex, OpenAI API or other owner-funded AI service was invoked. The deterministic prompt evaluator is repository tooling and does not execute an LLM.
