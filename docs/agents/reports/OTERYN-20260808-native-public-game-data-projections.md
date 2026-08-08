# Native Public Game Data projection architecture review — 2026-08-08

## Result

`FOCUSED CONSUMER BOUNDARY RESOLVED — RUNTIME IMPLEMENTATION / PRODUCER / STORAGE / CUTOVER DEFERRED`

## Evidence reviewed

- accepted ADR 0031 native Oteryn-v2 integration boundary;
- `docs/architecture/DATA_OWNERSHIP.md`;
- `docs/architecture/MODULE_CATALOG.md`;
- `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md`;
- accepted `docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`;
- delivered `app/PublicGameData/CanaryGameDataRepository.php`;
- delivered `app/PublicGameData/PublicCharacterProfileService.php`;
- delivered `app/PublicGameData/GuildIndexQuery.php`;
- related current-surface evidence Issue #487;
- character lifecycle/product Issues #277/#317/#319/#320;
- Game Catalog programme #330.

## Current delivered state

`PublicGameData` is a real delivered module, but its present game-data authority path is compatibility-oriented:

- highscores query Canary `players` directly;
- character profile facts join Canary player/guild tables and read houses/deaths/kills;
- public online state/list reads Canary `cluster_sessions`;
- guild index/detail reads Canary guild/membership/player tables;
- current `PublicCharacterProfileService` composes Canary game facts with Platform-owned Identity and `CharacterProfilePreference` privacy/presentation state.

This delivered behavior remains valid for the current Legacy Canary Compatibility path. It must not be mistaken for the native Oteryn-v2 steady-state integration model.

## Architecture gap

ADR 0031 already requires native game facts to cross by explicit versioned contracts rather than shared tables or cross-system SQL. The focused runtime-status contract already defines world/channel runtime observation, readiness, freshness and aggregate capacity/player-count semantics.

The remaining PublicGameData families lacked one canonical consumer contract for:

- source ownership and stable identities;
- event/snapshot/query evidence semantics;
- idempotent repeated delivery and ordering;
- fresh/stale/unavailable/invalid versus empty/not-found truthfulness;
- website last-known-good behavior independent of synchronous runtime availability;
- tombstones and rename/delete/restore/transfer reconciliation;
- CharacterProfiles/privacy composition;
- rebuild generations, high-watermarks, gaps, poison evidence and recovery;
- per-family Legacy Canary -> native cutover and rollback.

## Resolution

`docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md` defines the Platform consumer semantics for five native projection families:

1. character public facts and search;
2. highscores/rankings;
3. character activity (deaths/kills);
4. guild public facts/membership;
5. individual character presence.

It explicitly excludes and routes:

- world/channel runtime health/readiness and aggregate capacity/player counts to `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`;
- Game Catalog/content facts to their owning catalogue/content contracts;
- authenticated Character Portfolio/current ownership to Accounts/Characters/Character Authority contracts;
- Platform CharacterProfiles/Identity privacy preferences to Platform presentation policy.

## Key accepted semantics

### Native authority remains game-owned

Oteryn-v2 remains source authority for native game facts. Platform projection storage is disposable read-model state and may be rebuilt/replaced/rolled back without writing derived facts back into the game domain.

### Stable identity over display names

Character projections use canonical `CharacterId`; `WorldId`/`ChannelId` are used for applicable scopes. Canary numeric IDs remain compatibility-only.

Guild projection implementation is explicitly gated on an accepted stable game-owned canonical guild identity. Mutable guild name and Canary numeric guild ID cannot become native identity by convenience. The exact native guild identifier representation remains `UNKNOWN` and is intentionally not invented by this Platform task.

### Privacy/presentation remains a Platform overlay

The current Platform pattern of composing game facts with Platform-owned privacy/presentation preferences is preserved conceptually. Native producer data cannot force exposure that Platform privacy policy denies, while Platform privacy permission cannot fabricate a missing game fact.

Current `canary_account_id` / `canary_player_id` preference associations remain compatibility state and require canonical-identity migration before native cutover.

### Runtime status is not duplicated

Individual character presence may be projected for public online-list/profile presentation. World/channel health, readiness, aggregate player counts and capacity remain owned by the dedicated runtime-status contract. Aggregate counts cannot be expanded into invented individual characters, and individual presence cannot establish admission readiness.

### Ordinary website reads are projection reads

Public HTTP/API/SSR reads consume Platform last-known-good projection state. They do not synchronously call the game runtime as a normal fallback. Stale/unavailable/invalid evidence must remain distinguishable from authoritative empty/zero/not-found state.

### Rebuild and reconciliation are first-class

The contract defines generation-based rebuild from authoritative baseline/snapshot plus replay/tail where available, safe high-watermarks, gap detection, targeted reconciliation, quarantine/poison behavior and bounded rollback to the prior known-good Platform generation.

### Migration is per family

Legacy Canary and native Oteryn-v2 provenance must remain explicit. Shadow/diff is allowed, but field-by-field silent authority mixing is forbidden. Each family cuts over only after producer contract, canonical identity, idempotency, rebuild/reconciliation, freshness, privacy and rollback evidence pass.

## Deliberately deferred details

The contract does not select or invent:

- Oteryn-v2 event/query/snapshot names or wire schemas;
- broker/transport;
- producer authentication mechanism;
- native canonical guild identifier representation;
- ranking category/season/ruleset semantics;
- numeric freshness/SLA values;
- replay retention window;
- Platform projection tables/indexes/storage engine;
- Laravel worker/job implementation;
- cache/CDN invalidation mechanism;
- staging/production cutover order.

These remain implementation/producer authority and do not permit fallback to native shared SQL.

## Overlap review

- Issue #487 remains owner of current delivered public-surface browser/evidence gaps; no test/evidence scope is duplicated.
- Issues #277/#317/#319/#320 remain owners of character lifecycle/product operations; the new contract only defines how authoritative outcomes affect projections.
- `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md` remains authoritative for runtime-status/readiness/aggregate-capacity semantics.
- Game Catalog programme #330 remains authoritative for catalogue/content snapshot semantics.
- No open PR or Issue was found owning this exact generic native PublicGameData projection/reconciliation boundary before Issue #902.

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: pending-final-task-checkpoint
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - five projection families are explicitly catalogued
    - runtime-status and Game Catalog authority are referenced rather than duplicated
    - current Canary SQL/Redis paths are classified compatibility-only
    - stable identity, privacy overlay, freshness, rebuild, reconciliation and per-family rollback rules are explicit
    - native guild identity uncertainty is explicit and implementation-gating rather than silently filled with Canary/name identity
    - no runtime/schema/worker/workflow/deployment/external-repository path is changed
```

## E2E

`NOT_APPLICABLE` — this task changes architecture/documentation only. It creates no executable producer, consumer worker, database schema, API route, browser behavior, staging deployment or production cutover.

## Nonclaims

This review does not prove or authorize:

- an Oteryn-v2 producer;
- Platform projection persistence/workers;
- canonical guild identity implementation;
- migration of CharacterProfiles from legacy identifiers;
- replacement/removal of current Canary readers;
- native producer/consumer E2E;
- staging or production cutover;
- any external-repository write.
