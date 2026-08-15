---
task_id: OTERYN-20260815-portal-programme-coherence
issue: 1092
status: validating
project_lane: oteryn-platform-core
phase: validate
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
  - Issue #1092 and PR #1093
  - open PR #1086 CI orchestration ownership
  - terminal Issues #1072/#1089 and branch-hygiene authority
  - current Portal Completion PR/task ownership including #1061 and #1073
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

- [x] `ROADMAP -> PORTAL_COMPLETENESS_ARCHITECTURE -> PORTAL_COMPLETION_DELIVERY_PLAN -> OTERYN_PORTAL_COMPLETION -> WORK_ALLOCATION` responsibilities are explicitly separated without creating a second scheduler.
- [x] `OTERYN_PORTAL_COMPLETION` remains the sole live selector and clarifies one selected slice per worker/invocation entry versus safe global parallelism across independently owned tasks.
- [x] Portal P0 live-drift reconciliation excludes historical-ref retention/deletion, which remains repository-governance-owned under ADR 0037/0039 and steady-state branch hygiene.
- [x] New durable architecture decisions route to `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`; Portal coordination only decomposes/applies already accepted architecture unless a new decision is explicitly routed first.
- [x] Multi-world/ruleset/season handling is a conditional cross-cutting invariant with an activation trigger, not an orphaned independent queue item.
- [x] Portal launch/completion disposition is machine-readable and explicitly non-scheduling/non-live-state.
- [x] PlayerCompanion selector wording reflects terminal foundation plus individually promoted follow-up slices.
- [x] Work Allocation is visibly a non-scheduling capability maturity/execution matrix.
- [x] Historical Work Reconciliation prompt is terminal/tombstoned and cannot restart completed Issue #1072.
- [x] Game Catalog programme cannot grant standing Canary/server-repository access from a Platform invocation and preserves the existing atomic consumer-first merge ordering.
- [x] Deterministic prompt/policy regression cases encode the new invariants.
- [x] The completion-scope manifest has a semantic validator for JSON shape, allowed dispositions, uniqueness, non-scheduling rules, forbidden mutable live state and critical workstream invariants.
- [x] No active implementation-owned CI/workflow path from #1086 was modified. Existing `ci.yml` already executes `tools/validation/test_prompt_eval.py` and remains untouched.
- [x] Merged #1089/#1090/#1091 lifecycle state was incorporated without changing its implementation.
- [x] Accidental duplicate PRs/refs #1094/#1095 are terminally closed/absent and no unexplained duplicate branch remains.
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
  - tools/validation/test_prompt_eval.py
synchronization_paths:
  - docs/agents/tasks/active/OTERYN-20260815-steady-state-branch-hygiene.md
  - docs/agents/tasks/archive/OTERYN-20260815-steady-state-branch-hygiene.md
modules:
  - repository-governance
  - portal-completion-control-plane
  - prompt-contracts
dependencies:
  - Issue #1072 terminal historical reconciliation
  - Issue #1089 / merged PRs #1090 and #1091 steady-state branch hygiene
  - open PR #1086 owns CI/workflow-economy paths and remains non-overlapping
