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
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
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
- [x] PlayerCompanion foundation is terminal; implementation follow-ups remain individually conditional.
- [x] Global completion cannot bypass PlayerCompanion product decisions: a required decision gate inventories every remaining canonical capability and requires owner-approved IMPLEMENT/DEFER/REJECT plus rationale before terminal completion.
- [x] Work Allocation is a non-scheduling capability maturity/execution matrix.
- [x] Historical Work Reconciliation is tombstoned `TERMINAL_DO_NOT_RUN`.
- [x] Game Catalog grants no standing server-repository authority and preserves atomic consumer-first merge ordering.
- [x] Prompt-contract and semantic completion-scope regression tests cover the new invariants.
- [x] Every canonical scope workstream ID, disposition, selector entry, activation trigger and authority boundary is pinned fail-closed by deterministic validation.
- [x] PlayerCompanion decision inventory is pinned to the complete remaining P0/P1/P2 capability set from canonical architecture, without silently choosing product outcomes.
- [x] CI orchestration changes from #1086 remain non-overlapping with this task.
- [x] Duplicate PRs/refs #1094/#1095 are closed/absent.
- [x] Initial Codex P2 findings for checkpoint identity and full workstream inventory were remediated and verified by Agent Governance.
- [x] Exact-head Codex P1/P2 findings for PlayerCompanion disposition completeness and trigger semantics are remediated without weakening product/authority gates.
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
updated_at: 2026-08-15T11:04:28Z
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
  - Protected main advanced to dfec0c0683202c0c380b78cb4b956b2d19de68b8 only through lifecycle-only archive PR #1098 after the prior synchronization; that main delta does not overlap Issue #1092 owned paths.
  - Portal Completion is the sole live selector; completion scope cannot select work, claim ownership, persist live selector state or promote READY.
  - Work Allocation is post-selection only and Architecture Review owns new or superseding durable decisions.
  - Multi-world/ruleset/season is conditional cross-cutting applicability.
  - Portal Completeness Architecture requires every benchmark capability to have an IMPLEMENT/DEFER/REJECT disposition with owner and rationale before release-scope closure.
  - PlayerCompanion Architecture states product delivery remains incomplete until the owner-approved capability inventory has explicit implementation/defer/reject disposition.
  - PlayerCompanion canonical remaining capability inventory contains Loot Split; seven additional P0 follow-ups after the Session Analyzer foundation; nine P1 capabilities; and seven P2 capabilities, for twenty-four explicit remaining decisions.
  - Historical Work Reconciliation is TERMINAL_DO_NOT_RUN and future branch hygiene remains ADR 0037/0039 governance.
  - Game Catalog retains historical provenance and the atomic-required prohibition on merging a producer schema before a compatible inactive consumer.
  - Game Catalog grants no standing cross-repository access or mutation authority from a Platform invocation.
  - Exact-head 8c264ea814a06e3c05d3a6e7f321f3c71e345216 passed all eight emitted repository workflows, including Agent Governance 31880971013 and CI 31880971004.
  - Initial Codex P2 threads for checkpoint identity and incomplete workstream inventory were replied to with exact remediation evidence and resolved.
  - Exact-head Codex review on 8c264ea814a06e3c05d3a6e7f321f3c71e345216 identified a P1 gap where conditional PlayerCompanion follow-ups could disappear from global completion without explicit owner dispositions, plus a P2 gap where activation-trigger semantics were not pinned.
  - The remediation adds a REQUIRED non-implementation PlayerCompanion capability-inventory decision gate and preserves actual follow-up implementation as CONDITIONAL only after an owner-approved IMPLEMENT disposition.
  - The deterministic scope validator now pins every canonical ID, disposition, selector entry, activation trigger and boundary, plus the exact twenty-four-item PlayerCompanion decision inventory and allowed field shape.
derived:
  - The P1 remediation strengthens the existing owner-decision requirement without deciding IMPLEMENT/DEFER/REJECT outcomes on the owner's behalf.
  - The P2 remediation prevents trigger or authority-boundary drift from silently activating unauthorized work or suppressing required work.
  - A fresh exact-head CI generation and fresh Codex review are required after the coherent second-review remediation and current-main synchronization.
unknown:
  - Exact remediated PR head until the coherent second-review fix and main-sync commit is created.
  - Exact conclusions of required CI and final Codex review on that new head.
  - Final implementation and closeout merge SHAs until merge occurs.
conflicts: []
first_failure:
  marker: codex-exact-head-playercompanion-and-trigger-findings
  evidence: Codex review submitted 2026-08-15T11:04:27Z/28Z on exact head 8c264ea814a06e3c05d3a6e7f321f3c71e345216 identified P1 missing explicit PlayerCompanion inventory dispositions and P2 unpinned activation-trigger semantics; both are addressed in the next coherent commit.
rejected_hypotheses:
  - treat one terminal PlayerCompanion foundation workflow as satisfying disposition requirements for all remaining tools
  - silently mark all PlayerCompanion follow-ups DEFERRED or REJECTED without owner authority
  - make all PlayerCompanion follow-up implementations immediately REQUIRED
  - validate trigger strings only as non-empty
  - weaken or bypass checkpoint validation
  - treat a stale Codex review as sufficient after a head change
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
  - command: canonical architecture check for second Codex findings
    result: PASS
    evidence: PORTAL_COMPLETENESS_ARCHITECTURE completion gate requires per-capability IMPLEMENT/DEFER/REJECT; PLAYER_COMPANION_ARCHITECTURE completion criteria require owner-approved capability inventory dispositions.
  - command: prior exact-head repository validation
    result: PASS
    evidence: 8c264ea814a06e3c05d3a6e7f321f3c71e345216 passed all eight emitted workflows including Agent Governance 31880971013 and CI 31880971004.
  - command: exact-head Codex review on prior candidate
    result: FAIL
    evidence: P1 PlayerCompanion disposition completeness and P2 trigger-semantic pinning findings remain open until this coherent remediation is committed and re-reviewed.
  - command: second-review remediated exact-head repository validation
    result: NOT_RUN
    evidence: the coherent remediation/main-sync commit has not yet been created; exact current head is resolved from live PR state after commit.
  - command: second-review remediated exact-head Codex review
    result: NOT_RUN
    evidence: must be re-triggered after the coherent remediation/main-sync commit so review covers the exact final candidate.
  - command: dedicated stochastic prompt-adherence trials
    result: NOT_RUN
    evidence: Codex PR review is a code/governance review and is not represented as repeated prompt-runtime adherence evaluation.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: governance/documentation/prompt-contract work creates no product runtime or browser journey.
blockers: []
next_action: Commit the P1/P2 Codex remediations together with the non-overlapping current-main sync, run exact-head CI, re-trigger Codex review, resolve all threads, and merge only when every exact-head gate is terminal green.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1092 remains active on PR #1093; branch must be deleted after successful merge and verified absent before terminal closeout.
source_branch_evidence: pending
```
