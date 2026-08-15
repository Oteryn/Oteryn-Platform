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
  - docs/agents/PROMPTING_STANDARD.md
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

- [x] Global Roadmap, Portal Completeness Architecture, Portal Delivery Plan, live Portal Completion selector and Work Allocation have explicit non-overlapping responsibilities.
- [x] `OTERYN_PORTAL_COMPLETION` remains the sole live selector.
- [x] `OTERYN_PORTAL_COMPLETION_SCOPE.json` is machine-readable but non-scheduling/non-owning/non-live-state.
- [x] Architecture Review owns new durable ADR-level decisions; Portal coordination applies/decomposes accepted architecture.
- [x] Multi-world/ruleset/season handling is a conditional cross-cutting invariant rather than a standing queue item.
- [x] PlayerCompanion foundation is terminal; follow-up tools are independently promoted conditional slices.
- [x] Work Allocation is explicitly a non-scheduling capability maturity/execution matrix.
- [x] Historical Work Reconciliation prompt is tombstoned `TERMINAL_DO_NOT_RUN`.
- [x] Game Catalog cannot grant standing server/game-repository authority and preserves the existing atomic consumer-first merge gate.
- [x] Deterministic prompt-contract cases and semantic scope-manifest validation protect the new invariants.
- [x] Open CI refactor #1086 paths remain untouched.
- [x] Duplicate bookkeeping PRs #1094/#1095 are closed and their refs are absent.
- [ ] Exact-head CI passes after the checkpoint-format repair.
- [ ] Exact-head final self-review/review hygiene remain terminal on the repaired head.
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
updated_at: 2026-08-15T09:42:00Z
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
  - Portal Completion is explicitly the sole live selector and Work Allocation is post-selection only.
  - Completion scope cannot select work, claim ownership, persist live selector state or promote READY.
  - Architecture Review owns new or superseding durable decisions; Portal delivery applies accepted architecture.
  - Multi-world/ruleset/season is conditional cross-cutting applicability and PlayerCompanion follow-ups are conditional independent slices.
  - Historical Work Reconciliation is TERMINAL_DO_NOT_RUN and branch hygiene remains ADR 0037/0039 governance.
  - Game Catalog retains historical PR/schema/task provenance and its atomic-required rule that no producer schema may merge before a compatible inactive consumer exists.
  - Game Catalog grants no standing cross-repository access or mutation authority from a Platform invocation.
  - Prompt contract evaluation on prior head d0bbebf331a13e9ab8cc3c10581b15de5c45bac4 passed 31 cases across 11 categories with 8 safety-critical cases and zero model trials claimed.
  - Semantic portal completion scope test passed on prior head d0bbebf331a13e9ab8cc3c10581b15de5c45bac4.
  - Exact-head whole-diff self-review on prior head d0bbebf331a13e9ab8cc3c10581b15de5c45bac4 recorded PASS after repairing GAME-CATALOG-ORDER-01.
  - Agent Governance run 31877385848 failed only because this task checkpoint used unsupported nested key resolved_findings; all earlier governance/prompt-policy steps in that job passed.
  - Live task liveness on the failing Agent Governance run still validated all active tasks with zero advisory findings.
derived:
  - The current repair is checkpoint-format-only and does not alter the accepted portal control-plane design or product/runtime behavior.
  - New exact-head CI is required because the task file changed after the prior self-review/CI generation.
unknown:
  - Exact conclusions of required CI on the repaired head until the new generation completes.
  - Final implementation and closeout merge SHAs until merge occurs.
conflicts: []
first_failure:
  marker: checkpoint-parser-unsupported-nested-key
  evidence: Agent Governance run 31877385848 step Validate active task checkpoints failed with scalar key resolved_findings cannot have nested values; the unsupported key has been removed in this revision.
rejected_hypotheses:
  - prompt-contract evaluation failed
  - semantic completion-scope validation failed
  - live task ownership is invalid
  - weaken checkpoint validation to accept an ad-hoc field
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
  - command: manual portal control-plane scenario matrix
    result: PASS
    evidence: stale allocation cannot reorder; REQUIRED is not READY; inactive conditional does not self-activate; existing portal owner remains OWNED; ADR-level decision routes Architecture Review; no-trigger multi-world creates no speculative task; historical #1072 stops at tombstone; Game Catalog cannot inspect server repo without current permission.
  - command: prior exact-head prompt contract and semantic scope tests
    result: PASS
    evidence: Agent Governance run 31877385848 passed prompt evaluator tests and prompt suite; test_portal_completion_scope_manifest_contract also passed.
  - command: prior exact-head whole-diff self-review
    result: PASS
    evidence: PR review 4943508478 on d0bbebf331a13e9ab8cc3c10581b15de5c45bac4; zero unresolved review threads.
  - command: repaired exact-head repository validation
    result: PENDING
    evidence: new generation is required after the checkpoint-format-only change.
  - command: model/runtime prompt trials
    result: NOT_APPLICABLE
    evidence: no separately authorized owner-funded model invocation was used; deterministic/static checks do not claim stochastic model adherence.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: repository governance/documentation/prompt-contract task creates no product runtime or browser journey.
blockers: []
next_action: Inspect the repaired exact-head Agent Governance and CI generation; if green, record an exact-head checkpoint-only self-review and proceed through normal merge/closeout without marking Ready solely to trigger owner-funded review.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1092 remains active on PR #1093; branch must be deleted after successful merge and verified absent before terminal closeout.
source_branch_evidence: pending
```
