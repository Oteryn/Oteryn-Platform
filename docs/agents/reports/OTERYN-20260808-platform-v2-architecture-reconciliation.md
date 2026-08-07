# Oteryn Platform ↔ Oteryn-v2 Architecture Reconciliation Review

Date: 2026-08-08  
Repository under change: `blakinio/Oteryn-Platform`  
External repositories: read-only evidence only

## Verdict

`TRANSITIONAL / ARCHITECTURALLY SOUND CORE / CROSS-REPOSITORY RECONCILIATION REQUIRED`

The Platform modular-monolith core remains a sound basis. The material gap is integration semantics: current Canary-compatible SQL/IDs/session/protocol paths must be isolated from the target native Oteryn-v2 architecture so future modules do not inherit compatibility assumptions accidentally.

## Proven current state

- Oteryn Platform remains a Laravel modular monolith with explicit module boundaries.
- Current delivered Account/Character/PublicGameData integrations contain Canary numeric identifiers and contracted direct/shared-data access in compatibility paths.
- The current Gateway/Identity contract and Canary-compatible Game Session path are repository-delivered but do not prove production activation.
- Transitional native protocol artifacts exist in Platform and were designed before the later Oteryn-v2 ownership consolidation.
- ADR 0030 already establishes the native Character Portfolio rule: Platform composes/orchestrates while game-domain Character Authority owns canonical CharacterId/current ownership/lifecycle mutations.
- Read-only accepted Oteryn-v2 evidence establishes Platform AccountId/Identity/control-plane ownership and game-domain gameplay/protocol/character authority.

## Accepted reconciliation

ADR 0031 records the durable decision. The focused canonical model is `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md`.

The target split is:

```text
Oteryn Platform
├── Native Oteryn-v2 Integration
│   └── versioned commands / queries / events / projections / pre-admission contracts
└── Legacy Canary Compatibility
    └── Canary IDs / SQL / session / protocol adapters / migration bridges
```

Canary compatibility remains supported until separate cutover work proves replacement and rollback; it is not the native target model.

## Ownership summary

### Platform

- canonical `AccountId`;
- Identity/Auth/OAuth+PKCE/MFA/recovery and Platform sessions;
- one-time Game Login Ticket;
- World Registry/routing policy;
- Game Gateway ticket redemption and pre-admission control plane;
- Platform entitlements/business/commercial workflow;
- Bazaar auction/bid/wallet/commission saga state;
- portal/CMS/admin/support workflow and Platform-local read models.

### Oteryn-v2 game domain

- canonical `CharacterId`;
- current `AccountId <-> CharacterId` ownership;
- character lifecycle and native mutations;
- final gameplay admission and authoritative admitted-session/lease/fencing semantics;
- gameplay/world state and persistence;
- native `protocol-oteryn` gameplay semantics;
- authoritative gameplay analytics source facts.

## Key corrections

1. Shared/direct Canary SQL is compatibility-only; native v2 steady-state integration uses explicit contracts and separate persistence.
2. Platform caches/read models never prove current ownership or mutation authority.
3. Platform may carry protocol selection/routing metadata but does not own canonical native gameplay packet/state semantics.
4. `protocol-oteryn` is the target native gameplay family; Canary/Tibia protocol remains compatibility/reference only.
5. Public Game Data should move toward resilient events/snapshots/query contracts feeding Platform projections rather than synchronous arbitrary game-table coupling.
6. Game Analytics source facts belong to the runtime/game domain; Platform consumes approved projections/aggregates/read APIs.
7. Cross-system mutations use idempotent command/receipt/reconciliation semantics; distributed ACID is not assumed and timeout is not proof of outcome.

## Open PR classification snapshot

This table is dated operational review evidence only. It does not override each PR's live state.

| PR | Classification | Rationale at review time |
|---|---|---|
| #541 | `NEEDS_DECISION` | Documentation checkpoint around public-domain/password-recovery evidence; not part of native-v2 architecture. Requires live-state reconciliation before merge. |
| #405 | `FIX` | `PRODUCTION_PROVEN=false` remains safe, but portions of edge evidence predate later Cloudflare work; evidence should be refreshed/rebased rather than merged mechanically. |
| #391 | `NEEDS_DECISION` | Official Tibia Linux live-reference capability may remain bounded research/reference evidence only; it must not define native runtime architecture. |
| #338 | `NEEDS_DECISION` | Game Catalog 1.3 NPC/shop consumer may remain a Canary compatibility path; native catalogue ownership must not inherit Canary producer semantics automatically. |

No PR was closed solely because of this architecture review.

## Risks

### P0/P1

- dual integration model ambiguity;
- dual native gameplay-protocol authority;
- dual character ownership authority;
- Canary numeric-ID leakage into new native consumers;
- shared-database shortcuts for new native mutations;
- missing explicit projection/freshness/reconciliation contracts;
- web availability coupled directly to mutable game persistence;
- unversioned cross-repository behavior.

### P2

- fragmented correlation/tracing across Platform/Gateway/game;
- incomplete compatibility-adapter sunset inventory;
- mixed-version operational ambiguity;
- projection freshness drift.

## Deferred focused decisions

### P1

- exact Character command/query transport and receipt schemas;
- PublicGameData event/projection catalogue and freshness SLAs;
- World Registry/LiveOps runtime-status contract;
- Products/Entitlements game-delivery saga;
- Support/Moderation game-enforcement command contract;
- native Game Catalog/content ownership versus legacy Canary importers;
- exact Platform pre-admission → game-owned admitted-session/lease handoff.

### P2

- unified correlation/trace/security envelope;
- per-adapter Canary sunset/removal criteria;
- mixed-version contract-drift monitoring.

## Repository effects of this review

Architecture/documentation only. No Laravel runtime, database migration, Oteryn-v2/Canary write, protocol implementation, deployment or production mutation is part of this package.
