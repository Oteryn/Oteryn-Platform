---
task_id: OTERYN-20260816-reference-source-architecture
repository: blakinio/Oteryn-Platform
mode: architecture
task_kind: discovery
issue: 1121
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-content
status: blocked
implementation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0034-native-game-catalog-content-ownership.md
  - docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/agents/programs/OTERYN_CONTENT_COMPLETION.md
  - docs/agents/handovers/OTERYN-20260816-content-audit-to-coordinator.md
search_first:
  - reference source authority CrystalServer content provenance
  - third-party structured facts GameCatalog Wiki PlayerCompanion
  - architecture decision backlog content authority
optional_reads: []
---

# OTERYN-20260816-reference-source-architecture

## Goal

Resolve Issue #1121 as an architecture-only package: determine whether a provenance-pinned, explicitly non-native/non-executable third-party reference source may be used by Oteryn Platform content surfaces without weakening ADR 0034 native authority or Legacy Canary Compatibility isolation.

## Acceptance criteria

- [x] Re-read current architecture authority, ADR 0034, native Game Catalog content contract and Legacy Canary import contract from current `main`.
- [x] Search for an existing accepted decision that already fully resolves the Issue; extend/reconcile rather than duplicate if one exists.
- [x] Enumerate the smallest viable alternatives: reject/defer reference profiles, permit a bounded reference-only profile, or use a separate non-authoritative read-model boundary.
- [x] For each alternative define identity, provenance, completeness/freshness, source conflict, profile separation, precedence and failure semantics.
- [x] Preserve native Oteryn-v2 content authority and prohibit CrystalServer/third-party data from impersonating native or `legacy-canary` runtime authority.
- [x] Keep executable gameplay parameters, availability/reachability and native canonical identity outside third-party reference authority.
- [x] Keep third-party prose/dialogue/maps/media publication rights as a separate decision; do not bulk-copy them.
- [x] Record the owner-approved architecture outcome using the repository ADR/contract process and provide explicit #1115 implementation handoffs.
- [x] No runtime/product code, migrations, deployment, production/staging mutation, external server/game repository access or owner-funded AI use.
- [x] Run documentation/governance validation and exact-head whole-diff self-review on the final architecture candidate before attempting readiness.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260816-reference-source-architecture.md
  - docs/architecture/adr/0042-non-native-reference-content-boundary.md
  - docs/architecture/adr/README.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/contracts/NON_NATIVE_REFERENCE_CONTENT_CONTRACT.md
  - docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md
modules:
  - Architecture Review
  - ReferenceContent
  - GameCatalog
  - Wiki
  - PlayerCompanion
dependencies:
  - Issue #1121
  - ADR 0034
  - OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md
  - GAME_CATALOG_IMPORT_CONTRACT.md
  - Issue #1115 content completion programme
blockers:
  - PR #1122 is still Draft and repository evidence proves Draft-to-Ready can trigger owner-funded Codex Code Review; this task explicitly forbids owner-funded Codex/OpenAI/API usage.
cross_repository_tasks:
  - none; external server/game repository access is not authorized
```

The architecture worker claimed the durable decision/contract/focused-architecture paths before editing them. `docs/architecture/MODULE_CATALOG.md` was initially reserved during design, then released unchanged because ADR 0042 defines a logical read-model/contract boundary and does not authorize or claim a delivered runtime module. No runtime/product paths are owned by this task.

## Owner-approved architecture outcome

ADR 0042 selects a separate logical `ReferenceContent` read-model boundary. A conforming source snapshot is permanently classified `NON_AUTHORITATIVE_REFERENCE`: it never becomes a native `GameCatalog` authority profile, never uses `legacy-canary`, never participates in activation/fallback, never mints native identity, and never proves current runtime availability/reachability.

The owner-supplied `crystalserver-main.zip` may therefore be used only through pinned, deterministic, static extraction/reconciliation semantics. `data-global` and `data-crystal` remain separate source profiles. Wiki and PlayerCompanion consumers must preserve the reference evidence class and limitations. Third-party prose/dialogue/maps/media publication rights remain outside this decision and fail closed under ADR 0026 / `THIRD_PARTY_NOTICES.md` until separately proven.

The architecture package is complete and validated on candidate head `01ea1a6c373be3a58432770c18ca1cd0adde73c0`, but it is **not canonical on `main`** because the PR cannot safely leave Draft under the current no-owner-funded-AI authorization. No merge, Issue closure or task archive is claimed.

## CONTENT-COORD terminal handoff

Current lane state is based on canonical merged authority, not on an unmerged draft:

```text
SOURCE-PIPELINE: BLOCKED
  Architecture question is resolved, and after ADR 0042 reaches main this lane becomes READY for a bounded deterministic static extraction/normalization pipeline into ReferenceContent.
  Current blocker: ADR 0042 is not yet canonical on main because PR #1122 cannot safely transition Draft -> Ready without risking forbidden owner-funded Codex review.

