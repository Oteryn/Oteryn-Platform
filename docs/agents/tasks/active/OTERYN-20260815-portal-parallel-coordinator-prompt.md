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
  - PR #1083
optional_reads: []
---

# OTERYN-20260815-portal-parallel-coordinator-prompt

## Goal

Add a standalone repository-owned prompt and executable dedicated evaluation for a parallel `OTERYN_PORTAL_COMPLETION` coordinator/auditor/integrator without modifying the canonical prompt or shared eval surfaces owned by the now-terminal PR #1076 task.

## Acceptance criteria

- [x] Parallel execution is allowed only for genuinely independent tasks with separate Issues/tasks/branches/PRs and non-overlapping `owned_paths`.
- [x] Existing live task/PR ownership is reused rather than duplicated.
- [x] Worker handoff uses checkpoint `status: ready`, a separate `CANDIDATE_READY_FOR_AUDIT` handoff state, and stops branch writes before coordinator audit/takeover.
- [x] Coordinator audit distrusts worker summaries, verifies exact-head outcome, remediates or returns findings, and integrates in dependency-safe order.
- [x] Canonical selector ordering remains authoritative and is rerun at synchronization barriers.
- [x] External/server repositories, production/protected environments, credentials, signing, payments and owner-funded AI remain unauthorized absent separate exact permission.
- [x] The prompt has a versioned `PROMPT_EVAL_STANDARD.md` contract and a dedicated schema-valid balanced evaluation inventory with explicit no-model-trial limitation.
- [x] The dedicated suite is wired to a bounded workflow that executes the repository evaluator whenever the prompt/suite/workflow changes.
- [x] The task does not author the canonical prompt/shared-eval changes; those bytes are inherited only from protected `main` after PR #1076/#1084 completed.
- [x] Both material PR review findings are repaired and both review threads are resolved.
- [x] Repaired candidate head `e088630e5d5b9ce31300705e534da9fb35b37a27` and checkpoint head `f6e67a6ff6e4b5c3e32b1f1f9cb180ddbc9b194f` passed dedicated prompt eval, Agent Governance, repository CI and additionally triggered validation workflows; runtime/browser E2E is `NOT_APPLICABLE` with reason.
- [ ] Final synchronized head after protected-main advancement passes exact-head merge gates.
- [ ] Issue/PR/task lifecycle is terminal and ownership is released after merge and archive closeout.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json
  - .github/workflows/parallel-coordinator-prompt-eval.yml
  - docs/agents/tasks/active/OTERYN-20260815-portal-parallel-coordinator-prompt.md
  - docs/agents/tasks/archive/OTERYN-20260815-portal-parallel-coordinator-prompt.md
modules:
  - agent-governance
  - portal-completion
dependencies:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - tools/validation/prompt_eval.py
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
updated_at: 2026-08-15T07:41:00Z
head: f6e67a6ff6e4b5c3e32b1f1f9cb180ddbc9b194f
branch: docs/portal-parallel-coordinator-prompt
pr: 1083
status: ready
phase: close
project_lane: oteryn-platform-core
session_id: chatgpt-20260815-portal-parallel-prompt
session_role: implementation_owner
execution_mode: chat_github
execution_reason: final protected-main synchronization and merge closeout
context_routes:
  - agent-governance
  - portal-completion
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: full
invocation_started_at: 2026-08-15T07:15:00Z
last_progress_at: 2026-08-15T07:41:00Z
ci_checks_for_current_head: 0
ci_check_generation: current_main_sync
terminal_ci_wait_started_at: 2026-08-15T07:41:00Z
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 0
stall_warnings: 0
owned_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json
  - .github/workflows/parallel-coordinator-prompt-eval.yml
  - docs/agents/tasks/active/OTERYN-20260815-portal-parallel-coordinator-prompt.md
proven:
  - Issue #1082 owns this additive package
  - PR #1083 is Ready for Review and has zero unresolved review threads
  - the two initial P2 review findings are repaired: valid checkpoint handoff status and executable schema-valid dedicated eval
  - Parallel Coordinator Prompt Eval, Agent Governance, CI, Phase 7, Platform DB Outage, Game Auth Ticket Concurrency and Edge Security Emulation all passed on checkpoint head f6e67a6ff6e4b5c3e32b1f1f9cb180ddbc9b194f
  - required CI jobs `classify-changes` and `test` both passed on f6e67a6ff6e4b5c3e32b1f1f9cb180ddbc9b194f
  - protected main advanced independently to 88b3dce7b822ed27d9f61c412493c57ba6608a38 through terminal PR #1076/#1084 prompt-hardening closeout
  - main advancement changes only the canonical prompt/shared eval and archived #1075 task, which this task does not own; final task diff remains additive after synchronization
derived:
  - a final merge-from-main synchronization is required before implementation merge because branch protection requires current-base checks
unknown:
  - exact required-check outcome on the synchronized final head created by this merge-from-main checkpoint
conflicts: []
first_failure:
  marker: protected-main-advanced-before-merge
  evidence: merge API reported required checks expected after main advanced from 860033172c8b4f1ba21d8d79263f04e2f0a49928 to 88b3dce7b822ed27d9f61c412493c57ba6608a38
rejected_hypotheses:
  - an inert JSON inventory satisfies deterministic prompt regression acceptance
  - CANDIDATE_READY_FOR_AUDIT may be persisted as checkpoint status
  - force/bypass merge is acceptable when main advances
changed_paths:
  - .github/workflows/parallel-coordinator-prompt-eval.yml
  - docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/tasks/active/OTERYN-20260815-portal-parallel-coordinator-prompt.md
validation:
  - command: exact changed-file inventory
    result: PASS
    evidence: four declared task-owned paths relative to synchronized protected main
  - command: dedicated prompt eval / Agent Governance / CI on f6e67a6ff6e4b5c3e32b1f1f9cb180ddbc9b194f
    result: PASS
    evidence: runs 31872210343 / 31872210336 / 31872210352
  - command: additional triggered validation workflows on f6e67a6ff6e4b5c3e32b1f1f9cb180ddbc9b194f
    result: PASS
    evidence: Phase 7 31872210346, Platform DB Outage 31872210345, Game Auth Ticket Concurrency 31872210340, Edge Security Emulation 31872210344
  - command: PR #1083 review hygiene
    result: PASS
    evidence: both material review threads resolved; no unresolved thread remains
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: no executable product route, API, persistence or frontend behavior changes
blockers:
  - none
next_action: synchronize protected main 88b3dce7b822ed27d9f61c412493c57ba6608a38 into PR #1083, verify exact synchronized diff and final required checks, then squash-merge if eligible
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository agent-governance task branch
source_branch_evidence: repository metadata proves delete_branch_on_merge=true; exact source-ref absence must be verified after implementation merge
```

## Notes

Issue: #1082. PR: #1083. The task remains additive; canonical prompt/shared-eval bytes entering the branch come only from synchronized protected `main`, not from task-owned edits.
