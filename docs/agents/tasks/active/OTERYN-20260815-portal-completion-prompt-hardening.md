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

- [x] Replace the prompt-local shortened terminal response with delegation to the canonical anti-stall terminal response.
- [x] Separate normal fresh independent validation routing from the one-owner remediation self-review model.
- [x] Make selector traversal ordered and short-circuit after the first canonical `READY` candidate while still classifying all siblings in the current mixed entry.
- [x] Preserve mandatory startup reads and move selected-slice context to just-in-time retrieval.
- [x] Separate coordinator execution mode from selected-slice execution mode while preserving the standard `execution_mode` field.
- [x] Reconcile entry-slice wording with the anti-stall allowance for at most one additional task.
- [x] Put focused v1.2 regression cases in the repository's canonical deterministic eval suite so required CI actually executes them.
- [x] Preserve connector-first routing and false-GitHub-blocker protections.
- [x] Preserve Platform-only, non-production, non-protected, non-payment, non-external-repository and no-standing-Codex authority.
- [x] Record runtime/browser E2E as `NOT_APPLICABLE` with the concrete docs-only reason.
- [x] Verify the repaired exact branch diff and canonical deterministic eval on repair head `ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5`.
- [x] Resolve both material P2 review findings after exact-head required CI proved the repair.
- [ ] Complete final required GitHub CI and exact-head task-only delta self-review on the checkpoint head.
- [ ] Merge/close Issue #1075 and archive this task when all gates pass.

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
updated_at: 2026-08-15T07:08:16Z
head: LIVE_PR_HEAD
branch: docs/issue-1075-portal-completion-prompt-hardening
pr: 1076
status: validating
phase: final_ci
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
  - Issue #1075 and PR #1076 own this bounded prompt-hardening task
  - prompt v1.2 keeps OTERYN_PORTAL_COMPLETION.md as the sole selector authority and short-circuits only after all siblings in the current mixed entry are classified
  - prompt v1.2 preserves the standard execution_mode field and resolves selected-slice execution mode only after canonical selection
  - prompt v1.2 preserves connector-first, Platform-only, production/protected/payment/external-repository and owner-funded-AI boundaries
  - runtime/browser E2E is NOT_APPLICABLE because only agent-governance text/JSON changes
  - review found that the first separate focused eval was schema-incompatible and absent from required CI despite a green unrelated default suite
  - coherent repair removed the separate suite, added focused portal-v1.2 cases to canonical docs/agents/evals/prompt-contract-v1.json and pointed the prompt contract at that required suite
  - Agent Governance run 31871134618 and CI run 31871134611 passed on repaired exact head ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5
  - required workflows invoke tools/validation/prompt_eval.py with default docs/agents/evals/prompt-contract-v1.json, so those PASS results execute the new portal-v1.2 cases
  - both P2 review threads PRRT_kwDOTcsYjs6Ze3O1 and PRRT_kwDOTcsYjs6Ze3O3 were resolved only after that exact-head evidence passed
  - protected main advanced independently to 1f15865a285beefd34eb779eea89c48ef9f0c8d7 via PR #1078, changing only docs/agents/tasks/archive/OTERYN-20260731-portal-backend-frontend-audit.md
  - PR #1076 remains mergeable and its three changed paths do not overlap the intervening main change
derived:
  - no rebase is required for content conflict resolution; final merge gate must still re-read current base/head and any current-base check generation
  - this checkpoint-only commit changes task evidence only; prompt and canonical eval bytes remain identical to the repaired head already proven by required CI
unknown:
  - required GitHub CI result and review hygiene on the resulting final checkpoint head
conflicts: []
first_failure:
  marker: none
  evidence: both prior P2 findings repaired and resolved with exact-head CI evidence
rejected_hypotheses:
  - keep the separate focused suite and rely on its manual PASS claim
  - change prompt_eval.py/workflows solely to support a second suite when the canonical required suite can own the focused cases directly
  - weaken or resolve review findings without executable deterministic coverage
  - treat unrelated main archive-only advancement as overlapping prompt/eval ownership
changed_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-completion-prompt-hardening.md
validation:
  - command: canonical deterministic prompt contract on ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5
    result: PASS
    evidence: Agent Governance run 31871134618 and CI run 31871134611 both execute tools/validation/prompt_eval.py with canonical prompt-contract-v1.json
  - command: repair exact-head self-review on ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5
    result: PASS
    evidence: PR #1076 comment id 5301080752 records repaired suite integration and zero remaining material finding from the two P2 items
  - command: review hygiene after repair
    result: PASS
    evidence: review threads PRRT_kwDOTcsYjs6Ze3O1 and PRRT_kwDOTcsYjs6Ze3O3 resolved after passing evidence
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: governance Markdown/JSON only; no executable browser/runtime journey
  - command: final required CI on LIVE_PR_HEAD
    result: NOT_RUN
    evidence: this checkpoint-only commit intentionally creates the final pre-merge candidate head; no further pre-merge branch commit is planned
blockers:
  - none
next_action: observe required checks on LIVE_PR_HEAD under the bounded terminal-CI contract; if green, verify only task-record delta since ba8487e, review hygiene, current main/base and mergeability, then squash-merge PR #1076 with expected exact head without another branch commit
```

## Review findings

```yaml
findings:
  - id: PRRT_kwDOTcsYjs6Ze3O1
    severity: P2
    status: resolved
    summary: separate focused eval suite used an unsupported schema and could not pass prompt_eval.py
    repair: portal-v1.2 cases moved into schema-valid canonical docs/agents/evals/prompt-contract-v1.json
    evidence: Agent Governance 31871134618 PASS; CI 31871134611 PASS on ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5
  - id: PRRT_kwDOTcsYjs6Ze3O3
    severity: P2
    status: resolved
    summary: required CI did not invoke the separate focused suite
    repair: prompt now uses canonical default suite already executed by both required workflows; redundant suite removed
    evidence: Agent Governance 31871134618 PASS; CI 31871134611 PASS on ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5
```

## Anti-stall state

```yaml
invocation_started_at: 2026-08-15T06:52:51Z
last_progress_at: 2026-08-15T07:08:16Z
ci_checks_for_current_head: 0
ci_check_generation: ready
terminal_ci_wait_started_at: 2026-08-15T07:08:16Z
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 0
stall_warnings: 0
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 3
  session_id: portal-prompt-hardening-20260815T065251Z
  session_started_at: 2026-08-15T06:52:51Z
  checkpointed_at: 2026-08-15T07:08:16Z
  last_progress_at: 2026-08-15T07:08:16Z
  phase: final_ci
  exact_head: LIVE_PR_HEAD
  pull_request: 1076
  active_operation: bounded terminal CI wait for final task-only checkpoint head
  external_run_ids: []
  operation_started_at: 2026-08-15T07:08:16Z
  wait_deadline_at: 2026-08-15T07:45:40Z
  check_generation: ready
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: required checks on PR #1076 current exact head are terminal and merge/review/current-base gates can be re-evaluated
  next_action: Observe one aggregate required-check snapshot for PR #1076 current exact head; if pending, wait at least three minutes before the next unchanged snapshot; if green, verify the task-only delta, review hygiene, current main/base and mergeability, then squash-merge with expected exact head.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active and awaiting final exact-head checks/merge
source_branch_evidence: pending
```

## Notes

No Codex, OpenAI API or other owner-funded AI service was invoked. The deterministic prompt evaluator is repository tooling and does not execute an LLM.
