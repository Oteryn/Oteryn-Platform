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
- [x] PlayerCompanion global completion cannot bypass the canonical capability inventory: twenty-four remaining capabilities require owner-approved IMPLEMENT/DEFER/REJECT decisions and rationale.
- [x] Global Portal Completion requires per-capability proof for the complete named-release benchmark inventory; broad workstream dispositions cannot substitute.
- [x] Every per-capability record requires stable `capability_id`, owner, rationale, outcome and authority evidence; missing/duplicate/conflicting/ambiguous evidence remains `DECISION_REQUIRED`.
- [x] Work Allocation is a non-scheduling capability maturity/execution matrix.
- [x] Historical Work Reconciliation is tombstoned `TERMINAL_DO_NOT_RUN`.
- [x] Game Catalog grants no standing server-repository authority and preserves atomic consumer-first merge ordering.
- [x] Deterministic validation pins scope workstreams, exact trigger/boundary semantics, PlayerCompanion inventory and the global per-capability disposition-proof contract.
- [x] CI orchestration changes from #1086 remain non-overlapping with this task.
- [x] Duplicate PRs/refs #1094/#1095 are closed/absent.
- [x] Prior Codex checkpoint/full-inventory, PlayerCompanion-disposition and trigger-semantic findings were remediated and resolved.
- [x] Third exact-head Codex P2 per-capability-proof finding is remediated without inventing product outcomes.
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
updated_at: 2026-08-15T11:13:34Z
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
  - Protected main is dfec0c0683202c0c380b78cb4b956b2d19de68b8 and the current Issue #1092 candidate bcf6b437012a081761352d34ba2ace760f549a95 is behind_by=0 before the third-review remediation.
  - Portal Completeness Architecture requires an explicit named-release capability inventory and an IMPLEMENT/DEFER/REJECT disposition with owner and rationale for every benchmark capability before release-scope closure.
  - PlayerCompanion Architecture separately requires owner-approved capability inventory dispositions before its product delivery can be complete.
  - Portal Completion is the sole live selector; scope/workstream rows cannot prove live state or READY.
  - Work Allocation is post-selection only and Architecture Review owns new durable decisions.
  - Historical Work Reconciliation is TERMINAL_DO_NOT_RUN; future branch hygiene remains ADR 0037/0039 governance.
  - Game Catalog retains historical provenance, atomic consumer-first merge ordering and no standing server-repository authority.
  - Exact-head bcf6b437012a081761352d34ba2ace760f549a95 passed Agent Governance 31881322784 and all eight emitted workflows, including CI 31881322729.
  - First four Codex findings were remediated, replied to with exact evidence and resolved.
  - Third exact-head Codex review on bcf6b437012a081761352d34ba2ace760f549a95 identified one P2: broad workstream terminality could still substitute for per-capability owner/rationale/outcome proof.
  - The remediation adds a machine-readable global per-capability disposition contract and makes missing, duplicate, conflicting or ambiguous records DECISION_REQUIRED.
  - Deterministic validation now pins the per-capability proof contract and mandatory Portal Completion programme markers in addition to exact workstream/trigger/boundary semantics.
derived:
  - The remediation strengthens the existing completion boundary without choosing IMPLEMENT/DEFER/REJECT outcomes or expanding implementation scope.
  - A broad terminal workstream can no longer justify global completion unless every canonical member capability has its own owner-approved durable record.
  - A fresh exact-head CI generation and Codex review are required after this coherent remediation.
unknown:
  - Exact remediated PR head until the coherent commit is created.
  - Exact conclusions of required CI and final Codex review on that new head.
  - Final implementation and closeout merge SHAs until merge occurs.
conflicts: []
first_failure:
  marker: codex-exact-head-global-per-capability-proof-gap
  evidence: Codex review submitted 2026-08-15T11:13:34Z on bcf6b437012a081761352d34ba2ace760f549a95 found that broad workstream dispositions could satisfy the written global rule while benchmark capabilities still lacked owner/rationale/outcome records required by canonical architecture.
rejected_hypotheses:
  - enumerate or decide all product outcomes inside this governance repair
  - let workstream DEFER or REJECT imply member-capability decisions
  - treat inactivity or missing ownership as an implicit capability disposition
  - weaken or bypass canonical Portal Completeness Architecture
  - weaken CI or Codex review requirements
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
  - command: canonical architecture verification
    result: PASS
    evidence: PORTAL_COMPLETENESS_ARCHITECTURE requires explicit capability inventory plus per-benchmark IMPLEMENT/DEFER/REJECT owner+rationale proof; PlayerCompanion has the same product-delivery invariant.
  - command: prior exact-head repository validation
    result: PASS
    evidence: bcf6b437012a081761352d34ba2ace760f549a95 passed all eight emitted workflows including Agent Governance 31881322784 and CI 31881322729.
  - command: third exact-head Codex review on prior candidate
    result: FAIL
    evidence: one P2 per-capability global-completion proof gap remains open until this coherent remediation is committed and re-reviewed.
  - command: remediated exact-head repository validation
    result: NOT_RUN
    evidence: coherent remediation commit has not yet been created.
  - command: remediated exact-head Codex review
    result: NOT_RUN
    evidence: must be re-triggered on the exact remediation head.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: governance/documentation/prompt-contract work creates no product runtime/browser journey.
blockers: []
next_action: Commit the per-capability global-completion proof contract, run exact-head CI, re-trigger Codex review, resolve the P2 thread, and merge only when every exact-head gate is terminal green.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1092 remains active on PR #1093; branch must be deleted after successful merge and verified absent before terminal closeout.
source_branch_evidence: pending
```
