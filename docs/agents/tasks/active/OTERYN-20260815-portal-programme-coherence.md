---
task_id: OTERYN-20260815-portal-programme-coherence
issue: 1092
status: implementing
project_lane: oteryn-platform-core
phase: implement
execution_mode: github_connector
task_kind: implementation
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
search_first:
  - Issue #1092
  - open PR #1086 CI orchestration ownership
  - terminal Issue #1072 and merged #1090 branch-hygiene authority
  - open Portal Completion PRs #1061 and #1073
optional_reads:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md
---

# OTERYN-20260815 portal programme coherence

## Goal

Make portal delivery authority, live selection, completion scope, execution allocation, architecture handoff, historical-governance routing and cross-repository authority unambiguous and deterministically regression-protected without changing runtime behavior.

## Acceptance criteria

- [ ] `ROADMAP -> PORTAL_COMPLETENESS_ARCHITECTURE -> PORTAL_COMPLETION_DELIVERY_PLAN -> OTERYN_PORTAL_COMPLETION -> WORK_ALLOCATION` responsibilities are explicitly separated without creating a second scheduler.
- [ ] `OTERYN_PORTAL_COMPLETION` remains the sole live selector and clarifies one selected slice per worker/invocation entry versus safe global parallelism across independently owned tasks.
- [ ] Portal P0 live-drift reconciliation excludes historical-ref retention/deletion, which remains repository-governance-owned under ADR 0037/0039 and steady-state branch hygiene.
- [ ] New durable architecture decisions route to `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`; Portal coordination only decomposes/applies already accepted architecture unless a new decision is explicitly routed first.
- [ ] Multi-world/ruleset/season handling is a conditional cross-cutting invariant with an activation trigger, not an orphaned independent queue item.
- [ ] Portal launch/completion disposition is machine-readable and explicitly non-scheduling/non-live-state.
- [ ] PlayerCompanion selector wording reflects terminal foundation plus individually promoted follow-up slices.
- [ ] Work Allocation is visibly a non-scheduling capability maturity/execution matrix.
- [ ] Historical Work Reconciliation prompt is terminal/tombstoned and cannot restart completed Issue #1072.
- [ ] Game Catalog programme cannot grant standing Canary/server-repository access from a Platform invocation.
- [ ] Deterministic prompt/policy regression cases protect the new invariants.
- [ ] No active PR/task owned path is modified without coordination; in particular #1086 CI orchestration and #1089/#1090 branch-hygiene paths remain untouched.
- [ ] Exact-head documentation/governance CI and self-review pass; runtime/browser E2E is `NOT_APPLICABLE` with a concrete reason.
- [ ] Issue/task/PR/source-branch closeout is terminal.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-portal-programme-coherence.md
  - docs/agents/tasks/archive/OTERYN-20260815-portal-programme-coherence.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json
  - docs/agents/programs/GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md
  - docs/agents/prompts/OTERYN-HISTORICAL-WORK-RECONCILIATION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
modules:
  - repository-governance
  - portal-completion-control-plane
  - prompt-contracts
dependencies:
  - Issue #1072 terminal historical reconciliation
  - Issue #1089 / merged PR #1090 steady-state branch hygiene
  - open PR #1086 owns CI/workflow-economy paths and must remain non-overlapping
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T09:10:00Z
head: pending-first-commit
branch: docs/portal-programme-coherence-20260815
pr: none
status: implementing
phase: implement
context_routes:
  - agent-governance
  - architecture
  - testing
owned_paths:
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json
  - docs/agents/programs/GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md
  - docs/agents/prompts/OTERYN-HISTORICAL-WORK-RECONCILIATION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
proven:
  - Protected main at task start is bd9110cb3f998f41241b01da295ef147d2dd428e.
  - Issue #1072 is closed completed; its historical-work task is archived and 37 reviewed historical refs are terminally absent.
  - PR #1090 merged steady-state branch hygiene into protected main; its active task remains in post-merge validation/closeout and owns only ADR 0039, historical-branch-audit workflow, branch_hygiene helper/tests and its task paths.
  - PR #1086 owns CI/workflow orchestration including ci.yml and BUILD_TEST_MATRIX; this task does not claim those paths.
  - Portal Completion already declares itself the live selector; Work Allocation declares itself non-scheduling.
  - Game Catalog programme contains legacy cross-repository permission wording that is subordinate to current Platform-only governance.
derived:
  - The smallest safe change is one governance/prompt-contract task with no runtime or workflow edits.
unknown:
  - Exact final head and required CI generation after the coherent documentation/prompt package is written.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - merge all portal documents into one monolith
  - create a second portal scheduler
  - create a second historical cleanup programme
  - weaken Platform-only external-repository authority to match legacy programme prose
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260815-portal-programme-coherence.md
validation:
  - command: live repository/ownership preflight
    result: PASS
    evidence: current main, active tasks, open PRs and relevant programme/authority documents were inspected before mutation
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: task changes only governance/documentation/prompt contracts and introduces no executable product/runtime journey
blockers: []
next_action: Reconcile the owned programme/authority/prompt files and extend deterministic prompt-contract cases without touching active CI or branch-hygiene ownership.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: github-20260815-portal-programme-coherence
  session_started_at: 2026-08-15T09:07:50Z
  checkpointed_at: 2026-08-15T09:10:00Z
  last_progress_at: 2026-08-15T09:10:00Z
  phase: implement
  exact_head: pending-first-commit
  pull_request: none
  active_operation: none
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: draft
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: Issue #1092 remains open and the authoritative task branch remains uniquely owned
  next_action: Apply the coherent portal-programme governance package and open the authoritative draft PR.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1092 is active on docs/portal-programme-coherence-20260815
source_branch_evidence: pending
```

## Temporary branch bookkeeping

Two extra no-change refs were created from the same task-start main during connector branch-name resolution: `docs/issue-1091-portal-programme-coherence` and `docs/portal-programme-coherence`. They have no independent scope or authority. They must be attached to explicit duplicate PR provenance and deleted through the repository branch-lifecycle mechanism before this task can be terminal.
