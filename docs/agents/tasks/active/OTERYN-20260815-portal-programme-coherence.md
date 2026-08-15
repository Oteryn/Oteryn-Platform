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
  - merged PR #1086 CI orchestration
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
- [x] CI orchestration changes from #1086 remain non-overlapping with this task.
- [x] Duplicate PRs/refs #1094/#1095 are closed/absent.
- [x] Codex P2 checkpoint finding is remediated by using schema-valid `UNKNOWN` rather than a symbolic pseudo-SHA.
- [x] Codex P2 scope-inventory finding is remediated by fail-closed validation of all 21 canonical workstream IDs, dispositions and selector entries.
- [ ] Remediated exact-head Agent Governance and required CI pass.
- [ ] Final exact-head Codex review/self-review and review hygiene are terminal.
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
  - merged PR #1086 CI/workflow-economy state, non-overlapping with Issue #1092 paths
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T10:56:22Z
head: UNKNOWN
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
  - Protected main at Codex remediation is d5468c96a5a6dd5fd6a52d504e9cf8a2757f5a7e and the Issue #1092 branch was synchronized through eba31638e9b4f70c59bd76c4ac55b5456b10bd2b with behind_by=0 before applying review findings.
  - The main advances from merged #1086 and #1096 do not modify any of the eleven effective Issue #1092 paths; #1096 explicitly avoided ARCHITECTURE_AUTHORITY.md because PR #1093 owns it.
  - Portal Completion is the sole live selector; completion scope cannot select work, claim ownership, persist live selector state or promote READY.
  - Work Allocation is post-selection only and Architecture Review owns new or superseding durable decisions.
  - Multi-world/ruleset/season is conditional cross-cutting applicability and PlayerCompanion follow-ups are conditional independent slices.
  - Historical Work Reconciliation is TERMINAL_DO_NOT_RUN and future branch hygiene remains ADR 0037/0039 governance.
  - Game Catalog retains historical provenance and the atomic-required prohibition on merging a producer schema before a compatible inactive consumer.
  - Game Catalog grants no standing cross-repository access or mutation authority from a Platform invocation.
  - Exact-head e844ea0c731ab5347a300523d86891329a0868f6 passed Agent Governance, required CI and all emitted relevant validation workflows before the later non-overlapping main synchronization.
  - The repository owner explicitly authorized Ready-triggered Codex/OpenAI review for PR #1093 and Issue #1092.
  - Ready-triggered Codex review on e844ea0c731ab5347a300523d86891329a0868f6 produced two P2 findings: replace LIVE_PR_1093_HEAD with a schema-valid commit identity/UNKNOWN, and validate the complete 21-workstream completion-scope inventory.
  - This remediation uses head UNKNOWN because a task file cannot truthfully embed the SHA of the same commit that contains it; live PR state supplies the exact current head.
  - The semantic scope validator now requires exact set equality with all 21 canonical workstream IDs and verifies every canonical disposition and selector entry.
derived:
  - Both Codex findings are narrow fail-closed governance improvements and do not require changing portal product scope, architecture authority, CI workflows or runtime behavior.
  - A fresh exact-head CI generation and fresh Codex review are required after this remediation commit.
unknown:
  - Exact remediated PR head until the coherent review-fix commit is created.
  - Exact conclusions of required CI and final Codex review on that new head.
  - Final implementation and closeout merge SHAs until merge occurs.
conflicts: []
first_failure:
  marker: codex-ready-review-p2-findings
  evidence: Codex review submitted 2026-08-15T10:56:22Z identified the symbolic checkpoint head and partial six-of-twenty-one scope inventory enforcement; both are remediated in the next coherent commit.
rejected_hypotheses:
  - weaken or bypass checkpoint validation
  - treat the old e844ea0c Codex review as sufficient after a head change
  - validate only selected launch-critical workstreams
  - change CI workflow paths delivered by PR #1086
  - expand cross-repository authority
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
  - command: prior exact-head repository validation
    result: PASS
    evidence: e844ea0c731ab5347a300523d86891329a0868f6 passed Agent Governance 31878090616, CI 31878090559 and all emitted relevant validation workflows.
  - command: Ready-triggered Codex review on prior head
    result: FAIL
    evidence: review from chatgpt-codex-connector identified two P2 findings, both addressed in this coherent remediation.
  - command: remediated exact-head repository validation
    result: NOT_RUN
    evidence: the coherent review-fix commit has not yet been created; exact current head is resolved from live PR state after commit.
  - command: remediated exact-head Codex review
    result: NOT_RUN
    evidence: must be re-triggered after the coherent review-fix commit so review covers the exact final candidate.
  - command: dedicated stochastic prompt-adherence trials
    result: NOT_RUN
    evidence: Codex PR review is a code/governance review and is not represented as repeated prompt-runtime adherence evaluation.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: governance/documentation/prompt-contract work creates no product runtime or browser journey.
blockers: []
next_action: Commit the two Codex review remediations coherently, run exact-head CI, re-trigger Codex review on that head, resolve all threads, and merge only when the exact-head gates are terminal green.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1092 remains active on PR #1093; branch must be deleted after successful merge and verified absent before terminal closeout.
source_branch_evidence: pending
```
