---
task_id: OTERYN-20260816-reference-source-architecture
repository: blakinio/Oteryn-Platform
mode: architecture
task_kind: discovery
issue: 1121
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-content
status: validating
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
- [x] Record the accepted architecture outcome using the repository ADR/contract process and provide explicit #1115 implementation handoffs only where warranted.
- [x] No runtime/product code, migrations, deployment, production/staging mutation, external server/game repository access or owner-funded AI use.
- [ ] Run documentation/governance validation and exact-head whole-diff self-review before readiness.

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
blockers: []
cross_repository_tasks:
  - none; external server/game repository access is not authorized
```

The architecture worker claimed the durable decision/contract/focused-architecture paths before editing them. `docs/architecture/MODULE_CATALOG.md` was initially reserved during design, then released unchanged after self-scope review because ADR 0042 defines a logical read-model/contract boundary and does not authorize or claim a delivered runtime module. No runtime/product paths are owned by this task.

## Decision

ADR 0042 selects a separate logical `ReferenceContent` read-model boundary. A conforming source snapshot is permanently classified `NON_AUTHORITATIVE_REFERENCE`: it never becomes a native `GameCatalog` authority profile, never uses `legacy-canary`, never participates in activation/fallback, never mints native identity, and never proves current runtime availability/reachability.

The owner-supplied `crystalserver-main.zip` may therefore be used only through pinned, deterministic, static extraction/reconciliation semantics. `data-global` and `data-crystal` remain separate source profiles. Wiki and PlayerCompanion consumers must preserve the reference evidence class and limitations. Third-party prose/dialogue/maps/media publication rights remain outside this decision and fail closed under ADR 0026 / `THIRD_PARTY_NOTICES.md` until separately proven.

## CONTENT-COORD handoff

```text
SOURCE-PIPELINE: READY
  Architecture permits a bounded deterministic static extraction/normalization pipeline into ReferenceContent.
  No automatic publication, GameCatalog activation, runtime execution or authority promotion is permitted.

WIKI-REFERENCE: READY
  Architecture permits a bounded structured-reference slice that visibly preserves NON_AUTHORITATIVE_REFERENCE provenance.
  Exact third-party publication rights remain an independent fail-closed gate; copied prose/dialogue/maps/media are not authorized by ADR 0042.

PLAYER-COMPANION: READY
  Architecture permits a bounded reference-aware vertical slice with source-evidence provenance and limitations.
  Any workflow requiring current authoritative deterministic gameplay truth remains dependent on authoritative GameCatalog/ruleset evidence; ReferenceContent is never fallback authority.
