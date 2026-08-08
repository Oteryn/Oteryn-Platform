---
task_id: OTERYN-20260808-native-public-game-data-projections
repository: blakinio/Oteryn-Platform
project_lane: oteryn-platform-content
issue: 902
status: completed
architecture_pr: 903
merge_sha: c9b99cbd3e38dd4c17b211a81891e8ecf5303af1
---

# OTERYN-20260808 native PublicGameData projections — closeout

## Terminal result

`DONE — NATIVE PUBLIC GAME DATA PROJECTION BOUNDARY ACCEPTED ON MAIN`

PR #903 was squash-merged to protected `main` as `c9b99cbd3e38dd4c17b211a81891e8ecf5303af1` after exact-head review and CI.

## Accepted boundary

`docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md` now defines Platform consumer semantics for:

1. character public facts/search;
2. highscores/rankings;
3. deaths/kill activity;
4. guild public facts/membership;
5. individual character presence.

Oteryn-v2 remains authoritative for native game facts. Platform owns rebuildable public read models, search/pagination, truthful freshness/degraded presentation and CharacterProfiles/Identity privacy overlays.

World/channel runtime health/readiness and aggregate capacity/player-count facts remain under `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`. Game Catalog/content facts remain under their catalogue/content authority.

Current Canary SQL/Redis reads remain `Legacy Canary Compatibility`; native cutover is per projection family, provenance-tagged, rebuildable and reversible without silent field-by-field mixing of authorities.

## Important invariants

- native CharacterId / applicable WorldId + ChannelId are canonical; Canary numeric IDs are compatibility-only;
- native guild implementation is gated on a stable game-owned canonical guild identity and cannot use mutable guild name or Canary numeric ID as native authority;
- ordinary website reads use Platform projection state rather than synchronous runtime fallback;
- stale/unavailable/invalid evidence is distinct from empty/zero/not-found;
- at-least-once/repeated delivery is idempotent and old/out-of-order revisions cannot overwrite newer state;
- rename/delete/restore/transfer/tombstone effects reconcile affected indexes/projections by stable identity;
- Platform privacy may further restrict fresh game facts but cannot fabricate missing game facts;
- rebuild uses generation/baseline plus replay/tail or another bounded authoritative reconciliation path;
- high-watermark gaps and poison records become explicit stale/reconciling/quarantine state rather than being silently skipped;
- rollback switches the Platform read source/generation and never writes derived projection state into Oteryn-v2.

## Exact-head validation

Final PR #903 head: `e83db9b4302cf70f9f3b443db29a87d42944db5e`.

All selected workflows passed:

- Agent Governance — `31250583095`;
- CI — `31250583088`;
- Native protocol contract — `31250583104`;
- Native protocol contract audits — `31250583099`;
- Game Auth Ticket Concurrency — `31250583101`;
- Edge Security Emulation — `31250583084`;
- Platform DB Outage Validation — `31250583087`;
- Phase 7 Production-Like Validation — `31250583118`.

The prior generation failed only because the task checkpoint used unsupported result label `PASS_PARTIAL`; that governance vocabulary defect was repaired and the complete final generation passed.

Final review state:

- changed paths: exactly three owned documentation/task/report paths;
- `behind_by=0` before merge;
- unresolved material findings: 0;
- unresolved review threads: 0;
- runtime/browser E2E: `NOT_APPLICABLE` because no executable producer, worker, schema, public route or cutover was implemented.

## Deferred implementation authority

Still intentionally `UNKNOWN`/deferred:

- exact Oteryn-v2 event/query/snapshot schemas and transport;
- broker/delivery infrastructure;
- native canonical guild identifier representation;
- numeric per-family freshness/SLA values;
- Platform projection tables/indexes/workers;
- replay retention window;
- cache/CDN invalidation implementation;
- staging/production cutover order.

No Oteryn-v2, Canary, runtime, database, workflow, deployment or production mutation occurred.

## Final checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T11:34:00+02:00
status: completed
phase: closeout
architecture_pr: 903
architecture_merge_sha: c9b99cbd3e38dd4c17b211a81891e8ecf5303af1
final_validated_head: e83db9b4302cf70f9f3b443db29a87d42944db5e
validation:
  - command: Agent Governance 31250583095
    result: PASS
  - command: CI 31250583088
    result: PASS
  - command: Native protocol contract 31250583104
    result: PASS
  - command: Native protocol contract audits 31250583099
    result: PASS
  - command: Game Auth Ticket Concurrency 31250583101
    result: PASS
  - command: Edge Security Emulation 31250583084
    result: PASS
  - command: Platform DB Outage Validation 31250583087
    result: PASS
  - command: Phase 7 Production-Like Validation 31250583118
    result: PASS
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/documentation-only task
blockers: []
next_action: none
```
