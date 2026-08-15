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
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
search_first:
  - Issue #1092 and PR #1093
  - open PR #1086 CI orchestration ownership
  - terminal Issues #1072 and #1089
optional_reads:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md
---

# OTERYN-20260815 portal programme coherence

## Goal

Make portal delivery authority, live selection, completion scope, execution allocation, architecture handoff, historical-governance routing and cross-repository authority unambiguous and deterministically regression-protected without changing runtime behavior.

## Acceptance criteria

- [x] Roadmap, Portal Completeness Architecture, Portal Delivery Plan, live Portal Completion selector and Work Allocation have explicit non-overlapping responsibilities.
- [x] `OTERYN_PORTAL_COMPLETION` is the sole live selector.
- [x] Completion scope is machine-readable but non-scheduling/non-owning/non-live-state.
- [x] Architecture Review owns new durable ADR-level decisions; Portal coordination applies accepted architecture.
- [x] Multi-world/ruleset/season is a conditional cross-cutting invariant.
- [x] PlayerCompanion foundation is terminal; follow-ups are independent conditional slices.
- [x] Work Allocation is a non-scheduling capability maturity/execution matrix.
- [x] Historical Work Reconciliation is tombstoned `TERMINAL_DO_NOT_RUN`.
- [x] Game Catalog grants no standing server-repository authority and preserves atomic consumer-first merge ordering.
- [x] Prompt-contract and semantic completion-scope regression tests cover the new invariants.
- [x] CI orchestration paths owned by #1086 remain untouched.
- [x] Duplicate PRs/refs #1094/#1095 are closed/absent.
- [ ] Repaired exact-head Agent Governance and required CI pass.
- [ ] Final exact-head self-review/review hygiene remain terminal.
- [ ] PR/Issue/task/source-branch closeout is terminal.

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
modules:
  - repository-governance
  - portal-completion-control-plane
  - prompt-contracts
dependencies:
  - terminal Issue #1072 historical reconciliation
  - terminal Issue #1089 steady-state branch hygiene
  - open PR #1086 owns CI/workflow-economy paths and remains non-overlapping
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T09:52:00Z
head: LIVE_PR_1093_HEAD
branch: docs/portal-programme-coherence-20260815
pr: 1093
status: validating
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
  - Protected main is 536c6320f91d1df981600530e50522d84b1c0588 and PR #1093 includes it as an ancestor with behind_by=0 before this checkpoint-only repair.
  - Portal Completion is the sole live selector; completion scope cannot select work, claim ownership, persist live selector state or promote READY.
  - Work Allocation is post-selection only and Architecture Review owns new or superseding durable decisions.
  - Multi-world/ruleset/season is conditional cross-cutting applicability and PlayerCompanion follow-ups are conditional independent slices.
  - Historical Work Reconciliation is TERMINAL_DO_NOT_RUN and future branch hygiene remains ADR 0037/0039 governance.
  - Game Catalog retains historical provenance and the atomic-required prohibition on merging a producer schema before a compatible inactive consumer.
  - Game Catalog grants no standing cross-repository access or mutation authority from a Platform invocation.
  - Prompt contract evaluation and semantic portal completion scope validation passed on prior exact-head generations.
  - Whole-diff self-review on prior exact-head generations found and repaired GAME-CATALOG-ORDER-01 before readiness.
  - Agent Governance run 31877385848 failed only on unsupported nested checkpoint key resolved_findings; that key was removed.
  - Agent Governance run 31877632986 then failed only because validation result PENDING is outside the allowed evidence vocabulary; it is now recorded as NOT_RUN.
  - Live task ownership/liveness remained valid in both failed Agent Governance generations.
derived:
  - Both Agent Governance failures were task-checkpoint schema/evidence-vocabulary defects, not portal control-plane design, prompt-contract, scope-validator or live ownership failures.
  - A new exact-head validation generation is required after this checkpoint-only repair.
unknown:
  - Exact conclusions of required CI on the repaired head until the new generation completes.
  - Final implementation and closeout merge SHAs until merge occurs.
conflicts: []
first_failure:
  marker: checkpoint-validation-result-vocabulary
  evidence: Agent Governance run 31877632986 rejected validation result PENDING; repository contract permits BLOCKED, FAIL, NOT_APPLICABLE, NOT_RUN or PASS and this revision uses NOT_RUN for not-yet-executed repaired-head CI.
rejected_hypotheses:
  - prompt-contract evaluation failed
  - semantic completion-scope validation failed
  - live task ownership is invalid
  - weaken checkpoint validation
  - change CI workflow owned by PR #1086
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
  - command: portal control-plane manual scenario matrix
    result: PASS
    evidence: stale allocation cannot reorder; REQUIRED is not READY; inactive conditional does not self-activate; existing portal owner remains OWNED; ADR-level decision routes Architecture Review; no-trigger multi-world creates no speculative task; historical #1072 stops at tombstone; Game Catalog cannot inspect server repo without current permission.
  - command: prompt-contract and semantic scope validation on prior candidate
    result: PASS
    evidence: Agent Governance run 31877632986 passed prompt evaluator tests and prompt suite; semantic scope manifest test passed.
  - command: prior exact-head whole-diff self-review
    result: PASS
    evidence: PR reviews 4943508478 and 4943519012; zero unresolved material findings after GAME-CATALOG-ORDER-01 repair.
  - command: repaired exact-head repository validation
    result: NOT_RUN
    evidence: this checkpoint creates the repaired head; a new required CI generation will execute after commit.
  - command: model/runtime prompt trials
    result: NOT_APPLICABLE
    evidence: no separately authorized owner-funded model invocation was used and deterministic checks do not claim stochastic adherence.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: governance/documentation/prompt-contract work creates no product runtime or browser journey.
blockers: []
next_action: Inspect the repaired exact-head Agent Governance and required CI; if green, record exact-head self-review and proceed through normal merge/closeout without marking Ready solely to trigger owner-funded review.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1092 remains active on PR #1093; branch must be deleted after successful merge and verified absent before terminal closeout.
source_branch_evidence: pending
```
