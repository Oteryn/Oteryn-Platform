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
- [ ] Review threads are resolved after exact repair inspection.
- [ ] Dedicated prompt-eval workflow plus required Agent Governance and CI pass on the exact final head; runtime/browser E2E remains `NOT_APPLICABLE` with reason.
- [ ] Issue/PR/task lifecycle is terminal and ownership is released after merge.

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
updated_at: 2026-08-15T07:27:30Z
head: aad5f7c87df010f865e59789c61d83a1095f2ab1
branch: docs/portal-parallel-coordinator-prompt
pr: 1083
status: implementing
phase: validate
project_lane: oteryn-platform-core
session_id: chatgpt-20260815-portal-parallel-prompt
session_role: implementation_owner
execution_mode: chat_github
execution_reason: review repair of additive prompt/eval/governance package
context_routes:
  - agent-governance
  - portal-completion
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: focused
invocation_started_at: 2026-08-15T07:15:00Z
last_progress_at: 2026-08-15T07:27:30Z
ci_checks_for_current_head: 0
ci_check_generation: ready
terminal_ci_wait_started_at: null
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
  - review thread PRRT_kwDOTcsYjs6Ze-4h identifies invalid ambiguity between candidate handoff and allowed checkpoint statuses
  - review thread PRRT_kwDOTcsYjs6Ze-4i proves the original dedicated eval is schema-invalid and not executed by existing required workflows
  - repository prompt evaluator schema requires id, deterministic_text_contract mode, exact policy keys, limitations, all eleven required categories, source/must_contain cases and at least three safety-critical cases
  - a new dedicated workflow can execute the corrected suite without modifying PR #1076 owned files or existing shared workflow semantics
derived:
  - the smallest safe repair is prompt handoff clarification plus schema-valid suite plus additive dedicated workflow
unknown:
  - exact-head validation result after the review repair
conflicts: []
first_failure:
  marker: pr-1083-review-contract-defects
  evidence: two unresolved P2 review threads on prompt handoff status and inert/schema-invalid dedicated eval
rejected_hypotheses:
  - an inert JSON inventory satisfies deterministic prompt regression acceptance
  - CANDIDATE_READY_FOR_AUDIT may be persisted as checkpoint status
  - modifying the PR #1076 canonical prompt/shared eval is necessary for this task
changed_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json
  - .github/workflows/parallel-coordinator-prompt-eval.yml
  - docs/agents/tasks/active/OTERYN-20260815-portal-parallel-coordinator-prompt.md
validation:
  - command: repository overlap review
    result: PASS
    evidence: canonical prompt/shared eval remain forbidden and unchanged; dedicated workflow is additive
  - command: prompt handoff repair review
    result: PASS
    evidence: checkpoint status is explicitly ready and candidate readiness is a separate handoff state
  - command: dedicated eval schema review against tools/validation/prompt_eval.py
    result: PASS
    evidence: corrected suite declares required schema/mode/policy/limitations/categories/source markers and three safety-critical cases
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: no executable product route, API, persistence or frontend behavior changes
blockers:
  - none
next_action: commit review repairs, inspect exact diff, resolve both review threads, then require dedicated prompt-eval plus repository-required exact-head CI before merge
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository agent-governance task branch
source_branch_evidence: pending merge and source-ref verification
```

## Notes

Issue: #1082. PR: #1083. The package remains additive and does not modify PR #1076 owned prompt/eval paths.
