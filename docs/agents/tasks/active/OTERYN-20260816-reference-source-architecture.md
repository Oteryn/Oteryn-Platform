---
task_id: OTERYN-20260816-reference-source-architecture
repository: blakinio/Oteryn-Platform
mode: architecture
task_kind: discovery
issue: 1121
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-content
status: investigating
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

- [ ] Re-read current architecture authority, ADR 0034, native Game Catalog content contract and Legacy Canary import contract from current `main`.
- [ ] Search for an existing accepted decision that already fully resolves the Issue; extend/reconcile rather than duplicate if one exists.
- [ ] Enumerate the smallest viable alternatives: reject/defer reference profiles, permit a bounded reference-only profile, or use a separate non-authoritative read-model boundary.
- [ ] For each alternative define identity, provenance, completeness/freshness, source conflict, profile separation, precedence and failure semantics.
- [ ] Preserve native Oteryn-v2 content authority and prohibit CrystalServer/third-party data from impersonating native or `legacy-canary` runtime authority.
- [ ] Keep executable gameplay parameters, availability/reachability and native canonical identity outside third-party reference authority.
- [ ] Keep third-party prose/dialogue/maps/media publication rights as a separate decision; do not bulk-copy them.
- [ ] Record the accepted/proposed architecture outcome using the repository decision-backlog/ADR process and provide explicit #1115 implementation handoffs only where warranted.
- [ ] No runtime/product code, migrations, deployment, production/staging mutation, external server/game repository access or owner-funded AI use.
- [ ] Run documentation/governance validation and exact-head whole-diff self-review before readiness.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260816-reference-source-architecture.md
  - docs/architecture/adr/0042-non-native-reference-content-boundary.md
  - docs/architecture/adr/README.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
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

The architecture worker has now claimed the exact durable decision/contract/focused-architecture paths before editing them. No runtime/product paths are owned by this task.

## Context checkpoint

```yaml
checkpoint_version: 2
updated_at: 2026-08-16T21:00:36Z
invocation_started_at: 2026-08-16T20:55:00Z
last_progress_at: 2026-08-16T21:00:36Z
head: c2dad69de9cdb1e027800aebd50850bee01c8904
branch: docs/issue-1121-reference-source-architecture
pr: 1122
status: investigating
phase: design
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
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
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
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/contracts/NON_NATIVE_REFERENCE_CONTENT_CONTRACT.md
  - docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md
proven:
  - Issue #1121 is open and labelled agent:ready
  - draft PR #1122 exists on docs/issue-1121-reference-source-architecture and live head was c2dad69de9cdb1e027800aebd50850bee01c8904 at takeover
  - scaffold repair head c2dad69de9cdb1e027800aebd50850bee01c8904 passed CI run 31971781327 and Agent Governance run 31971781324
  - protected main is f617120975cb1522cad87d74f8bea37f829b2b64 and matches PR #1122 base at takeover
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
  - highest accepted/currently allocated ADR prefix on main is 0041 and no open PR claims 0042
  - open related PRs #1116 and #1120 do not overlap the claimed architecture paths; #1122 is the sole open reference-source architecture PR
  - the task-required OTERYN_CONTENT_COMPLETION programme file is not on main; its candidate version was read from open draft PR #1116 and treated as non-authoritative programme evidence subordinate to Issue #1115 and merged audit handoff
  - no external server/game repository was accessed during takeover or analysis
derived:
  - ADR 0034 already rejects promoting CrystalServer-derived facts into native Oteryn gameplay authority
  - a remaining bounded decision exists only for non-native/non-executable reference use and presentation/tool consumption
  - a separate logical ReferenceContent read-model boundary is the least ambiguous way to permit reference use without giving GameCatalog authority-profile or activation semantics to third-party material
  - a new ADR is required because the selected boundary introduces a durable module/dependency/source-authority rule that outlives Issue #1121
unknown:
  - exact publication-rights status of third-party prose/dialogue/maps/media; this remains intentionally outside Issue #1121
conflicts: []
first_failure:
  marker: branch_pr_identity_omitted
  evidence: historical Agent Governance run 31971593663 job 95224784888; repaired on c2dad69de9cdb1e027800aebd50850bee01c8904
rejected_hypotheses:
  - promote CrystalServer source material to native Oteryn authority
  - reuse legacy-canary authority name for a CrystalServer source
  - blend third-party fields into authoritative rows without explicit presentation-only precedence
  - place third-party reference snapshots inside the authoritative GameCatalog activation/profile lifecycle
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260816-reference-source-architecture.md
validation:
  - command: fresh admission/ownership/uniqueness review
    result: PASS
    evidence: Issue #1121, PRs #1116/#1120/#1122, active task inventory, ADR inventory and open 0042 search
  - command: scaffold exact-head CI/Agent Governance
    result: PASS
    evidence: c2dad69de9cdb1e027800aebd50850bee01c8904; CI 31971781327 and Agent Governance 31971781324
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture-only documentation/contract task; no executable product or integration journey is changed
blockers: []
next_action: write the accepted ADR 0042 plus the non-native reference-content contract and focused authority/module/PlayerCompanion/native-contract reconciliations as one coherent architecture candidate
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: architecture review is active on draft PR #1122
source_branch_evidence: PR #1122 open; source branch is the sole task delivery path
```

## Notes

This task was dispatched by `CONTENT-COORD` after the content audit proved that source/native authority is a gating dependency for source-driven Wiki and Player Companion expansion. The owner explicitly instructed this Architecture Review to resolve the bounded decision autonomously. That instruction authorizes selecting and recording the architecture outcome inside this repository, but does not grant runtime, production, external-repository or third-party-publication authority.
