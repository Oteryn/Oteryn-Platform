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
  - PR #1083 review threads
optional_reads: []
---

# OTERYN-20260815-portal-parallel-coordinator-prompt

## Goal

Add a standalone repository-owned prompt and executable dedicated evaluation for a parallel `OTERYN_PORTAL_COMPLETION` coordinator/auditor/integrator without modifying the canonical prompt or shared eval surfaces owned by PR #1076.

## Acceptance criteria

- [x] Parallel execution is allowed only for genuinely independent tasks with separate Issues/tasks/branches/PRs and non-overlapping `owned_paths`.
- [x] Existing live task/PR ownership is reused rather than duplicated.
- [x] Worker handoff uses checkpoint `status: ready`, a separate `CANDIDATE_READY_FOR_AUDIT` handoff state, and stops branch writes before coordinator audit/takeover.
- [x] Coordinator audit distrusts worker summaries, verifies exact-head outcome, remediates or returns findings, and integrates in dependency-safe order.
- [x] Canonical selector ordering remains authoritative and is rerun at synchronization barriers.
- [x] External/server repositories, production/protected environments, credentials, signing, payments and owner-funded AI remain unauthorized absent separate exact permission.
- [x] The prompt has a versioned `PROMPT_EVAL_STANDARD.md` contract and a dedicated schema-valid balanced evaluation inventory with explicit no-model-trial limitation.
- [x] The dedicated suite is wired to a bounded workflow that executes the repository evaluator whenever the prompt/suite/workflow changes.
- [x] No canonical prompt/shared-eval file owned by PR #1076 is changed.
- [x] Both material PR review findings are repaired and both review threads are resolved.
- [x] Repaired candidate head `e088630e5d5b9ce31300705e534da9fb35b37a27` passed dedicated prompt eval, Agent Governance, repository CI and all additionally triggered validation workflows; runtime/browser E2E is `NOT_APPLICABLE` with reason. The checkpoint-only final head created by this update must still pass its exact-head merge gates.
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
updated_at: 2026-08-15T07:32:30Z
head: e088630e5d5b9ce31300705e534da9fb35b37a27
branch: docs/portal-parallel-coordinator-prompt
pr: 1083
status: ready
phase: close
project_lane: oteryn-platform-core
session_id: chatgpt-20260815-portal-parallel-prompt
session_role: implementation_owner
execution_mode: chat_github
execution_reason: additive prompt/eval/governance package at final merge gate
context_routes:
  - agent-governance
  - portal-completion
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: full
invocation_started_at: 2026-08-15T07:15:00Z
last_progress_at: 2026-08-15T07:32:30Z
ci_checks_for_current_head: 0
ci_check_generation: ready
terminal_ci_wait_started_at: 2026-08-15T07:32:30Z
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
  - protected main is synchronized through 860033172c8b4f1ba21d8d79263f04e2f0a49928
  - Issue #1082 owns this additive package and records the review-repair scope
  - PR #1083 is Ready for Review
  - PR #1076 owns the canonical portal-completion prompt and shared prompt-contract eval; this task excludes both paths
  - PR #1083 changed-file inventory contains only the dedicated workflow, standalone prompt, dedicated eval and this task record
  - review finding on invalid candidate checkpoint status was repaired by using checkpoint status ready plus separate HANDOFF_STATE
  - review finding on inert/schema-invalid eval was repaired with the repository evaluator schema plus dedicated workflow execution
  - both PR #1083 review threads are resolved on repaired head
  - dedicated Parallel Coordinator Prompt Eval run 31872049023 passed on repaired candidate head e088630e5d5b9ce31300705e534da9fb35b37a27
  - Agent Governance run 31872049094 passed on repaired candidate head e088630e5d5b9ce31300705e534da9fb35b37a27
  - CI run 31872048955 passed on repaired candidate head e088630e5d5b9ce31300705e534da9fb35b37a27
  - additionally triggered Phase 7, DB outage, game-auth concurrency and edge-emulation workflows all passed on repaired candidate head
  - runtime/browser E2E is not applicable because no executable product route, API, persistence or frontend behavior changes
derived:
  - the review-repaired package satisfies its prompt/eval behavior contract and is ready for final checkpoint-head CI
unknown:
  - exact required-check outcome on the checkpoint-only head created by this update
conflicts: []
first_failure:
  marker: pr-1083-review-contract-defects
  evidence: two P2 findings on initial candidate were repaired and review threads resolved
rejected_hypotheses:
  - an inert JSON inventory satisfies deterministic prompt regression acceptance
  - CANDIDATE_READY_FOR_AUDIT may be persisted as checkpoint status
  - modifying the PR #1076 canonical prompt/shared eval is necessary for this task
changed_paths:
  - .github/workflows/parallel-coordinator-prompt-eval.yml
  - docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/tasks/active/OTERYN-20260815-portal-parallel-coordinator-prompt.md
validation:
  - command: exact changed-file inventory
    result: PASS
    evidence: four declared task-owned paths only; canonical prompt/shared eval are absent
  - command: dedicated evaluation inventory schema and execution
    result: PASS
    evidence: Parallel Coordinator Prompt Eval run 31872049023 SUCCESS on e088630e5d5b9ce31300705e534da9fb35b37a27
  - command: Agent Governance run 31872049094
    result: PASS
    evidence: repaired candidate head e088630e5d5b9ce31300705e534da9fb35b37a27
  - command: CI run 31872048955
    result: PASS
    evidence: repaired candidate head e088630e5d5b9ce31300705e534da9fb35b37a27
  - command: additional triggered validation workflows
    result: PASS
    evidence: Phase 7 31872049007, Platform DB Outage 31872048980, Game Auth Ticket Concurrency 31872048928, Edge Security Emulation 31872048963 all SUCCESS
  - command: PR #1083 review hygiene
    result: PASS
    evidence: both material review threads repaired, replied to and resolved
  - command: exact-head whole-diff self-review
    result: PASS
    evidence: no open material authority, ownership, selector, checkpoint-status, eval-execution, E2E or closeout finding remains on repaired candidate
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: no executable product route, API, persistence or frontend behavior changes
blockers:
  - none
next_action: verify required checks on the checkpoint-only final PR head, then squash-merge PR #1083 if unchanged and eligible
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository agent-governance task branch
source_branch_evidence: pending implementation merge and source-ref verification during archive closeout
```

## Notes

Issue: #1082. PR: #1083. The package remains additive and does not modify PR #1076 owned prompt/eval paths.
