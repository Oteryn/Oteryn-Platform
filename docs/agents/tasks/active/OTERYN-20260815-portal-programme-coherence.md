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
- [x] Global Portal Completion requires per-capability owner-approved IMPLEMENT/DEFER/REJECT proof for the exact named-release benchmark inventory; broad workstream dispositions cannot substitute.
- [x] Every per-capability record requires stable `capability_id`, owner, rationale, outcome and authority evidence; missing/duplicate/conflicting/ambiguous evidence remains `DECISION_REQUIRED`.
- [x] Portal programme, executable prompt, Work Allocation and Delivery Plan use the same per-capability global-completion gate.
- [x] Work Allocation remains a non-scheduling capability maturity/execution matrix and does not make implementation mandatory for capabilities dispositioned DEFER/REJECT.
- [x] Historical Work Reconciliation is tombstoned `TERMINAL_DO_NOT_RUN`.
- [x] Game Catalog grants no standing server-repository authority and preserves atomic consumer-first merge ordering.
- [x] Deterministic validation pins exact scope root/rules schema, canonical authority/delivery/selector pointers, absence of root live-state fields, exact workstreams/triggers/boundaries, PlayerCompanion inventory, global per-capability proof contract and cross-document completion markers.
- [x] CI orchestration changes from #1086 remain non-overlapping with this task.
- [x] Duplicate PRs/refs #1094/#1095 are closed/absent.
- [x] All prior Codex findings through root-schema validation were remediated and resolved.
- [x] Codex P2 Work Allocation/Delivery Plan closeout-gate finding is remediated; manual self-review independently identified and included both affected documents plus exact `rules` semantics in the same fix.
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
updated_at: 2026-08-15T11:44:00Z
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
  - Protected main is dfec0c0683202c0c380b78cb4b956b2d19de68b8 and Issue #1092 candidate 6a145da169768e0e3185f550464f59bd931d5828 is behind_by=0 before completion-document reconciliation.
  - Portal Completeness Architecture requires named-release per-capability owner-approved dispositions before release-scope closure.
  - Portal Completion programme and executable prompt require exactly one durable per-capability record with stable capability_id, owner, rationale, outcome and authority_evidence; broad workstream dispositions cannot substitute and ambiguous evidence remains DECISION_REQUIRED.
  - Scope JSON remains non-scheduling/non-live and its root plus workstream/capability semantics are deterministic-test protected.
  - Exact-head 6a145da169768e0e3185f550464f59bd931d5828 passed Agent Governance 31882443297; its root-schema Codex P2 was replied to and resolved.
  - Codex review on 6a145da169768e0e3185f550464f59bd931d5828 identified one P2: Work Allocation's completion definition and the parallel Delivery Plan definition still allowed workstream-only closeout and omitted per-capability proof.
  - Independent manual cross-document review found the same Work Allocation/Delivery Plan drift and additionally found that scope `rules` meanings were not exact-schema pinned even after root-key validation.
  - The coherent remediation aligns Work Allocation and Delivery Plan with programme/prompt per-capability semantics and makes deterministic validation require exact root `rules` values, exact allowed-disposition list and completion markers in both documents.
  - Historical Work Reconciliation remains TERMINAL_DO_NOT_RUN; Game Catalog preserves atomic consumer-first merge ordering and no standing server-repository authority.
derived:
  - After this reconciliation no lower delivery-control document may define global completion using workstream terminality alone.
  - Follow-up implementation remains conditional on owner-approved IMPLEMENT; DEFER/REJECT are product decisions, not implementation requirements.
  - A fresh exact-head CI generation and Codex review are required because behavioural governance documents and validator changed.
unknown:
  - Exact reconciled PR head until the coherent commit is created.
  - Exact conclusions of required CI and final Codex review on that new head.
  - Final implementation and closeout merge SHAs until merge occurs.
conflicts: []
first_failure:
  marker: codex-exact-head-allocation-delivery-completion-drift
  evidence: Codex review submitted 2026-08-15T11:39:15Z on 6a145da169768e0e3185f550464f59bd931d5828 found Work Allocation and Delivery Plan still omitted the programme's required per-capability owner/rationale/outcome/authority-evidence proof.
rejected_hypotheses:
  - weaken programme/prompt per-capability proof to match lower documents
  - require implementation of every capability regardless of DEFER/REJECT outcome
  - leave duplicate completion definitions untested
  - allow scope rules semantics to drift while root keys stay fixed
  - skip exact-head revalidation because the change is documentation/test only
  - expand implementation or external-repository authority
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
  - command: prior exact-head Agent Governance
    result: PASS
    evidence: 6a145da169768e0e3185f550464f59bd931d5828 passed Agent Governance 31882443297 including root-schema semantic validation.
  - command: Codex review on prior candidate
    result: FAIL
    evidence: one P2 Work Allocation/Delivery Plan completion drift remains open until this coherent reconciliation is committed and re-reviewed.
  - command: cross-document manual self-review
    result: PASS
    evidence: independently confirmed both workstream-only completion definitions and added exact rules-semantic enforcement to the same repair scope.
  - command: reconciled exact-head repository validation
    result: NOT_RUN
    evidence: coherent reconciliation commit has not yet been created.
  - command: reconciled exact-head Codex review
    result: NOT_RUN
    evidence: must be re-triggered on the exact reconciliation head.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: governance/documentation/prompt-contract work creates no product runtime/browser journey.
blockers: []
next_action: Commit the Work Allocation/Delivery Plan/rules-semantic reconciliation, run exact-head CI, re-trigger Codex review, resolve the P2 thread, and merge only when every exact-head gate is terminal green.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1092 remains active on PR #1093; branch must be deleted after successful merge and verified absent before terminal closeout.
source_branch_evidence: pending
```