blockers: []
cross_repository_tasks: []
```

The synchronization paths reproduce the exact terminal #1091 active-to-archive state already present on protected `main@536c6320f91d1df981600530e50522d84b1c0588`. They are not Issue #1092 implementation ownership and produce no net tree difference from current main.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T09:34:00Z
head: LIVE_PR_1093_HEAD
branch: docs/portal-programme-coherence-20260815
pr: 1093
status: validating
phase: validate
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
  - tools/validation/test_prompt_eval.py
proven:
  - Protected main at task start was bd9110cb3f998f41241b01da295ef147d2dd428e and later advanced only through terminal steady-state branch-hygiene archive closeout #1091 to 536c6320f91d1df981600530e50522d84b1c0588.
  - PR #1093 history now includes current `main@536c6320f91d1df981600530e50522d84b1c0588` as an ancestor through a non-content merge commit; exact compare is behind_by=0 and the two #1091 synchronization paths no longer appear in the effective PR diff.
  - Issue #1072 is closed completed; its historical-work task is archived and 37 reviewed historical refs are terminally absent.
  - Issue #1089 is terminal; steady-state branch hygiene now owns future unexplained-ref detection and no second cleanup programme is required.
  - PR #1086 owns CI/workflow orchestration including ci.yml and BUILD_TEST_MATRIX; Issue #1092 does not claim or modify those paths.
  - Existing `ci.yml` already runs `python tools/validation/test_prompt_eval.py` and `python tools/validation/prompt_eval.py` in the blocking classify-changes path, so the new semantic test needs no workflow edit.
  - Portal Completion is explicitly the sole live selector; the completion-scope JSON is non-scheduling and cannot promote READY.
  - Work Allocation uses non-scheduling delivery bands/maturity and classifies multi-world plus PlayerCompanion follow-ups as conditional rather than standing READY work.
  - Architecture Authority separates global Roadmap, portal architecture, delivery plan, live selector, post-selection allocation and exact execution ownership.
  - Architecture Review owns new/superseding durable decisions; Portal delivery only applies/decomposes accepted architecture.
  - Historical Work Reconciliation prompt is TERMINAL_DO_NOT_RUN and routes future branch hygiene to ADR 0037/0039/current governance.
  - Game Catalog specialized programme preserves its exact historical PR/schema/task provenance while classifying it as historical planning evidence; it grants no cross-repository access or mutation authority and requires exact current owner authorization before any server/game repository access.
  - Game Catalog `atomic-required` ordering remains unchanged in effect: no producer schema may merge before a compatible inactive consumer exists; the new wording only adds that the dependency does not itself grant repository/staging/production authority.
  - Completion-scope commerce semantics require a terminal implement/defer/reject product disposition while keeping production payment activation separately deferred/protected.
  - `tools/validation/test_prompt_eval.py` parses the scope manifest and fails closed on scheduler/ownership/live-state leakage, invalid dispositions/selector entries, duplicate workstream ids, missing conditional triggers and drift of critical multi-world/PlayerCompanion/API/commerce invariants.
  - Draft PR #1093 is the sole authoritative Issue #1092 delivery PR.
  - Duplicate PRs #1094 and #1095 are closed unmerged with explicit delete disposition; exact branch search returned only the authoritative `docs/portal-programme-coherence-20260815` ref for this slug.
derived:
  - The portal control plane has one live scheduler, one separate launch-disposition projection and one post-selection execution matrix instead of overlapping pseudo-queues.
  - The completion-scope projection is protected both by text-contract cases and semantic JSON validation.
  - Game Catalog authority was corrected without discarding the specialized programme's historical audit trail or weakening its consumer-first compatibility gate.
  - No live product/CI/branch-hygiene implementation owner overlaps Issue #1092.
  - The remaining work is exact-head validation/review and lifecycle closeout, not additional product design.
unknown:
  - Exact final required CI conclusions on the final checkpoint head until current workflows complete.
  - Final absence of the authoritative task source ref until merge/source-branch closeout.
conflicts: []
first_failure:
  marker: self-review-game-catalog-atomic-ordering-regression
  evidence: an intermediate Issue #1092 Game Catalog rewrite changed the existing atomic-required dependency from "producer schema may not merge before a compatible inactive consumer" to the weaker "may not become active" formulation.
resolved_findings:
  - finding: GAME-CATALOG-ORDER-01
    severity: P1
    resolution: restored the exact merge-before-compatible-consumer prohibition and retained only the intended authority qualifier; historical PR/schema/task provenance was also restored instead of being unnecessarily generalized away.
rejected_hypotheses:
  - merge all portal documents into one monolith
  - create a second portal scheduler
  - create a second historical cleanup programme
  - treat Work Allocation P-bands as live priority
  - treat multi-world as speculative standalone infrastructure
  - allow a specialized programme to grant cross-repository authority
  - weaken Platform-only external-repository authority to match legacy programme prose
changed_paths:
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/programs/GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/prompts/OTERYN-HISTORICAL-WORK-RECONCILIATION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/tasks/active/OTERYN-20260815-portal-programme-coherence.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - tools/validation/test_prompt_eval.py
validation:
  - command: live repository/ownership preflight
    result: PASS
    evidence: current main, active tasks, open PRs and relevant programme/authority documents were inspected before mutation
  - command: current-main ancestry/effective diff reconciliation
    result: PASS
    evidence: branch history includes main@536c6320f91d1df981600530e50522d84b1c0588; compare reports behind_by=0 and exactly 11 effective Issue #1092 paths
  - command: manual prompt/control-plane scenario matrix
    result: PASS
    evidence: eight bounded scenarios reviewed: stale Work Allocation cannot reorder selector; REQUIRED does not mean READY; inactive CONDITIONAL does not self-activate; concurrent existing portal PR remains OWNED not duplicated; new ADR-level question routes to Architecture Review; multi-world without a trigger creates no speculative task; historical #1072 invocation stops at tombstone; Game Catalog cannot inspect a server repository without current explicit authorization
  - command: deterministic prompt-contract + semantic scope tests
    result: PENDING_FINAL_HEAD_CI
    evidence: dedicated v1.3 hierarchy/scope/parallelism/tombstone/external-authority text cases plus semantic JSON assertions are in the blocking test path; no claim of model/runtime adherence is made
  - command: self-review repair
    result: PASS
    evidence: intermediate Game Catalog ordering/provenance regression was identified before readiness and repaired; final exact-head whole-diff review remains the next action
  - command: model/runtime prompt trials
    result: NOT_RUN
    evidence: no separately authorized owner-funded model invocation was used; deterministic repository contract and manual scenario review do not claim stochastic model adherence
  - command: duplicate branch/PR lifecycle
    result: PASS
    evidence: PR #1094 and #1095 are closed unmerged with delete disposition and exact branch search returns only authoritative Issue #1092 ref
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: task changes only governance/documentation/prompt contracts and introduces no executable product/runtime/browser journey
blockers: []
next_action: Perform exact-head whole-diff self-review on this checkpoint head, then inspect its required CI once under the bounded terminal-CI rules and repair only a concrete failing gate.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 5
  session_id: github-20260815-portal-programme-coherence
  session_started_at: 2026-08-15T09:07:50Z
  checkpointed_at: 2026-08-15T09:34:00Z
  last_progress_at: 2026-08-15T09:34:00Z
  phase: validate
  exact_head: LIVE_PR_1093_HEAD
  pull_request: 1093
  active_operation: exact-head self-review and repository validation
  external_run_ids: []
  operation_started_at: 2026-08-15T09:34:00Z
  wait_deadline_at: null
  check_generation: final-draft
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: PR #1093 remains the authoritative Issue #1092 branch and no ownership conflict appears
  next_action: Perform exact-head whole-diff self-review and inspect final required CI.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1092 is active on docs/portal-programme-coherence-20260815 and PR #1093
source_branch_evidence: pending
```

## Temporary branch bookkeeping

Two accidental exact-duplicate refs were created during connector branch-name resolution:

- `docs/issue-1091-portal-programme-coherence` -> closed unmerged duplicate PR #1094;
- `docs/portal-programme-coherence` -> closed unmerged duplicate PR #1095.

Both PRs carry `Branch-Disposition: delete`, and current exact branch search returns only `docs/portal-programme-coherence-20260815`. No duplicate ref remains live.
