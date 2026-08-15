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
- [x] The executable Portal Completion prompt repeats the same per-capability proof gate and cannot be satisfied by broad workstream terminality alone.
- [x] Work Allocation is a non-scheduling capability maturity/execution matrix.
- [x] Historical Work Reconciliation is tombstoned `TERMINAL_DO_NOT_RUN`.
- [x] Game Catalog grants no standing server-repository authority and preserves atomic consumer-first merge ordering.
- [x] Deterministic validation pins scope workstreams, exact trigger/boundary semantics, PlayerCompanion inventory and the global per-capability disposition-proof contract.
- [x] CI orchestration changes from #1086 remain non-overlapping with this task.
- [x] Duplicate PRs/refs #1094/#1095 are closed/absent.
- [x] Prior Codex findings for checkpoint identity, full workstream inventory, PlayerCompanion disposition completeness, trigger semantics and programme-level per-capability proof were remediated and resolved.
- [x] Final Codex P2 executable-prompt consistency finding is remediated.
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
updated_at: 2026-08-15T11:24:09Z
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
  - Protected main is dfec0c0683202c0c380b78cb4b956b2d19de68b8 and Issue #1092 exact candidate 70b7f7824f02d92bbb93a3dbb3d874d63c375a04 is behind_by=0 before the final prompt-consistency remediation.
  - Portal Completeness Architecture requires a named-release capability inventory and owner-approved IMPLEMENT/DEFER/REJECT disposition with rationale for every benchmark capability.
  - Portal Completion programme now requires exactly one durable per-capability record with stable capability_id, owner, rationale, outcome and authority_evidence; broad workstream dispositions cannot substitute and ambiguous evidence remains DECISION_REQUIRED.
  - Scope JSON carries the machine-readable per-capability proof contract and semantic validation pins it exactly.
  - Portal Completion remains the sole live selector; Work Allocation is post-selection only; Architecture Review owns new durable decisions; multi-world is conditional cross-cutting applicability.
  - Historical Work Reconciliation is TERMINAL_DO_NOT_RUN; Game Catalog preserves atomic consumer-first merge ordering and no standing server-repository authority.
  - Exact-head 70b7f7824f02d92bbb93a3dbb3d874d63c375a04 passed all eight emitted workflows, including Agent Governance 31881811371 and CI 31881811345, and exact-head self-review PASS is recorded as PR review 4943733590.
  - The third Codex programme-level per-capability P2 was remediated, replied to with exact evidence and resolved.
  - Final Codex review on 70b7f7824f02d92bbb93a3dbb3d874d63c375a04 identified one P2: executable prompt global completion wording was weaker than the strengthened programme and could still be satisfied by broad workstream dispositions.
  - The remediation makes the executable prompt require the exact same canonical per-capability inventory, stable ID, owner, rationale, outcome, authority-evidence, uniqueness and DECISION_REQUIRED semantics as the programme.
derived:
  - This is a prompt consistency repair only; it changes no product outcome, workstream disposition, architecture decision, runtime behavior, CI workflow or repository authority.
  - A fresh exact-head CI generation and Codex review are required because the executable prompt is behavioural code.
unknown:
  - Exact remediated PR head until the coherent prompt-consistency commit is created.
  - Exact conclusions of required CI and final Codex review on that new head.
  - Final implementation and closeout merge SHAs until merge occurs.
conflicts: []
first_failure:
  marker: codex-exact-head-executable-prompt-capability-proof-gap
  evidence: Codex review submitted 2026-08-15T11:24:09Z on 70b7f7824f02d92bbb93a3dbb3d874d63c375a04 found the executable prompt global-completion checklist still allowed broad workstream dispositions to substitute for per-capability proof required by the programme.
rejected_hypotheses:
  - weaken the programme-level per-capability proof to match the prompt
  - treat workstream terminality as capability proof
  - infer owner/rationale from inactivity or missing ownership
  - skip exact-head revalidation because the change is text-only
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
  - command: prior exact-head repository validation
    result: PASS
    evidence: 70b7f7824f02d92bbb93a3dbb3d874d63c375a04 passed all eight emitted workflows including Agent Governance 31881811371 and CI 31881811345.
  - command: prior exact-head self-review
    result: PASS
    evidence: PR review 4943733590 records zero open material findings after programme-level per-capability remediation.
  - command: final Codex review on prior candidate
    result: FAIL
    evidence: one P2 executable-prompt consistency finding remains open until this coherent remediation is committed and re-reviewed.
  - command: prompt-consistency remediated exact-head repository validation
    result: NOT_RUN
    evidence: coherent prompt-consistency commit has not yet been created.
  - command: prompt-consistency remediated exact-head Codex review
    result: NOT_RUN
    evidence: must be re-triggered on the exact remediation head.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: governance/documentation/prompt-contract work creates no product runtime/browser journey.
blockers: []
next_action: Commit the executable-prompt per-capability proof remediation, run exact-head CI, re-trigger Codex review, resolve the P2 thread, and merge only when every exact-head gate is terminal green.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1092 remains active on PR #1093; branch must be deleted after successful merge and verified absent before terminal closeout.
source_branch_evidence: pending
```
