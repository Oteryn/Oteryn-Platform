---
task_id: OTERYN-20260817-dynamic-semantic-atlas-project
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
search_first:
  - dynamic semantic atlas DYN-ATLAS
  - atlas hunt advisor player companion
  - open Atlas PR ownership
optional_reads:
  - docs/agents/PROJECT_STATE.md
---

# OTERYN-20260817-dynamic-semantic-atlas-project

## Goal

Persist the agreed Dynamic Semantic Atlas target architecture, phased programme and thin DYN-ATLAS-001 execution overlay without changing runtime code or weakening current Game/Atlas/Platform authority.

## Acceptance criteria

- [ ] Preserve the main Atlas as the product being evolved rather than defining a parallel replacement map.
- [ ] Record Atlas as a derived semantic projection/read model, never a second canonical World/Content authority.
- [ ] Record future capability families for NPC/shop/monster/loot knowledge, Hunt Intelligence, bounty/task planning and authorized live-state.
- [ ] Preserve `PlayerCompanion` ownership of personalized hunt guidance/progression/recommendations.
- [ ] Preserve privacy-first player-position rules and default-deny live/public disclosure.
- [ ] Record Svelte/TypeScript + PixiJS/WebGL2 as a recommended proof candidate, not an irreversible framework decision.
- [ ] Keep physical serializer, compression, chunk size/floor packing and canonical coordinate profile evidence-gated; FlatBuffers is not frozen.
- [ ] Keep DYN-ATLAS-001 limited to a static Semantic Thais Z7 proof.
- [ ] Provide a thin execution prompt that refuses to implement Atlas runtime in Platform merely because the actual Atlas repository is unavailable.
- [ ] Run applicable documentation/governance validation and exact-head full-diff self-review before merge.
- [ ] Merge only if current-head required checks and merge gate pass.

## Ownership

```yaml
owned_paths:
  - docs/architecture/oteryn-dynamic-semantic-atlas.md
  - docs/maps/oteryn-dynamic-semantic-atlas-program.md
  - docs/maps/oteryn-dynamic-semantic-atlas-execution-prompt.md
  - docs/agents/tasks/active/OTERYN-20260817-dynamic-semantic-atlas-project.md
modules:
  - Architecture
  - Atlas
  - PlayerCompanion
dependencies:
  - Platform ADR 0041
  - PLAYER_COMPANION_ARCHITECTURE.md
  - read-only evidence from blakinio/Oteryn-v2@5577f6fc7c1f7ddef482f0f7b08039047704e36b
blockers:
  - none for this documentation task
cross_repository_tasks:
  - none; Oteryn-v2 is read-only evidence and receives no writes in this task
```

No existing open Atlas PR or active task owned these paths at preflight. Open PR #1138 owns repository-migration programme/evaluation paths only; #1116/#1120 own content-programme coordination paths; #338 owns inactive Game Catalog schema 1.3 NPC/shop consumer paths. This task does not modify those paths.

## Evidence basis

### PROVEN

- Platform `main` preflight SHA: `fcafc20bc9705ca92256fdddc7433bcc3d191c40`.
- Platform ADR 0041 makes future Oteryn-Atlas an independently releasable derived browser/read-model product and keeps Game canonical World/Content + Atlas export ownership.
- Platform `PLAYER_COMPANION_ARCHITECTURE.md` already owns `HuntAdvisor`, `ProgressTracker` and personalized `Recommendations`.
- Oteryn-v2 current `main` preflight SHA: `5577f6fc7c1f7ddef482f0f7b08039047704e36b`.
- Oteryn-v2 contains `docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md`; the contract is artifact-first, immutable and default-deny public-safe.
- Oteryn-v2 physical-profile readiness evidence reports `EVIDENCE_GAP` and explicitly leaves serializer/chunk-size decisions open pending canonical spatial/coordinate authority.
- Oteryn-v2 ANL-02 is a candidate/nonbinding analytics contract that provides useful design evidence for versioned hunt/session, XP/profit and world/spawn analytics but is not promoted by this task to accepted Game authority.
- Oteryn-v2 social presence baseline treats exact placement as non-public and privacy/consent controlled.

