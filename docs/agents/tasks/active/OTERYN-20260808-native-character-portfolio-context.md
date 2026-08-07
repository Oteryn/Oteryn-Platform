# OTERYN-20260808-native-character-portfolio-context

```yaml
task_id: OTERYN-20260808-native-character-portfolio-context
repository: blakinio/Oteryn-Platform
programme: OTERYN_PLATFORM_ARCHITECTURE
mode: ARCHITECTURE_CONTINUATION
status: analysis_ready
base_branch: main
base_sha: 5929e088df618ca35713b8a7004baa52d0e5af83
branch: docs/OTERYN-20260808-native-character-portfolio-context
created_at: 2026-08-08T00:18:00+02:00
owner: next architecture agent
implementation_authorized: false
runtime_changes_authorized: false
cross_repository_runtime_changes_authorized: false
```

## Goal

Continue architecture work for the Oteryn web platform, with `blakinio/Oteryn-Platform` as the primary repository and `blakinio/Oteryn-v2` treated as the game-domain system integrated through explicit contracts.

The immediate architecture subject is a **Native Character Portfolio / Account Center v2 boundary** that removes Canary numeric identifiers and direct Canary assumptions from the target web-platform model without prematurely implementing runtime migrations.

## Scope guard

This task is about the **web platform**. Do not switch the primary workstream to game-server architecture.

Use Oteryn-v2 only to verify and consume the accepted cross-repository ownership contracts needed by the Platform.

Do not implement runtime code, database migrations, Oteryn-v2 changes, Canary changes, protocol changes or production changes unless the repository owner explicitly authorizes implementation.

## Current verified Platform state

Verified Platform `main` at task creation:

`5929e088df618ca35713b8a7004baa52d0e5af83`

The portal foundation is considered sound and scalable. The current canonical direction retains:

- Laravel modular monolith;
- server-rendered Blade as the baseline UI;
- explicit bounded modules;
- Platform/game-domain separation;
- explicit operation-specific mutation contracts;
- no default SPA rewrite;
- no default microservice decomposition.

`docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md` explicitly states that the portal architecture is not globally product-complete.

## Existing accepted portal architecture relevant to this task

### Portal completeness

`docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`

Key current gaps include:

- PlayerCompanion implementation;
- LiveOps;
- first-party Platform API;
- multi-world/profile/season-safe presentation;
- client distribution provenance/update policy;
- expanded community/read surfaces;
- federated search;
- production completion evidence.

### PlayerCompanion

ADR 0025 and `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md` already accept `PlayerCompanion` as a bounded module in the Laravel modular monolith.

It consumes versioned data from GameCatalog, Wiki, PublicGameData, future GameAnalytics and LiveOps.

It must not become the canonical owner of characters or game state.

### Frontend / Account Center

ADR 0008 defines Public Portal, Account Center and Admin Console as distinct but related UI systems.

The ADR contains historical wording that a general Account Overview was not yet delivered. That statement is now stale: `routes/web.php` contains `/account` and the current repository contains `AccountOverviewController` and `AccountOverviewReadModel`.

Do not rewrite ADR history silently. If the stale statement becomes materially misleading, resolve it through a new focused ADR/clarification or other authority-compatible update.

## Current implementation evidence and architecture drift

### Account Center exists

Current route:

```text
GET /account -> AccountOverviewController::show
```

Current Account Center implementation uses:

- `IdentityCanaryAccount`;
- `canary_account_id`;
- `CanaryGameDataRepository::activeCharactersForAccount()`;
- a Platform-local `CHARACTER_LIMIT = 10`;
- direct counting of Canary-backed character rows to infer whether character creation is allowed.

This is valid current Canary compatibility behavior but is not the desired native Oteryn-v2 target model.

### Character profile preferences exist

Current `app/CharacterProfiles` contains Platform-owned presentation/privacy preferences including:

- public comment;
- account-association visibility;
- status/guild/house/skills/deaths/kills visibility;
- main-character flag.

The current persistence key uses numeric `canary_player_id`.

This is another Canary compatibility identity that should not be the long-term cross-boundary character identity.

### Module catalog drift to review

