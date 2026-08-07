# ADR 0031 — Native Oteryn-v2 integration and legacy Canary compatibility boundary

## Status

Accepted — 2026-08-08

- Decision owner: repository owner
- Decision Issue: #863
- Applies to: Oteryn Platform integration ownership, native-v2 versus Canary compatibility separation, game admission/control-plane responsibilities, native gameplay protocol ownership, cross-system persistence boundaries, public game-data projections and game-analytics consumption
- Does not authorize: Laravel/runtime implementation, database migration, Oteryn-v2 or Canary repository writes, wire-protocol implementation, deployment, production activation or removal of existing compatibility paths

## Context

Oteryn Platform is architecturally sound as a Laravel modular monolith, but its delivered integration history contains two different models that must not be treated as one architecture:

1. **current Canary compatibility** — direct/shared-schema reads, operation-specific Canary writes, Canary numeric account/player identifiers, Canary-compatible Game Session paths and compatibility protocol adapters;
2. **target native Oteryn-v2 integration** — canonical Platform and game-domain identities, explicit service contracts, separate authority/persistence, game-owned authoritative gameplay admission/session state and native `protocol-oteryn` gameplay semantics.

Current compatibility code is real delivered behavior and must not be rewritten out of history. It is also not permission to make Canary table shape, Canary IDs or Canary wire/session semantics permanent dependencies of new native Platform capabilities.

ADR 0030 already establishes the same principle for character portfolio and lifecycle integration: Platform composes/orchestrates, while Oteryn-v2 Character Authority remains authoritative for canonical `CharacterId`, current `AccountId <-> CharacterId` ownership and native character mutations.

The Platform therefore needs one durable integration boundary that tells agents which parts are compatibility state and which parts define the target native ecosystem.

## Decision

### 1. Keep the Platform modular monolith

The integration reconciliation does not justify a Platform rewrite or a default microservice decomposition.

Oteryn Platform remains a Laravel modular monolith with a separately deployable Game Gateway where already accepted. New services require measured independent scaling, security isolation, lifecycle or ownership need.

### 2. Separate native integration from legacy compatibility

The target conceptual split is:

```text
Oteryn Platform
├── Native Oteryn-v2 Integration
│   ├── versioned commands
│   ├── versioned queries
│   ├── integration events
│   ├── projections/read models
│   └── explicit admission/control-plane contracts
│
└── Legacy Canary Compatibility
    ├── Canary numeric-ID mappings
    ├── direct/read-only SQL contracts
    ├── operation-specific legacy writes
    ├── Canary-compatible session/protocol adapters
    └── migration/rollback bridges
```

`Legacy Canary Compatibility` is an anti-corruption/migration boundary, not the native target domain model.

Existing Canary behavior may remain until separately authorized cutover criteria are proven. New native consumers must not adopt compatibility identifiers or table/protocol assumptions as canonical design.

### 3. Platform authority

Oteryn Platform is authoritative for the Platform/application domain, including:

- canonical native `AccountId` identity and issuance;
- Identity authentication/security policy;
- OAuth Authorization Code + PKCE and Platform sessions;
- MFA, recovery and Platform account-security lifecycle;
- one-time Game Login Ticket lifecycle;
- World Registry identity, policy and routing/control-plane decisions within the accepted Platform boundary;
- Game Gateway ticket redemption and pre-admission orchestration;
- Platform-owned entitlements, commercial/business state and customer-facing workflow state;
- Character Bazaar auction/bid/wallet/commission saga state;
- portal UX, CMS, notifications, administration, support workflow and Platform-local projections/caches.

Platform-issued material authorizes an attempt to enter the game flow; it does not replace final game-domain ownership/admission validation.

### 4. Oteryn-v2 game-domain authority

For native integration, the game domain remains authoritative for:

- canonical `CharacterId` identity and issuance;
- current `AccountId <-> CharacterId` ownership;
- character lifecycle and game-domain mutation outcomes;
- authoritative logical world membership of a character;
- final gameplay admission and authoritative gameplay-session/lease/fencing semantics;
- gameplay/world state, combat, movement, inventory, loot, quests and economy state;
- native game persistence and its physical schema;
- native `protocol-oteryn` gameplay wire semantics, framing/serialization evolution and game-state reconciliation behavior;
- authoritative gameplay telemetry/source facts used for Game Analytics.