### DERIVED

- Dynamic Atlas can become the spatial surface for PlayerCompanion HuntAdvisor/bounty planning without moving recommendation or gameplay authority into Atlas.
- Static Game exports, semantic knowledge, owner-private PlayerCompanion state and authorized live overlays need separate data planes.
- Svelte/TypeScript + PixiJS/WebGL2 is the strongest current browser proof candidate, but current Game contracts intentionally leave implementation/physical profile details open.

### UNKNOWN / deferred

- final canonical Game coordinate/floor/stack/anchor profile;
- final Game -> Atlas serializer/compression/chunk packing;
- exact future Oteryn-Atlas repository/execution coordinate if physical extraction is not yet available;
- final Game public allowlist for loot/NPC interactions;
- accepted producer coverage for decision-grade GameAnalytics HuntAdvisor metrics;
- future live Atlas schema and web-to-Game command contract.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-17T21:50:00Z
head: fcafc20bc9705ca92256fdddc7433bcc3d191c40
branch: docs/oteryn-20260817-dynamic-semantic-atlas
pr: none
status: implementing
context_routes:
  - architecture
  - agent-governance
owned_paths:
  - docs/architecture/oteryn-dynamic-semantic-atlas.md
  - docs/maps/oteryn-dynamic-semantic-atlas-program.md
  - docs/maps/oteryn-dynamic-semantic-atlas-execution-prompt.md
  - docs/agents/tasks/active/OTERYN-20260817-dynamic-semantic-atlas-project.md
proven:
  - Platform main preflight is fcafc20bc9705ca92256fdddc7433bcc3d191c40
  - no active task or open PR owns the Dynamic Atlas target paths
  - ADR 0041 preserves Game canonical authority and derived independent Atlas ownership
  - PlayerCompanion owns hunt guidance progress tracking and personalized recommendations
  - Oteryn-v2 main evidence is pinned at 5577f6fc7c1f7ddef482f0f7b08039047704e36b
  - Oteryn-v2 physical-profile evidence keeps serializer chunk size and coordinate details evidence-gated
derived:
  - one semantic Atlas foundation can support static map knowledge personalized hunt planning and later authorized live overlays without becoming gameplay authority
unknown:
  - exact future Oteryn-Atlas runtime repository availability for DYN-ATLAS-001 implementation
  - final Game canonical spatial coordinate profile and first executable physical profile
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - make Atlas a second canonical semantic World model
  - freeze FlatBuffers from architecture discussion without the Game physical-profile gate
  - expose all online players as public exact map positions
  - use Game Gateway as a generic browser gameplay mutation API
  - implement Atlas runtime inside Platform solely because future Oteryn-Atlas is unavailable
changed_paths:
  - docs/architecture/oteryn-dynamic-semantic-atlas.md
  - docs/maps/oteryn-dynamic-semantic-atlas-program.md
  - docs/maps/oteryn-dynamic-semantic-atlas-execution-prompt.md
  - docs/agents/tasks/active/OTERYN-20260817-dynamic-semantic-atlas-project.md
validation:
  - command: documentation/governance validation
    result: NOT_RUN
    evidence: implementation commit not yet created
  - command: exact-head full-diff self-review
    result: NOT_RUN
    evidence: implementation commit not yet created
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: documentation architecture/programme/prompt task only; no executable product path changes
blockers:
  - none
next_action: create the documentation commit and open the draft PR for exact-head validation
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository documentation PR; delete after successful squash merge
source_branch_evidence: pending merge
```

## Notes

This task intentionally writes only Oteryn-Platform documentation. Cross-repository Game implementation remains separately authorized work.