`app/CharacterProfiles` and `routes/modules/character-profile-preferences.php` exist, but the top-level module table in `docs/architecture/MODULE_CATALOG.md` does not list `CharacterProfiles` as a standalone module.

This needs an explicit architecture classification rather than accidental growth.

Recommended direction for review:

- `Characters`: Platform orchestration of explicitly approved character operations;
- `PublicGameData`: public game-owned character/world projections;
- `Accounts`: authenticated Account Center and character portfolio composition;
- `CharacterProfiles`: small Platform-owned privacy/presentation preference subdomain, not a second authoritative character system.

A rename such as `CharacterPresentation` may be considered later, but do not perform a repository-wide rename without a bounded decision and migration justification.

## Cross-repository source of truth already accepted

Oteryn-v2 has accepted native Character Authority semantics in its canonical architecture.

The important consumer-side facts for Platform are:

- Platform Identity owns and issues `AccountId`;
- Oteryn-v2 Character Authority owns and issues canonical `CharacterId`;
- authoritative current `AccountId <-> CharacterId` ownership is game-domain owned;
- Platform may consume an authorized account-to-character query/projection;
- Platform caches/read models are non-authoritative;
- final gameplay admission revalidates authoritative ownership;
- create, rename, delete/restore/finalize, world transfer and account/Bazaar transfer use versioned game-owned command boundaries in the native target;
- Platform direct SQL writes to native game character tables are not the steady-state target;
- rename/world transfer/account transfer preserve `CharacterId`;
- terminal deletion never permits CharacterId reuse.

Do not re-open these ownership questions unless current source-of-truth documents have changed.

## Candidate target: Native Character Portfolio / Account Center v2

This candidate is the immediate architecture subject. It has been recorded for continuation, but details must be reconciled with canonical Platform authority before being promoted to an Accepted ADR.

Target shape:

```text
Platform Identity
  AccountId
    |
    v
Account Center
    |
    v
Character Portfolio application boundary
    |
    +-- Platform privacy/preferences
    +-- PlayerCompanion character context
    +-- Marketplace/Bazaar orchestration
    +-- character lifecycle command orchestration
    |
    v
Character projection / command adapter
    |
    | versioned contract
    v
Oteryn-v2 Character Authority
    CharacterId
```

### Candidate invariant: no native game DB coupling

The Account Center must not know the physical Oteryn-v2 database schema and must not use direct SQL to infer native character ownership or mutation authority.

### Candidate account-to-character query

Conceptually consume a game-owned authorized projection such as:

```text
GetCharactersForAccount(AccountId)
    -> CharacterSummary[]
```

Candidate summary fields:

```text
CharacterId
WorldId
name
vocation_or_class
level
lifecycle_status
projection_revision
freshness
capabilities
```

Exact API transport, wire schema and FND-02 protocol mechanics remain out of scope until their respective architecture gates are resolved.

### Candidate portfolio response

The Platform should prefer a semantic portfolio/capability response over reconstructing game policy from raw rows:

```text
CharacterPortfolio
{
    characters
    active_count
    allowed_count
    can_create
    create_block_reason
    projection_revision
    freshness
}
```

The exact names are not frozen. The key architectural point is that the producer supplies authoritative policy/capability facts rather than the browser or Platform deriving them from legacy table shape.

## Candidate capability rule

The Platform should not decide native creation eligibility using logic equivalent to:

```text
if character_count < 10
    allow creation
```

The preferred target is capability-driven:

```text
can_create_character = true | false
reason = SLOT_LIMIT | WORLD_CLOSED | ACCOUNT_STATE | RULESET | ENTITLEMENT | OTHER_TYPED_REASON
```

This supports future policies such as:

- account-wide or world-specific slot rules;
- entitlements/premium slot additions;
- seasonal/test/reference world restrictions;
- ruleset changes;
- temporary creation closures;
- account lifecycle restrictions.

The Platform may render the allowed/current counts for UX, but should not become the authoritative owner of game-domain character-slot policy.

## Candidate identity migration direction

Target native Platform references should use canonical `CharacterId`, not `canary_player_id`.

Current numeric Canary identifiers should remain only in an explicitly named compatibility adapter/migration boundary while Canary compatibility is required.

Do not simply replace integer columns in-place without deciding:

- dual-read/dual-reference migration period;
- projection backfill strategy;
- idempotent mapping between legacy numeric ID and CharacterId where required;
- stale/missing mapping semantics;
- rollback/removal conditions;
- privacy and audit consequences.

## Character profile/privacy ownership

Recommended semantic split:

Game domain owns:

- character identity;
- current ownership;
- authoritative gameplay state;
- lifecycle state;
- game-domain world placement.

Platform owns presentation/privacy preferences, for example:

```text
public_comment
show_account_association
show_status
show_guild
show_house
show_skills
show_deaths
show_kills
is_main_character
```

These Platform preferences should reference canonical `CharacterId` in the native target.

They must never become proof of character ownership or gameplay authority.

## Why this should precede PlayerCompanion implementation

Character Portfolio is a shared dependency for many future web capabilities:

- Account Center;
- Character Profiles/privacy;
- PlayerCompanion personalization;
- saved builds/goals tied to owned characters;
- Marketplace/Bazaar orchestration;
- Support views;
- future first-party Platform API;
- authorized Game Analytics projections.

Building PlayerCompanion directly on `canary_player_id` or `IdentityCanaryAccount` would create migration debt and duplicate the compatibility model in new modules.

## Recommended Platform architecture order

Use this as current continuation priority unless newer authority changes it:

1. Native Character Portfolio / Account Center v2 boundary.
2. LiveOps + World Registry presentation boundary.
3. PlatformAPI boundary for first-party client/tools consumers.
4. PlayerCompanion P0 vertical slices.
5. Expanded Community/PublicGameData surfaces: houses, guilds, statistics as approved.
6. Federated portal/content search.
7. Client updater/download provenance and release policy.

This is an architecture-planning order, not implementation authorization.

## Required next-agent work

The next architecture agent should:

1. Re-read current `main` and repository agent instructions before relying on this checkpoint.
2. Reconcile this candidate boundary against `ARCHITECTURE_AUTHORITY.md`, ADR 0001, ADR 0008, ADR 0025, ADR 0028, ADR 0029, `DATA_OWNERSHIP.md`, `MODULE_CATALOG.md`, `PORTAL_COMPLETENESS_ARCHITECTURE.md`, current Characters/Accounts/PublicGameData/CharacterProfiles code and the accepted Oteryn-v2 Character Authority contract.
3. Identify contradictions caused by current Canary-specific Account Center behavior.
4. Decide whether a new Platform ADR is needed for `Native Character Portfolio / Account Center v2`.
5. Propose explicit producer/consumer ownership, freshness/failure semantics, capability semantics, migration/compatibility boundaries and module ownership.
6. Keep wire format, exact HTTP/gRPC endpoints and Oteryn-v2 runtime implementation deferred unless their gate is already accepted.
7. Present any material new architecture decisions to the repository owner for explicit acceptance before recording them as Accepted.
8. If accepted, record the decision canonically in one bounded documentation package and update any affected focused architecture/module inventory documents without rewriting historical ADRs.

## Questions still deliberately unresolved

- Exact placement/name of the Character Portfolio application service inside Platform modules.
- Whether `CharacterProfiles` remains a small subdomain/package or receives a formal module-catalog row.
- Exact native projection API shape and transport.
- Exact capability/error taxonomy.
- Exact character-slot product policy.
- Exact entitlement integration.
- Exact legacy Canary-to-CharacterId migration/backfill strategy.
- Exact cache TTL/freshness policy and projection consistency model.
- Exact rename/delete/transfer web UX.
- Exact Platform API exposure.

These are for architecture analysis, not assumptions.

## Explicit non-goals

- Do not design game combat/progression here.
- Do not continue GAME-CHAR-01 as the primary task.
- Do not implement Oteryn-v2 runtime.
- Do not migrate databases yet.
- Do not delete Canary compatibility code yet.
- Do not hard-code new slot limits.
- Do not create a microservice solely for Character Portfolio without measured need.
- Do not make Platform authoritative for native character ownership or gameplay state.

## Completion condition for this checkpoint

This checkpoint is complete when another agent can enter `blakinio/Oteryn-Platform`, read this file plus the referenced canonical architecture sources, and continue the Native Character Portfolio / Account Center v2 architecture discussion without relying on chat history.