A Platform cache, projection, workflow row, ticket or browser claim never becomes proof of current character ownership or current gameplay authority.

### 5. Admission/control-plane flow

The target responsibility chain is:

```text
Rust client
  -> Platform Identity OAuth + PKCE
  -> one-time Game Login Ticket
  -> Platform Game Gateway
  -> authoritative ticket redemption
  -> World Registry routing/policy
  -> bounded pre-admission material
  -> Oteryn-v2 authoritative ownership/admission/lease validation
  -> game-owned admitted session
  -> protocol-oteryn gameplay
```

The exact credential/session envelope remains versioned contract work. The enduring ownership rule is that Platform controls identity and pre-admission orchestration while the game domain controls final gameplay admission and the authoritative admitted gameplay session.

Do not create:

- a second Identity authority in the game server;
- a second reusable password-login path for native gameplay;
- direct OAuth access/refresh-token authentication in the game server;
- post-admission protocol downgrade/fallback that bypasses the accepted admission contract.

### 6. Native gameplay protocol ownership

The target native runtime uses `protocol-oteryn` between the Rust client and the Rust game server.

Platform may bind and transport **selection metadata** required for secure routing/pre-admission, such as a supported protocol family/version/capability identity. Platform does not own gameplay packet semantics, state-reconciliation semantics or the canonical gameplay IDL merely because Gateway carries the selected tuple.

Canary/Tibia gameplay protocols remain compatibility/reference mechanisms only. They are not a second target native runtime family.

The initial native design retains the useful ADR 0011 simplification: one initial native protocol version and no speculative native profile catalogue. Future incompatible native variants require a new explicit decision and contract revision.

This ADR supersedes ADR 0010 and ADR 0011 as the authority for **native gameplay protocol ownership and target family-selection semantics**. Historical implementation evidence and the no-profile rationale remain useful where they do not conflict with this ADR.

### 7. Separate persistence; no native steady-state shared-table dependency

Target native integration must not require Platform application credentials to know or mutate Oteryn-v2 physical gameplay tables.

Steady-state native boundaries use explicit contracts:

```text
Platform persistence             Oteryn-v2 persistence
        |                                 |
        +---- commands / queries ---------+
        +---- events / projections -------+
```

Direct/shared SQL may remain only inside explicitly named Canary compatibility or migration contracts with narrow privileges, compatibility windows, rollback and removal criteria.

No new native Platform capability may infer authority merely from game-table shape or directly update native game-owned rows.

### 8. Character mutations use game-owned commands

Native character creation, rename, deletion/restore/finalization, world transfer and ownership transfer cross the boundary through versioned game-owned commands or equivalent explicit service contracts.

Cross-system mutations require stable operation/idempotency identity, bounded durable outcomes/receipts, retry/reconciliation semantics and audit evidence proportional to risk.

No distributed ACID transaction is assumed. Timeout or transport failure is not proof of success or failure; the orchestrating Platform saga reconciles against authoritative operation/ownership state.

ADR 0030 remains the focused Platform owner for Account Center / Character Portfolio composition.

### 9. Public Game Data uses projections/query contracts

For native v2, public and authenticated read surfaces should consume bounded projections, snapshots, integration events or dedicated query contracts rather than arbitrary gameplay-table reads.

Conceptual flow:

```text
Oteryn-v2 authoritative state
  -> event / snapshot / query contract
  -> Platform projection/read model
  -> cache/API/SSR/CDN consumers
```

Every projection defines source, revision/observation marker, freshness, stale behavior, unavailable behavior, privacy classification and reconciliation strategy.

This prevents normal website availability from depending directly on synchronous access to mutable game runtime persistence.

Current direct Canary read services remain valid only for their declared compatibility scope until replaced.

### 10. Game Analytics source/consumer split

The game domain owns authoritative gameplay telemetry/source facts because only the runtime can observe complete game-state transitions reliably.

