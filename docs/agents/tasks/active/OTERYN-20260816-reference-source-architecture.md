---
task_id: OTERYN-20260816-reference-source-architecture
repository: blakinio/Oteryn-Platform
mode: architecture
task_kind: discovery
issue: 1121
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-content
status: ready
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
modules:
  - Architecture Review
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
  - none for architecture investigation; any final owner-acceptance requirement must be recorded rather than inferred
cross_repository_tasks:
  - none; external server/game repository access is not authorized
```

The architecture worker must update `owned_paths` with the exact ADR/backlog/contract/programme files before editing them. This scaffold intentionally reserves only its task record and Issue/branch identity.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T20:48:00Z
head: UNKNOWN
branch: docs/issue-1121-reference-source-architecture
pr: none
status: ready
context_routes:
  - architecture
  - agent-governance
  - content
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260816-reference-source-architecture.md
proven:
  - Issue #1121 exists and is labelled agent:ready
  - ADR 0034 is accepted and assigns native gameplay-content authority to Oteryn-v2 while Platform owns catalogue lifecycle
  - ADR 0034 keeps Legacy Canary Compatibility importers as explicit anti-corruption adapters
  - OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT forbids co-authoritative source blending and allows only presentation metadata supplementation under explicit provenance/precedence
  - owner-supplied CrystalServer archive is source material only under #1115 and is not native authority
  - architecture programme currently reports no active task in its durable queue snapshot
  - no external server/game repository was accessed while scaffolding this task
derived:
  - ADR 0034 already rejects promoting CrystalServer-derived facts into native Oteryn gameplay authority
  - a remaining bounded decision exists only for non-native/non-executable reference use and presentation/tool consumption
unknown:
  - whether a reference-only profile belongs inside GameCatalog, a separate read model, or should be rejected/deferred
  - exact allowed public Wiki/PlayerCompanion uses of third-party structured facts
  - exact publication-rights status of third-party prose/dialogue/maps/media
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - promote CrystalServer source material to native Oteryn authority
  - reuse legacy-canary authority name for a CrystalServer source
  - blend third-party fields into authoritative rows without explicit presentation-only precedence
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260816-reference-source-architecture.md
validation:
  - command: coordinator live-state and authority preflight
    result: PASS
    evidence: Issue #1121 plus current ADR 0034 and Game Catalog contracts
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture scaffold only; no executable product path changed
blockers:
  - none
next_action: claim exact architecture decision/backlog/document paths, inspect current architecture decision registry for overlap, and produce the smallest evidence-backed architecture candidate for Issue #1121
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: architecture review task is READY and has not started implementation/review work
source_branch_evidence: pending
```

## Notes

This task was dispatched by `CONTENT-COORD` after the content audit proved that source/native authority is a gating dependency for source-driven Wiki and Player Companion expansion. The worker must not reinterpret this scaffold as permission for runtime implementation or external-repository access.
