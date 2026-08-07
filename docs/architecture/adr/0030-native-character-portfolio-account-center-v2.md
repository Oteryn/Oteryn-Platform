# ADR 0030 — Native Character Portfolio and Account Center v2 boundary

## Status

Proposed — 2026-08-08

- Decision owner: repository owner
- Decision Issue: #857
- Coordination ID: `OTV2-CHARACTER-LIFECYCLE-BOUNDARY`
- Applies to: authenticated Account Center character portfolio composition, native character identity references, Platform presentation/privacy preferences, character-command orchestration, capability/freshness semantics and Canary compatibility migration direction
- Does not authorize: Laravel runtime changes, database migrations, Oteryn-v2/Canary writes, protocol wire-format changes, deployment or production activation

## Context

Oteryn Platform already has a delivered authenticated Account Center. Its current `AccountOverviewReadModel` is valid for the Canary compatibility path, but it resolves `IdentityCanaryAccount`, uses numeric `canary_account_id`, queries `CanaryGameDataRepository::activeCharactersForAccount()` and derives character-creation availability from a Platform-local `CHARACTER_LIMIT = 10`.

Platform also owns `character_profile_preferences`. The current persistence model identifies the game character with numeric `canary_player_id`. Those records are valid current compatibility state, but the numeric Canary identifier is not the native Oteryn-v2 character identity.

Accepted architecture has since established:

- ADR 0001: keep the Laravel modular monolith unless measured evidence justifies extraction;
- ADR 0008: Account Center is a distinct portal application surface; its statement that Account Overview was not delivered is historical evidence from its acceptance date, not current implementation truth;
- ADR 0025: PlayerCompanion is a bounded Platform module and must not become character/game-state authority;
- ADR 0028: Platform owns and issues canonical native `AccountId`; Canary account IDs are legacy compatibility state;
- ADR 0029: Platform owns and issues canonical `WorldId`/`ChannelId`, while game-domain state decides the character's current logical world membership;
- Oteryn-v2 ADR-0012 / merged PR #90: Character Authority owns canonical `CharacterId`, authoritative current `AccountId <-> CharacterId` ownership and native create/rename/delete/restore/world-transfer/account-transfer mutations; Platform reads are authorized projections and Platform direct writes to native character tables are not the steady-state design.

The Platform therefore needs a durable native consumer boundary before PlayerCompanion, PlatformAPI and future account features copy Canary-specific identifiers or policy inference into new code.

## Options

### Option A — Accounts-owned Character Portfolio application boundary — recommended

Keep Character Portfolio inside the modular monolith as an `Accounts` application/read boundary used by Account Center.

- `Accounts` owns authenticated portfolio composition and presentation-ready account state.
- `Characters` owns Platform orchestration of explicitly approved character commands.
- Oteryn-v2 Character Authority remains authoritative for character identity, ownership, lifecycle and native mutation results.
- `PublicGameData` remains the owner of public game-data projections; it may continue to provide general authorized game facts where its existing contracts apply, but it does not own Account Center composition or prove character ownership.
- `CharacterProfiles` is formally classified as the Platform-owned presentation/privacy preference subdomain and never becomes a second character authority.
- No separate service or deployment unit is introduced.

### Option B — PublicGameData-owned authenticated portfolio

Expand `PublicGameData` to own both public projections and the authenticated owner portfolio.

This reduces named boundaries but mixes public-read responsibilities with private ownership-sensitive Account Center composition and command-preparation semantics.

### Option C — standalone CharacterPortfolio module/service

Create a new top-level module or deployable service for portfolio reads, capabilities and orchestration.

This provides naming isolation but introduces another domain/deployment boundary before independent scaling, security isolation or lifecycle requirements are proven.

## Proposed decision

Accept **Option A**.

### 1. Accounts owns the authenticated portfolio use case

The native Account Center reads a semantic application result rather than raw Oteryn-v2/Canary rows:

```text
Authenticated Platform Identity
  -> canonical AccountId
  -> Accounts / Character Portfolio application boundary
  -> authorized game-owned projection
  -> presentation/privacy composition
  -> Account Center
```

`CharacterPortfolio` is an application/read model, not an authoritative character aggregate and not a new deployment unit.

The concrete class/interface names are implementation details and are not frozen by this ADR.

### 2. Character Authority remains the native character source of truth

For native Oteryn-v2 integration:

- Platform supplies canonical `AccountId`;
- Character Authority supplies canonical `CharacterId`;
- Character Authority owns the current `AccountId <-> CharacterId` relation;
- a portfolio read is an authorized projection of game-owned state;
- every native mutation is executed and validated through a versioned game-owned command boundary;
- Platform caches/read models never become ownership proof.

A stale Platform record, browser-supplied character identifier or presentation preference is never sufficient authorization for a character operation.

### 3. Baseline portfolio projection is semantic and identity-safe

A future authorized projection is conceptually equivalent to:

```text
GetCharacterPortfolio(AccountId, applicability)
    -> CharacterPortfolio
```

The baseline semantic character summary may expose only fields needed by the authenticated product use case, such as:

```text
CharacterId
WorldId
name
class_or_vocation
level_or_progression_summary
lifecycle_status
projection_revision
observed_at
```

`CharacterId` and `WorldId` use their canonical typed identities.

`ChannelId` is **not** part of the baseline character portfolio. Channel is topology/runtime placement, not durable character identity or logical world membership. It may be added to a separate runtime/session projection only when a concrete product use case and source semantics require it.

Exact HTTP/gRPC transport, wire schema and FND-02 protocol encoding remain deferred.

### 4. Capability decisions are source-owned, not reconstructed from rows

The native target must remove logic equivalent to:

```text
count(active_characters) < 10 => can_create
```

The UI receives typed capability results from application services.

Capability authority is split by domain:

- **game-domain gates** — Character Authority decides game-owned invariants such as character lifecycle, world/ruleset availability, name reservation and game-owned slot/admission constraints;
- **Platform gates** — Platform decides Platform-owned authentication/session/MFA, account/business/legal and entitlement workflow constraints;
- **effective UI capability** — the application layer combines the required gates fail-closed and preserves typed denial reasons/source provenance.

Platform must not recreate a game-domain rule from row count or stale projection.

If a future Platform entitlement changes a game-domain limit, the entitlement/grant must cross an explicit versioned contract into the authoritative game command/capability decision. Platform must not bypass Character Authority by locally adding slots.

Exact denial-code vocabulary and entitlement transport are deferred.

### 5. Freshness and failure are first-class

Every native portfolio projection must distinguish at least:

- authoritative success with an observation/revision marker;
- empty portfolio;
- stale projection;
- dependency unavailable;
- ownership/account ambiguity;
- unsupported/incompatible contract state.

A cached projection may be used for bounded low-risk presentation only under a later explicit freshness policy. It must never:

- establish current ownership;
- authorize a mutation;
- convert dependency failure into an empty list;
- extend private disclosure after ownership can no longer be proven.

Mutation commands revalidate current authoritative ownership and command preconditions regardless of what the last portfolio projection displayed.

Exact TTL, caching implementation and consistency mechanism are deferred.

### 6. CharacterProfiles remains Platform-owned presentation/privacy state

`CharacterProfiles` is a real Platform subdomain and should be represented explicitly in `MODULE_CATALOG.md` after this ADR is accepted.

It owns only Platform presentation/privacy preferences such as:

- `public_comment`;
- account-association/status/guild/house/skills/deaths/kills visibility;
- optional main-character selection.

In the native target those preferences reference canonical `CharacterId`.

Platform-local `identity_id` may remain an internal persistence surrogate under ADR 0028. A preference row may associate the local Platform owner with `CharacterId`, but neither that association nor `is_main_character` proves current game ownership.

The current `canary_player_id` remains compatibility state until a separately authorized migration.

### 7. Canary compatibility migration is additive and fail-closed

Do not replace numeric identifiers in place.

A later migration must provide an explicit anti-corruption/mapping boundary with these invariants:

- canonical `CharacterId` is added before legacy removal;
- Canary player ID -> `CharacterId` mapping comes from authoritative migration/projection evidence, never hashing, truncation or undocumented derivation;
- unresolved or conflicting mappings remain unmapped and cannot be attached to another character by inference;
- canonical `CharacterId` becomes the preferred native reference after verified backfill/cutover;
- legacy reads/writes remain only in the declared Canary compatibility path during coexistence;
- rollback is possible while compatibility columns/mappings are retained;
- legacy-column removal occurs only in a later gated task after consumers and rollback criteria are proven.

Exact schema, dual-read/dual-write mechanics and backfill tooling are deferred.

### 8. Module dependency direction

Target dependency direction:

```text
Identity
  -> Accounts / Character Portfolio composition
       -> authorized Character Authority projection adapter
       -> CharacterProfiles preferences
       -> presentation result

Characters
  -> versioned Character Authority command adapter

PublicGameData
  -> public/general game read projections

PlayerCompanion
  -> authorized owned-character context from the Accounts boundary
  -> game facts/rules from its already accepted data sources

Marketplace / Support / PlatformAPI
  -> module application services
  -> never raw Canary/Oteryn-v2 character tables
```

This refines the authenticated ownership-context boundary without making `Accounts` the owner of gameplay facts that ADR 0025 assigns to GameCatalog/PublicGameData/LiveOps/GameAnalytics.

### 9. Current Canary paths remain supported compatibility evidence

This proposed decision does not invalidate delivered Canary compatibility behavior.

Until separately authorized native implementation and migration tasks are merged and proven:

- `IdentityCanaryAccount`, `canary_account_id`, `canary_player_id` and operation-specific Canary adapters may remain in their declared current paths;
- existing Canary character-create/Bazaar contracts remain valid for their current compatibility scope;
- new native consumers must not treat those numeric IDs as permanent Oteryn-v2 identities;
- no existing path is deleted merely because the target architecture is accepted.

### 10. Historical ADR 0008 is preserved

ADR 0008 remains an accepted historical decision for frontend shell/information architecture.

Its statement that Account Overview was not yet delivered was true for its acceptance context and is not rewritten. This ADR records the later current-state fact that Account Center exists and defines the native target beneath that surface.

## Consequences

### Positive

- Account Center gets one stable native application boundary without creating a microservice.
- `AccountId`, `CharacterId`, `WorldId` and `ChannelId` retain distinct owners and semantics.
- PlayerCompanion and PlatformAPI avoid inheriting Canary numeric IDs.
- Platform presentation/privacy data remains clearly Platform-owned.
- Character policy cannot silently drift into Blade/read-model row-count logic.
- stale/unavailable/empty states become architecturally distinct.
- migration can be additive and reversible.

### Costs

- a native authorized Character Authority projection/command adapter is still required;
- CharacterProfiles needs a later additive CharacterId migration;
- capability composition needs typed cross-domain failure semantics;
- current Canary compatibility code remains until an explicit cutover;
- exact entitlement integration and projection freshness require later decisions.

## Rejected shortcuts

- make Platform authoritative for native character ownership because it authenticates the account;
- reuse `canary_player_id` or `canary_account_id` as native identifiers;
- treat a cached portfolio as mutation authorization;
- derive native create eligibility from a hard-coded row count;
- make PublicGameData the owner of private Account Center composition;
- create a new microservice without measured isolation/scaling/lifecycle need;
- put `ChannelId` into the durable character identity model by default;
- rewrite accepted historical ADRs to match later implementation;
- remove Canary compatibility before a proven migration/cutover.

## Acceptance and follow-up

This ADR remains **Proposed** until the repository owner answers Issue #857.

If Option A is accepted, the same bounded architecture package should:

1. change this ADR lifecycle to `Accepted`;
2. remove `ARCH-DEC-0001` from the active architecture decision backlog;
3. update `MODULE_CATALOG.md` to classify `CharacterProfiles` and clarify Accounts/Characters/PublicGameData responsibilities;
4. update `DATA_OWNERSHIP.md` to distinguish current Canary preference keys from the native CharacterId target;
5. update `PORTAL_COMPLETENESS_ARCHITECTURE.md` with the accepted Character Portfolio dependency before PlayerCompanion;
6. preserve ADR 0008 history unchanged;
7. leave runtime, schema, transport and cross-repository implementation for separately authorized tasks.

No implementation, database migration, Oteryn-v2/Canary repository write, protocol activation, deployment or production change is authorized by accepting this ADR alone.

## References

- Issue #857
- `docs/architecture/ARCHITECTURE_AUTHORITY.md`
- ADR 0001
- ADR 0008 — Oteryn frontend information and shell architecture
- ADR 0025 — Player Companion and portal-tools boundary
- ADR 0028 — Platform AccountId cross-boundary identity
- ADR 0029 — Platform WorldId/ChannelId topology identity
- `docs/architecture/DATA_OWNERSHIP.md`
- `docs/architecture/MODULE_CATALOG.md`
- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`
- `app/Accounts/ReadModels/AccountOverviewReadModel.php`
- `app/CharacterProfiles/Models/CharacterProfilePreference.php`
- Oteryn-v2 merged PR #90 / ADR-0012 / `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`
