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
- [x] Synchronize the task branch with the current protected `main` without rewriting history or modifying unrelated archive content.
- [x] Repair the invalid checkpoint validation enum exposed by the synchronized final checks.
- [ ] Complete final required checks and exact-head task-only/base-sync delta self-review.
- [ ] Squash-merge PR #1076, close Issue #1075, archive this task and release branch/path ownership.

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
policy_version: 2
updated_at: 2026-08-15T07:17:00Z
head: LIVE_PR_HEAD
branch: docs/issue-1075-portal-completion-prompt-hardening
pr: 1076
status: validating
phase: final_ci_after_checkpoint_fix
execution_mode: chat
execution_reason: bounded agent-governance validation through GitHub connector; no owner-funded AI invocation required
project_lane: oteryn-platform-core
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
  - PR review found two real P2 defects: the first focused eval suite was schema-incompatible and was not executed by required CI.
  - Repair head ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5 removed the separate suite, moved focused portal cases into canonical schema-valid `prompt-contract-v1.json`, and pointed the prompt at that suite.
  - Agent Governance 31871134618 and CI 31871134611 passed on ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5; both workflows execute the canonical prompt evaluator, proving the repaired portal cases are in required validation.
  - Review threads PRRT_kwDOTcsYjs6Ze3O1 and PRRT_kwDOTcsYjs6Ze3O3 were resolved only after that evidence passed.
  - Final checkpoint head 18a5e536df9a849230a518ff2ff400667dbd7c06 had CI 31871206722 PASS and Agent Governance 31871206716 FAIL only at terminal source-branch closeout after all prompt/eval/checkpoint checks passed.
  - Protected main advanced independently and the unrelated archive closeout defect was repaired on main by PR #1079.
  - Current protected main 3c3499f38100ec15ba76f958558444c87d644c15 was merged normally into the task branch as d09fde29c5d8a9764e3e3dc6bb3f5a3a5712afda, preserving current-main archive bytes without history rewrite.
  - On d09fde29c5d8a9764e3e3dc6bb3f5a3a5712afda both deterministic prompt-contract steps passed in CI and Agent Governance.
  - CI 31871442184 failed at `Validate active task checkpoint contract` and Agent Governance 31871442254 failed at `Validate active task checkpoints` on d09fde29c5d8a9764e3e3dc6bb3f5a3a5712afda.
  - `docs/agents/GOVERNANCE_CONTRACT.json` permits validation results only PASS, FAIL, BLOCKED, NOT_RUN and NOT_APPLICABLE.
  - The task checkpoint on d09fde29c5d8a9764e3e3dc6bb3f5a3a5712afda contained unsupported validation result `PARTIAL`, which is the direct checkpoint-contract defect repaired by this commit.
derived:
  - no prompt or canonical eval change is required for the d09 checkpoint failure because their deterministic validation steps passed on that exact head
  - replacing unsupported `PARTIAL` with truthful `FAIL` is the smallest contract-correct checkpoint repair
unknown:
  - final required-check result and mergeability on the resulting checkpoint-fixed exact head
conflicts: []
first_failure:
  marker: CI 31871442184 `Validate active task checkpoint contract` / Agent Governance 31871442254 `Validate active task checkpoints`
  evidence: prompt-contract validation passed first; checkpoint.py rejects validation result values outside GOVERNANCE_CONTRACT.json, and the task contained unsupported `PARTIAL`
rejected_hypotheses:
  - prompt/eval regression caused the synchronized-head failures; both deterministic prompt-contract steps passed on d09fde29c5d8a9764e3e3dc6bb3f5a3a5712afda
  - weaken or bypass checkpoint validation
  - change the governance contract to permit `PARTIAL` instead of correcting this task record
  - perform another main synchronization or prompt edit without evidence that either is required
changed_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-completion-prompt-hardening.md
validation:
  - command: canonical deterministic prompt contract on ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5
    result: PASS
    evidence: Agent Governance 31871134618 and CI 31871134611
  - command: review repair exact-head self-review
    result: PASS
    evidence: PR #1076 comment 5301080752 and both resolved P2 threads
  - command: final checkpoint CI on 18a5e536df9a849230a518ff2ff400667dbd7c06
    result: FAIL
    evidence: CI 31871206722 passed but Agent Governance 31871206716 failed terminal source-branch closeout after all prompt/eval/checkpoint steps passed
  - command: synchronized-head deterministic prompt contracts on d09fde29c5d8a9764e3e3dc6bb3f5a3a5712afda
    result: PASS
    evidence: CI 31871442184 and Agent Governance 31871442254 both passed their deterministic prompt-contract steps before checkpoint validation
  - command: synchronized-head repository gate on d09fde29c5d8a9764e3e3dc6bb3f5a3a5712afda
    result: FAIL
    evidence: both required workflows failed checkpoint validation because the task record used unsupported result `PARTIAL`
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: governance Markdown/JSON only
  - command: final required CI on LIVE_PR_HEAD
    result: NOT_RUN
    evidence: this checkpoint-only repair creates the final pre-merge candidate head; no further branch commit is planned unless a new materially different failure is proven
blockers:
  - none
next_action: observe required checks on LIVE_PR_HEAD under the bounded terminal-CI contract; if green, verify only the task-record delta, review hygiene, current main/base and mergeability, then squash-merge PR #1076 with expected exact head
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
last_progress_at: 2026-08-15T07:17:00Z
ci_checks_for_current_head: 0
ci_check_generation: ready
terminal_ci_wait_started_at: 2026-08-15T07:17:00Z
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
  generation: 5
  session_id: portal-prompt-hardening-20260815T065251Z
  session_started_at: 2026-08-15T06:52:51Z
  checkpointed_at: 2026-08-15T07:17:00Z
  last_progress_at: 2026-08-15T07:17:00Z
  phase: final_ci_after_checkpoint_fix
  exact_head: LIVE_PR_HEAD
  pull_request: 1076
  active_operation: bounded terminal CI wait for final checkpoint-fixed head
  external_run_ids: []
  operation_started_at: 2026-08-15T07:17:00Z
  wait_deadline_at: 2026-08-15T07:45:40Z
  check_generation: ready
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: PR #1076 current exact head has terminal required checks and clean review/current-base/merge gates
  next_action: Observe one aggregate required-check snapshot for PR #1076 current exact head; if pending, wait at least three minutes before the next unchanged snapshot; if green, verify the task-only delta, review hygiene, current main/base and mergeability, then squash-merge with expected head.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: this dedicated documentation/governance task branch has no continuing ownership or recovery purpose after PR #1076 merges and required archive closeout is persisted
source_branch_evidence: repository governance uses delete_branch_on_merge for merged same-repository task branches; final closeout must verify the source ref is absent after merge or reconcile it through Branch Lifecycle
```

## Notes

No Codex, OpenAI API or other owner-funded AI service was invoked. The deterministic prompt evaluator is repository tooling and does not execute an LLM.