WIKI-REFERENCE: BLOCKED
  After ADR 0042 reaches main this lane becomes READY for a bounded structured-reference slice that visibly preserves NON_AUTHORITATIVE_REFERENCE provenance.
  Independent constraint remains: exact third-party publication rights fail closed; copied prose/dialogue/maps/media are not authorized by ADR 0042.

PLAYER-COMPANION: BLOCKED
  After ADR 0042 reaches main this lane becomes READY for a bounded reference-aware vertical slice with source-evidence provenance and limitations.
  Any workflow requiring current authoritative deterministic gameplay truth still depends on authoritative GameCatalog/ruleset evidence; ReferenceContent is never fallback authority.
```

`DECISION_REQUIRED` is **not** the architecture status: the bounded source-authority decision itself is resolved. The present stop is an execution/permission blocker preventing canonical merge. `CONTENT-COORD` must not dispatch these implementation lanes as READY until ADR 0042 is actually merged to protected `main`.

## Readiness blocker evidence

- PR #1122 is still Draft.
- Exact architecture candidate head `01ea1a6c373be3a58432770c18ca1cd0adde73c0` passed all observed repository workflows: Agent Governance `31972618507`, CI `31972618511`, Native protocol contract `31972618570`, Native protocol contract audits `31972618554`, Edge Security Emulation `31972618527`, Platform DB Outage Validation `31972618521`, Game Auth Ticket Concurrency `31972618601`, and Phase 7 Production-Like Validation `31972618562`.
- Exact-head whole-diff self-review is PASS on `01ea1a6c373be3a58432770c18ca1cd0adde73c0`; structured review `4947312319` reports zero findings.
- Review threads are zero and PR #1122 was mergeable while Draft.
- Merged PR #1117 records the repository precedent: its Draft -> Ready transition was held while owner-funded Codex was unauthorized; only after explicit one-time authorization did the transition occur, and `chatgpt-codex-connector[bot]` then consumed the code-review path and returned a usage-limit result. PR #1116 repeats the same safety rule for the current content programme.
- The current task instruction explicitly says: `Nie używaj owner-funded Codex/OpenAI/API.` No equivalent one-time authorization exists for PR #1122.
- Therefore Draft -> Ready is not an authorized action and direct merge/bypass is also forbidden by repository merge gates.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T21:13:10Z
invocation_started_at: 2026-08-16T20:55:00Z
last_progress_at: 2026-08-16T21:13:10Z
head: 01ea1a6c373be3a58432770c18ca1cd0adde73c0
branch: docs/issue-1121-reference-source-architecture
pr: 1122
status: blocked
phase: merge_gate
session_role: architecture-reviewer
execution_mode: github
execution_reason: bounded documentation/contract architecture package with GitHub-native validation
project_lane: oteryn-platform-content
context_pressure: medium
context_growth: stable
context_score: 6
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one durable source-authority question with one coherent ADR/contract/focused-doc package
ci_checks_for_current_head: 2
ci_check_generation: final-architecture-candidate
terminal_ci_wait_started_at: 2026-08-16T21:10:31Z
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 1
stall_warnings: 0
context_routes:
  - architecture
  - agent-governance
  - content
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260816-reference-source-architecture.md
  - docs/architecture/adr/0042-non-native-reference-content-boundary.md
  - docs/architecture/adr/README.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/contracts/NON_NATIVE_REFERENCE_CONTENT_CONTRACT.md
  - docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md
proven:
  - Issue #1121 is open and labelled agent:ready
  - draft PR #1122 exists on docs/issue-1121-reference-source-architecture
  - protected main remained f617120975cb1522cad87d74f8bea37f829b2b64 through final architecture-candidate validation
  - ADR 0034 assigns native gameplay-content authority to Oteryn-v2 while Platform owns catalogue lifecycle and Legacy Canary Compatibility remains an explicit anti-corruption boundary
  - owner-supplied crystalserver-main.zip is pinned at SHA-256 920a59e15175a5f53721f60b17f4bb37370bf0b61cd91abb4c909bf0d85e5f26 and data-global/data-crystal are separate source profiles
  - architecture candidate 01ea1a6c373be3a58432770c18ca1cd0adde73c0 contains ADR 0042, NON_NATIVE_REFERENCE_CONTENT_CONTRACT and focused authority/GameCatalog/PlayerCompanion reconciliations
  - all observed workflows on architecture candidate 01ea1a6c373be3a58432770c18ca1cd0adde73c0 completed successfully
  - structured exact-head self-review 4947312319 on 01ea1a6c373be3a58432770c18ca1cd0adde73c0 is PASS with findings []
  - PR #1122 has zero review threads and no requested-change review
  - PR #1117 comment 5309467678 proves explicit one-time owner authorization was required before its Draft-to-Ready Codex-triggering transition; bot comment 5309468353 proves that path invoked Codex code-review usage
  - PR #1116 explicitly remains Draft under the same current programme because Draft-to-Ready may invoke owner-funded Codex without authorization
  - current user instruction explicitly forbids owner-funded Codex/OpenAI/API for this task
  - no external server/game repository was accessed during takeover, analysis, architecture editing or validation
derived:
  - the bounded architecture decision is resolved but is not canonical until merged to protected main
  - under current authorization, changing PR #1122 from Draft to Ready would create a material risk of invoking a forbidden owner-funded Codex review
  - direct merge, branch-protection bypass, duplicate ready PR or equivalent workaround would violate repository governance rather than safely remove the blocker
unknown:
  - whether the repository owner can disable automatic/metered Codex review for this specific Ready transition without changing task authorization
conflicts: []
first_failure:
  marker: readiness_transition_requires_forbidden_metered_ai_risk
  evidence: PR #1117 comments 5309171072 and 5309467678/5309468353 plus current PR #1116 body establish the Draft-to-Ready metered Codex path; task authorization explicitly forbids it
rejected_hypotheses:
  - promote CrystalServer source material to native Oteryn authority
  - reuse legacy-canary authority name for a CrystalServer source
  - blend third-party fields into authoritative rows
  - place third-party reference snapshots inside authoritative GameCatalog activation/profile lifecycle
  - execute source Lua or other source code to derive reference facts
  - treat archive extraction time or source metadata as current Oteryn freshness
  - mark PR #1122 Ready despite the explicit no-owner-funded-AI instruction
  - bypass Draft/branch-protection/ordinary PR merge gates
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260816-reference-source-architecture.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/architecture/adr/0042-non-native-reference-content-boundary.md
  - docs/architecture/adr/README.md
  - docs/contracts/NON_NATIVE_REFERENCE_CONTENT_CONTRACT.md
  - docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md
validation:
  - command: fresh admission/ownership/uniqueness review
    result: PASS
    evidence: Issue #1121, PRs #1116/#1120/#1122, active task inventory, ADR inventory and open 0042 search
  - command: architecture candidate exact-head CI
    result: PASS
    evidence: 01ea1a6c373be3a58432770c18ca1cd0adde73c0; CI run 31972618511 and all other observed workflow runs completed successfully
  - command: exact-head whole-diff self-review
    result: PASS
    evidence: review 4947312319 on 01ea1a6c373be3a58432770c18ca1cd0adde73c0; findings []
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture-only documentation/contract task; no executable product or integration journey is changed
blockers:
  - safe non-metered Draft-to-Ready path for PR #1122 is not proven and owner-funded Codex/OpenAI/API usage is explicitly forbidden
next_action: establish a proven non-metered way to transition PR #1122 out of Draft (for example by disabling automatic Codex code review for this transition); then revalidate the exact head and continue ordinary squash merge and task archive closeout
```

## Source branch closeout

```yaml
source_branch_disposition: retain_while_blocked
source_branch_reason: PR #1122 is the sole architecture delivery path; it cannot be merged or closed without losing the owner-approved but not-yet-canonical architecture package.
source_branch_evidence: PR #1122 remains open Draft on docs/issue-1121-reference-source-architecture; current blocker is the forbidden metered-AI readiness transition risk.
```

## Notes

This task was dispatched by `CONTENT-COORD` after the content audit proved that source/native authority is a gating dependency for source-driven Wiki and Player Companion expansion. The owner explicitly instructed this Architecture Review to resolve the bounded decision autonomously. That authorization resolved the architecture choice inside this repository but explicitly excluded owner-funded Codex/OpenAI/API usage, so repository canonicalization must fail closed at the Ready transition until a non-metered path is proven.