Platform consumes explicitly approved:

- aggregates;
- projections;
- alerts;
- analytics read APIs;
- operator/public read models where authorized.

Platform must not reconstruct authoritative gameplay analytics by treating raw gameplay database tables as an undocumented analytics API.

### 11. Cross-repository contract discipline

Each material Platform ↔ Oteryn-v2 contract must define, as applicable:

- producer and consumer ownership;
- canonical typed identities;
- authentication/service identity and audience;
- API/session/protocol versions as distinct concepts;
- idempotency, ordering and replay rules;
- schemas/limits and typed failure vocabulary;
- projection freshness/revision semantics;
- observability/correlation and redaction;
- compatibility matrix;
- rollout, rollback and mixed-version behavior;
- deterministic contract tests/fixtures and their canonical owner.

One repository may consume or reference another repository's accepted contract; it must not silently create a second authority for the same semantics.

### 12. Migration and removal policy

Acceptance of the target architecture does **not** delete existing Canary compatibility.

Every compatibility path is removed only through a later bounded task that proves:

- native replacement capability;
- exact producer/consumer compatibility;
- migration/backfill correctness where identifiers/data change;
- rollback path;
- required staging/E2E evidence;
- no active consumer still depends on the legacy path;
- production activation authority when applicable.

Until then, current and target states must be labelled explicitly.

## Consequences

### Positive

- new Platform capabilities no longer inherit Canary schema/ID/protocol coupling by accident;
- identity, business/control-plane and gameplay authorities are unambiguous;
- native persistence can evolve independently from Platform persistence;
- website read availability can be decoupled from mutable game runtime storage;
- protocol ownership follows the component that owns authoritative gameplay semantics;
- migration can proceed incrementally without pretending legacy compatibility has already disappeared.

### Costs

- several current compatibility contracts remain in place during transition;
- new projection/command/event contracts and reconciliation tooling are required;
- current disabled native producer/session artifacts must be reconciled before activation;
- explicit compatibility mapping and sunset criteria add migration work;
- cross-repository contract testing becomes mandatory for important changes.

## Explicitly deferred decisions

This ADR does not invent the following details:

- exact HTTP/RPC/message-bus technologies for native Platform-v2 contracts;
- exact character-portfolio transport/cache TTL/capability-code vocabulary;
- exact entitlement-to-game grant delivery contract;
- exact support/moderation enforcement command contract;
- exact public game-data event catalogue and per-projection freshness SLA;
- exact Game Analytics schemas/retention tiers;
- exact World Registry runtime-status projection contract;
- exact Canary adapter sunset dates;
- exact cross-system trace/security envelope beyond the stated invariants.

These remain focused follow-up architecture decisions.

## Rejected alternatives

- rebuild Oteryn Platform as microservices solely to solve the compatibility transition;
- keep shared/direct game SQL as the default native integration model;
- reuse `canary_account_id` / `canary_player_id` as canonical native identities;
- make Platform a second authoritative owner of characters or gameplay sessions;
- keep Platform as canonical owner of gameplay wire semantics because Gateway carries protocol metadata;
- keep Canary/Tibia gameplay protocol as a permanent equal target family in native Oteryn-v2 architecture;
- delete compatibility code before replacement/cutover/rollback evidence exists;
- derive native game policy from stale row counts or presentation projections.

## References

- Issue #863
- `docs/architecture/ARCHITECTURE_AUTHORITY.md`
- `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md`
- ADR 0001 — Laravel modular monolith
- ADR 0028 — Platform AccountId cross-boundary identity
- ADR 0029 — Platform WorldId/ChannelId topology identity
- ADR 0030 — Native Character Portfolio / Account Center v2
- superseded ADR 0010 — native gameplay protocol selection
- superseded ADR 0011 — one native protocol version/no profile catalogue
- `docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md` — current delivered Gateway/Identity compatibility evidence
- `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md` — transitional disabled producer/contract evidence subordinate to this ADR for target ownership
- read-only Oteryn-v2 accepted Character Authority and cross-repository contract evidence