```

These are architecture-dependency states only. `CONTENT-COORD` must refresh live ownership/path locks and dispatch separate implementation tasks before runtime/product edits.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T21:09:30Z
invocation_started_at: 2026-08-16T20:55:00Z
last_progress_at: 2026-08-16T21:09:30Z
head: 6a1c93dca4d154f1c4d0dcbcf550cb2e242b314d
branch: docs/issue-1121-reference-source-architecture
pr: 1122
status: validating
phase: validation
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
ci_check_generation: repair-required
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
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
  - draft PR #1122 exists on docs/issue-1121-reference-source-architecture and architecture candidate head was 6a1c93dca4d154f1c4d0dcbcf550cb2e242b314d before this checkpoint repair
  - protected main was f617120975cb1522cad87d74f8bea37f829b2b64 and matched PR #1122 base at takeover
  - ADR 0034 is accepted and assigns native gameplay-content authority to Oteryn-v2 while Platform owns catalogue lifecycle
  - ADR 0034 keeps Legacy Canary Compatibility importers as explicit anti-corruption adapters
  - OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT forbids co-authoritative source blending and allows historical/other snapshots to coexist for comparison without field blending
  - GAME_CATALOG_IMPORT_CONTRACT is scoped only to Legacy Canary Compatibility and does not authorize external Wiki data
  - owner-supplied CrystalServer archive is source material only under #1115 and is not native authority
  - archive identity is crystalserver-main.zip SHA-256 920a59e15175a5f53721f60b17f4bb37370bf0b61cd91abb4c909bf0d85e5f26; data-global and data-crystal are separate source profiles
  - content-audit handoff permits only source-side candidate counts and keeps production population/publication rights separate
  - Wiki accepted architecture owns editorial publication lifecycle and must not claim complete authoritative game content
  - PlayerCompanion accepted architecture consumes versioned evidence but must not own canonical game/source facts and already distinguishes deterministic, simulation and recommendation outputs
  - ADR 0026 and THIRD_PARTY_NOTICES keep unresolved third-party game-data/prose/assets rights fail-closed
  - current architecture decision backlog is empty
  - highest accepted/currently allocated ADR prefix on takeover main was 0041 and no open PR claimed 0042
  - open related PRs #1116 and #1120 do not overlap the architecture package; #1122 is the sole open reference-source architecture PR
  - the task-required OTERYN_CONTENT_COMPLETION programme file is not on takeover main; its candidate version was read from open draft PR #1116 and treated as non-authoritative programme evidence subordinate to Issue #1115 and merged audit handoff
  - architecture candidate commit 6a1c93dca4d154f1c4d0dcbcf550cb2e242b314d adds ADR 0042 and NON_NATIVE_REFERENCE_CONTENT_CONTRACT and reconciles Architecture Authority, PlayerCompanion and the native GameCatalog contract
  - CI run 31972496628 on 6a1c93dca4d154f1c4d0dcbcf550cb2e242b314d failed first at active task checkpoint validation because checkpoint_version was 2 while GOVERNANCE_CONTRACT requires structural version 1
  - docs/agents/GOVERNANCE_CONTRACT.json on main declares shared_checkpoint_contract.version 1 and policy revision 3 explicitly retains structural checkpoint version 1
  - no external server/game repository was accessed during takeover, analysis or architecture editing
derived:
  - ADR 0034 already rejects promoting CrystalServer-derived facts into native Oteryn gameplay authority
  - a separate logical ReferenceContent read-model boundary is the least ambiguous way to permit reference use without giving GameCatalog authority-profile or activation semantics to third-party material
  - ADR 0042 is required because the selected boundary introduces a durable dependency/source-authority rule that outlives Issue #1121
  - no active architecture-decision-backlog entry is required because the owner authorized resolution of the bounded question and the accepted ADR is recorded in the same package
unknown:
  - exact publication-rights status of third-party prose/dialogue/maps/media; this remains intentionally outside Issue #1121
conflicts: []
first_failure:
  marker: checkpoint_version_contract_mismatch
  evidence: CI run 31972496628 job 95226970159 on 6a1c93dca4d154f1c4d0dcbcf550cb2e242b314d reported checkpoint_version must be 1; GOVERNANCE_CONTRACT confirms version 1
rejected_hypotheses:
  - promote CrystalServer source material to native Oteryn authority
  - reuse legacy-canary authority name for a CrystalServer source
  - blend third-party fields into authoritative rows without explicit presentation-only precedence
  - place third-party reference snapshots inside the authoritative GameCatalog activation/profile lifecycle
  - execute source Lua or other source code to derive reference facts
  - treat archive extraction time or source metadata as current Oteryn freshness
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
  - command: architecture candidate exact-head CI/Agent Governance
    result: FAIL
    evidence: 6a1c93dca4d154f1c4d0dcbcf550cb2e242b314d; CI 31972496628 first failure is checkpoint_version mismatch and Agent Governance 31972496619 also failed on that generation
  - command: checkpoint contract inspection
    result: PASS
    evidence: docs/agents/GOVERNANCE_CONTRACT.json shared_checkpoint_contract.version is 1 and tools/agents/checkpoint.py validates against that declared version
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture-only documentation/contract task; no executable product or integration journey is changed
blockers: []
next_action: validate the repaired checkpoint and architecture package on the new exact PR head, then perform whole-diff self-review and merge gates
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: architecture review remains active on draft PR #1122 until exact-head gates pass
source_branch_evidence: PR #1122 open; source branch is the sole task delivery path
```

## Notes

This task was dispatched by `CONTENT-COORD` after the content audit proved that source/native authority is a gating dependency for source-driven Wiki and Player Companion expansion. The owner explicitly instructed this Architecture Review to resolve the bounded decision autonomously. That instruction authorizes selecting and recording the architecture outcome inside this repository, but does not grant runtime, production, external-repository or third-party-publication authority.
