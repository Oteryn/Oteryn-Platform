# Oteryn-v2 Integration Architecture

## Status

`CURRENT TARGET ARCHITECTURE — IMPLEMENTATION/CUTOVER NOT IMPLIED`

This is the focused canonical Oteryn Platform document for the native Platform ↔ Oteryn-v2 integration boundary and the isolation of legacy Canary compatibility. ADR 0031 owns the durable decision.

The current overall state is:

`TRANSITIONAL / ARCHITECTURALLY SOUND CORE / CROSS-REPOSITORY RECONCILIATION REQUIRED`

That status means the Platform core architecture remains valid, while delivered Canary-compatible integrations and transitional native-protocol artifacts must be reconciled incrementally with the native Oteryn-v2 target before activation.

## Scope

This document owns the Platform-side model for:

- native versus compatibility integration boundaries;
- bounded-context ownership across Platform and Oteryn-v2;
- Platform identity/control-plane versus game-domain authority;
- admission dependency direction;
- native persistence separation;
- character command/query/projection integration;
- PublicGameData native projections;
- native runtime-status/readiness projection routing;
- Game Analytics consumption;
- compatibility anti-corruption and migration principles;
- cross-repository contract quality requirements;
- unresolved integration architecture backlog.

It does not own concrete Oteryn-v2 implementation, gameplay engine architecture, protocol IDL bytes or exact deployment topology. Oteryn-v2 remains external/read-only to this Platform document.

## Architecture principle

Do not confuse **what is currently delivered for Canary compatibility** with **what new native Oteryn-v2 integrations should be designed against**.

```text
CURRENT COMPATIBILITY                       TARGET NATIVE INTEGRATION

Platform                                    Platform
  |                                           |
  | Canary IDs / SQL / adapters               | typed identities
  | operation-specific legacy writes          | versioned contracts
  v                                           | commands/queries/events
Canary-compatible DB/runtime                  v
                                            Oteryn-v2 service/game boundary
                                                |
                                                v
                                            game-owned persistence/runtime
```

Both states may coexist during migration. Compatibility does not become target architecture merely because it is currently implemented.

## Target integration split

```text
Oteryn Platform
├── Native Oteryn-v2 Integration
│   ├── Identity / AccountId authority
│   ├── OAuth + PKCE / Platform sessions
│   ├── one-time Game Login Ticket
│   ├── World Registry / routing policy
│   ├── Game Gateway pre-admission control plane
│   ├── runtime-status/readiness consumer projection
│   ├── command orchestration
│   ├── queries / projections / read models
│   ├── integration events
│   ├── business / commerce / support workflows
│   └── Platform observability / reconciliation
│
└── Legacy Canary Compatibility
    ├── canary_account_id / canary_player_id
    ├── direct read-only SQL contracts
    ├── operation-specific legacy write credentials
    ├── Canary-compatible Game Session paths
    ├── Canary protocol/profile adapters
    └── migration mapping / rollback bridges
```

The compatibility side is an anti-corruption layer. New native application modules must not depend on its physical schema or numeric identifiers unless the dependency is explicitly a migration adapter.

## Bounded-context ownership

| Concern | Primary authority | Platform responsibility |
|---|---|---|
| Native account identity | Oteryn Platform | Issue canonical `AccountId`; authenticate user; own account-security lifecycle |
| OAuth/PKCE, MFA, recovery, web sessions | Oteryn Platform | Full authority |
| Game Login Ticket | Oteryn Platform | Issue, revoke/consume under accepted Gateway contract |
| World/Channel registry identity and routing policy | Oteryn Platform | Control-plane authority under ADR 0029 |
| Native runtime health/readiness/capacity source facts | Oteryn-v2 runtime/orchestration authority | Validate/cache/project accepted observations; never manufacture runtime truth from Platform configuration |
| Gateway ticket redemption and pre-admission | Oteryn Platform | Authenticate attempt, intersect routing policy with fresh required runtime evidence, bind bounded pre-admission metadata |
| Canonical character identity | Oteryn-v2 game domain | Consume `CharacterId`; never issue a competing native ID |
| Current AccountId ↔ CharacterId ownership | Oteryn-v2 game domain | Consume authorized projection; never treat cache as proof |
| Character lifecycle/mutation | Oteryn-v2 game domain | Orchestrate approved commands/sagas only |
| Final gameplay admission | Oteryn-v2 game domain | Supply trusted pre-admission context; game revalidates authority |
| Authoritative admitted gameplay session/lease/fencing | Oteryn-v2 game domain | Carry correlation/control metadata only as contracted |
| Gameplay/world state | Oteryn-v2 game domain | Read via approved projections/query contracts |
| Native gameplay protocol semantics | Oteryn-v2 game domain + native client contract owner | Platform may carry selection metadata; does not own gameplay packet semantics |
| Native game persistence | Oteryn-v2 game domain | No steady-state direct table dependency |
| Public game-data website models | Platform projection/read-model layer | Consume game-owned source facts with freshness/revision metadata |
| LiveOps world/service presentation | Platform LiveOps projection layer | Combine configured Platform policy with authoritative runtime observations without fabricating unavailable facts |
| Game Analytics source facts | Oteryn-v2 runtime | Consume approved aggregates/projections/alerts/read APIs |
| CMS/public portal | Oteryn Platform | Full Platform ownership |
| Support/moderation workflow | Oteryn Platform | Workflow/communication authority; game enforcement requires explicit command contract |
| Admin/RBAC/audit | Oteryn Platform | Enforce Platform privileges; never bypass game invariants with raw SQL |
| Wallet/payments/business workflow | Oteryn Platform | Business/financial authority within accepted commerce contracts |
| Character Bazaar commercial saga | Oteryn Platform | Auction/bid/wallet/commission/reconciliation; game owns character rebinding |

## Native admission flow

```text
Rust client
    |
    v
Oteryn Platform Identity
OAuth Authorization Code + PKCE
    |
    v
one-time Game Login Ticket
    |
    v
Oteryn Game Gateway
- service-authenticated ticket redemption
- World Registry routing/policy
- fresh required Oteryn-v2 runtime readiness/revision evidence
- bounded pre-admission metadata
    |
    v
Oteryn-v2 game boundary
- revalidate current AccountId/CharacterId ownership
- validate character/world/channel applicability
- validate session/lease/fencing state
- create/accept authoritative gameplay admission
    |
    v
protocol-oteryn
    |
    v
authoritative gameplay
```

### Admission invariants

- A Platform ticket authorizes an attempt, not an unconditional gameplay session.
- Platform `login_enabled`/configured `online` state is not sufficient native runtime readiness evidence.
- Missing, stale, unavailable, invalid or superseded required runtime evidence fails closed for new native routing/admission.
- Runtime health does not override Platform maintenance, entitlement, rollout or login policy.
- A stale portfolio or pre-transfer/pre-sale projection cannot authorize a former owner after ownership changes.
- Game admission revalidates authoritative current ownership and lifecycle state.
- No reusable user password is sent from Platform to the game server in the native path.
- OAuth access/refresh tokens are not native gameplay credentials.
- Failure after protocol/admission binding does not silently downgrade into another gameplay family.

### Native pre-admission handoff boundary

The accepted Platform-side semantics are defined by `docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md`.

The contract distinguishes three separate lifecycle objects:

```text
Game Login Ticket
  -> authorizes Gateway authentication of one login attempt

Platform native pre-admission material
  -> authorizes one bounded attempt against one selected native admission target

Oteryn-v2 canonical GameSessionId / lease / fencing state
  -> exists only under game-domain authority after final admission succeeds
```

Key invariants:

- native pre-admission material binds canonical `AccountId`, `CharacterId`, `WorldId`, `ChannelId` and selected route/revision context; Canary numeric IDs are not native authority;
- issuance composes authoritative ticket redemption, current safe character authorization evidence, World Registry policy and fresh applicable current-owner runtime evidence;
- the material is short-lived, audience/route/revision bound and replay resistant, and at most one successful authoritative admission may result from one authorization;
- Platform/Gateway never issue the canonical logical `GameSessionId`, character lease or gameplay fencing authority;
- Oteryn-v2 revalidates authoritative ownership/lifecycle and owns final admission/session/lease/fencing decisions;
- ambiguous issuance/admission outcomes do not justify blindly minting or replaying duplicate authority;
- Channel switching requires fresh destination routing/readiness evidence and fresh pre-admission material;
- reconnect/recovery uses the game-owned admitted-session contract rather than silently reusing the original pre-admission capability;
- exact Oteryn-v2 envelope bytes/transport/signing, consume store, TTL value, lease/fencing algorithm and `GameSessionId` wire form remain deferred to accepted cross-repository/FND authority.

This resolves the Platform-side P1 semantic handoff. It does **not** claim a native Oteryn-v2 consumer, Platform producer implementation, exact envelope compatibility, staging E2E or production activation.

## Native runtime-status/readiness boundary

The accepted Platform consumer semantics are defined by `docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`.

The boundary deliberately separates:

```text
Platform configured policy/lifecycle
        +
fresh accepted Oteryn-v2 runtime observations
        =
consumer evaluation for Gateway / LiveOps
```

Key invariants:

- runtime observations are scoped to canonical `WorldId + ChannelId`;
- runtime/GameNode ownership generation, readiness, recovery and capacity remain game-runtime source facts;
- Platform may persist/cache a projection, but the projection does not become the runtime source of truth;
- freshness, applicability/revision and stale-owner rejection are mandatory before an observation may influence new admission;
- `stale`, `unavailable` and `invalid` evidence are not authoritative `offline` facts;
- public surfaces never fabricate zero players/offline from missing or stale evidence;
- one failed channel/client connection does not establish whole-world outage;
- exact Oteryn-v2 producer schema, cadence, TTL values, health algorithm and ownership-generation encoding remain deferred to Oteryn-v2 `OPS-CHANNEL-01`/FND authority.

This resolves the Platform-side P1 semantic boundary. It does **not** claim producer implementation, LiveOps implementation, Gateway integration, staging proof or production activation.

## Native protocol boundary

Target native gameplay is `protocol-oteryn` between Rust client and Rust game server.

Platform can legitimately own policy metadata required before connection, for example:

- whether a native protocol version is enabled for a world/channel;
- endpoint routing metadata;
- capability/version compatibility evidence needed for pre-admission;
- correlation and rollout revision identifiers.

Platform must not become the canonical owner of:

- gameplay message semantics;
- packet/state model;
- authoritative snapshot/delta/reconciliation rules;
- gameplay action/result semantics;
- game-server serialization/framing evolution.

These semantics follow the game/native protocol owner and are referenced by Platform through immutable version/capability identifiers.

### Transitional Platform protocol artifacts

The existing Platform `OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`, Game Session v2 producer work and ADRs 0010/0011 are preserved as historical/transitional evidence. ADR 0031 supersedes their target ownership model where they assign gameplay protocol semantics or target Canary/native family choice to Platform/Gateway.

The useful simplification from ADR 0011 remains: the first native Oteryn target has one native protocol version and no speculative profile catalogue. Future variants require a deliberate new decision.

## Persistence boundary

### Native target

```text
Platform DB / cache / queue
          |
          | explicit service contracts
          | commands / queries / events / projections
          v
Oteryn-v2 boundary
          |
          v
Oteryn-v2 persistence
```

Platform native integrations do not depend on the physical game database schema.

Runtime-status storage inside Platform is a consumer projection/read model. It must preserve source revision/freshness/applicability and cannot turn cached observations into game-runtime authority.

### Compatibility state

Direct Canary SQL may continue when an existing contract explicitly defines:

- exact fields/tables;
- read/write privileges;
- owner;
- transaction/locking rules;
- compatibility assumptions;
- rollback/removal criteria.

Those credentials/adapters are labelled compatibility-only and are not reused as native Oteryn-v2 database access.

## Character Portfolio and lifecycle

ADR 0030 is the focused Platform authority.

Target direction:

```text
Platform Identity / AccountId
       |
       v
Accounts / Character Portfolio
       |  authorized projection
       v
Oteryn-v2 Character Authority
       |
       +-- CharacterId
       +-- current ownership
       +-- lifecycle/world membership
       +-- native mutation authority
```

Native mutations are explicit commands such as the conceptual families:

- CreateCharacter;
- RenameCharacter;
- ScheduleCharacterDeletion;
- RestoreCharacter;
- FinalizeCharacterLifecycle;
- TransferCharacterWorld;
- TransferCharacterOwnership.

Exact command names/schema/transport are not frozen here.

Every retryable cross-system mutation needs stable operation identity, idempotent authoritative handling and a durable way to reconcile ambiguous timeout outcomes.

## Public Game Data architecture

The native target prefers projections over direct game-table coupling:

```text
Oteryn-v2 authoritative source facts
       |
       +-- integration events
       +-- snapshots
       +-- dedicated query contracts
       v
Platform projections/read models
       |
       +-- cache
       +-- SSR/API
       +-- CDN/public surfaces
```

Likely consumers include:

- character profiles;
- highscores;
- guilds;
- houses;
- online/status;
- achievements;
- world status/history;
- approved market/game-economy views.

Each projection must define:

- producer/source authority;
- applicability (world/ruleset/season/content revision where relevant);
- observation/revision marker;
- freshness/TTL policy;
- stale versus unavailable semantics;
- privacy allowlist;
- reconciliation/rebuild method;
- failure behavior.

A dependency outage must not be rendered as fabricated empty/offline state.

For native world/channel/service status specifically, `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md` is the focused semantic baseline. PublicGameData/LiveOps may expose a narrower public view, but it cannot be more certain than the authoritative observation evidence.

## Game Analytics integration

The runtime/game domain is the source of authoritative analytics events because it observes the complete gameplay transition context.

Platform consumers should use approved:

```text
aggregates
projections
alerts
analytics read APIs
operator read models
public-safe read models
```

Platform should not infer anti-cheat, economy duplication, class balance or world-behavior truth from undocumented table snapshots when a runtime event/projection contract is required.

Analytics output exposed to Platform must preserve privacy, authorization, redaction, retention and version/applicability semantics.

## Business and enforcement sagas

### Character Bazaar

Platform owns commercial state:

- listing/auction lifecycle;
- bids/winner selection;
- wallet holds/debits/credits;
- commission/business rules;
- customer notifications;
- commercial reconciliation/audit.

Game domain owns:

- seller/current ownership validation;
- transfer eligibility;
- authoritative ownership rebind;
- session/lifecycle conflict checks;
- durable transfer result/receipt.

No distributed ACID is assumed.

### Products / entitlements

Platform may own product/entitlement state, but any entitlement that changes a game-domain invariant crosses an explicit versioned grant/command contract. Platform does not silently write game tables to "add a slot" or change gameplay rights.

### Support / moderation

Platform owns case/report/enforcement workflow and communication records. A native game sanction or runtime enforcement action requires a separately accepted game-domain command contract. Admin UI never bypasses that boundary.

## Cross-repository contract baseline

A material Platform ↔ Oteryn-v2 contract defines:

1. producer and consumer;
2. source-of-truth ownership;
3. canonical typed identities;
4. authentication/service identity;
5. audience/expiry/replay/revocation where credentials exist;
6. separate API/session/gameplay-protocol versions;
7. idempotency and ordering rules;
8. schemas, limits and typed failure categories;
9. projection freshness/revision semantics;
10. observability/correlation/redaction;
11. rollout/rollback/mixed-version behavior;
12. deterministic fixtures/contract tests and canonical fixture ownership.

Cross-repository documentation is referenced, not duplicated into competing authorities.

The runtime-status contract intentionally freezes the Platform consumer semantics above while leaving exact Oteryn-v2 producer transport/encoding to the accepted external producer contract.

The pre-admission handoff contract intentionally freezes Platform authorization/binding/failure semantics while leaving the exact Oteryn-v2 admission envelope, consume/lease/fencing implementation and canonical admitted-session state to the accepted game-domain authority.

## Migration principles

Migration from Canary compatibility to native v2 is additive and reversible until final cutover.

### Required principles

- introduce canonical native IDs before removing legacy IDs;
- source mappings from authoritative migration evidence, never undocumented derivation;
- fail closed on missing/conflicting mappings;
- dual compatibility state has an explicit owner and expiry/removal gate;
- replace one bounded adapter/consumer family at a time where safe;
- prove native producer/consumer compatibility before disabling legacy paths;
- retain rollback until the cutover gate closes;
- remove legacy credentials/table privileges after the last consumer is proven absent;
- do not interpret an accepted target architecture as proof that migration already happened.

## Current-vs-target classification

| Area | Current delivered compatibility | Native target |
|---|---|---|
| Account game binding | Canary account ID mapping | canonical Platform `AccountId` across explicit contracts |
| Character identity | numeric Canary player ID in compatibility paths | canonical game-owned `CharacterId` |
| Character portfolio | direct Canary-backed reads | authorized semantic game-owned projection |
| Character mutation | operation-specific Canary SQL contracts | game-owned versioned commands + receipts |
| Public game data | direct read-only Canary SQL/Redis where contracted | events/snapshots/query contracts → Platform projections |
| World/runtime status | persisted compatibility status + bounded Canary runtime readers where implemented | configured Platform policy intersected with fresh canonical Oteryn-v2 runtime observations → Gateway/LiveOps projections |
| Game session | Canary-compatible Game Session path / transitional v2 producer | Platform bounded pre-admission authorization + game-owned final admission, lease/fencing and canonical `GameSessionId` |
| Gameplay protocol | Canary adapter + transitional Platform native contract | game/native-owner `protocol-oteryn`; Canary only compatibility/reference |
| Game persistence | shared/Canary-compatible DB access | separate v2 persistence behind contracts |
| Analytics | bounded existing projections / future GameAnalytics | game-runtime source facts → approved Platform analytics projections |

## Risks and required controls

### P0/P1 risks

- **Dual integration model ambiguity** — control with explicit native/compatibility naming and authority routing.
- **Dual native protocol authority** — control by ADR 0031 and one canonical game/native protocol owner.
- **Dual character authority** — control by ADR 0030; Platform projections never prove ownership.
- **Pre-admission/admitted-session collapse** — control by `OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md`; Platform authorizes a bounded attempt while Oteryn-v2 owns final admission, lease/fencing and canonical `GameSessionId`.
- **Runtime configuration/observation collapse** — control by `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`; configured online/login policy never substitutes for fresh runtime readiness, and stale/unavailable evidence is never fabricated as offline.
- **Canary ID leakage** — new native modules use canonical IDs; compatibility mappings stay in adapters.
- **Shared-database shortcut** — new native mutations/reads require explicit contracts.
- **Synchronous web dependency on game runtime** — public surfaces prefer resilient projections/read models.
- **Unversioned cross-repo semantics** — require explicit schema/version/capability and compatibility evidence.

### P2 risks

- fragmented correlation/tracing across Platform/Gateway/game;
- incomplete adapter-sunset inventory;
- projection freshness drift;
- mixed-version deployment ambiguity;
- operational failure modes not yet exercised in staging.

## Resolved focused integration boundaries

The following focused architecture questions are now semantically resolved but still require separately authorized implementation/reconciliation:

- **World/channel runtime status → World Registry / Game Gateway / LiveOps** — resolved by `docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`: Platform configured policy remains separate from authoritative Oteryn-v2 runtime observations; canonical scope is WorldId/ChannelId; freshness/revision/current-owner evidence is mandatory; stale/unavailable evidence fails closed for new admission and cannot be fabricated as public offline/zero state. Exact Oteryn-v2 producer transport/cadence/health algorithm remains external/deferred.
- **Platform/Game Gateway → native game pre-admission handoff** — resolved by `docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md`: Game Login Ticket, bounded pre-admission authorization and game-domain canonical `GameSessionId` are distinct; Platform binds one short-lived attempt to canonical identities/route/revisions and fresh required evidence; Oteryn-v2 revalidates authoritative ownership and owns final admission/lease/fencing/session state. Exact envelope bytes/transport/signing, consume store, TTL, lease/fencing algorithm and `GameSessionId` wire form remain external/deferred.

## Deferred architecture backlog

The following are intentionally not solved in this baseline and should be addressed in focused decisions rather than assumptions.

### P1

1. Character command/result schema and transport family beyond ADR 0030 semantics.
2. Public Game Data projection/event catalogue, freshness SLAs and rebuild/reconciliation rules beyond the focused runtime-status contract.
3. Products/entitlements → game-authority grant/delivery saga.
4. Support/moderation → game enforcement command contract.
5. Native Game Catalog/content ownership versus legacy Canary importers.

### P2

1. Unified correlation/trace/security envelope across Platform/Gateway/Oteryn-v2.
2. Per-adapter Canary compatibility sunset/removal criteria and inventory.
3. Mixed-version operational dashboards and contract-drift alerting.

## Open PR disposition snapshot

A dated review snapshot is preserved in `docs/agents/reports/OTERYN-20260808-platform-v2-architecture-reconciliation.md`. PR classification is transient operational evidence and is not part of the durable architecture authority in this focused document.

## Non-goals

- no runtime implementation;
- no database migration;
- no Oteryn-v2/Canary repository write;
- no deletion of compatibility adapters;
- no production activation;
- no protocol IDL/schema implementation;
- no runtime-status producer endpoint/event schema implementation;
- no pre-admission envelope/consumer implementation;
- no microservice decomposition solely for architectural neatness;
- no assumption that an unresolved deferred contract already exists.

## References

- ADR 0031 — native Oteryn-v2 integration and legacy Canary compatibility boundary
- ADR 0030 — Native Character Portfolio / Account Center v2
- ADR 0028 — Platform AccountId cross-boundary identity
- ADR 0029 — Platform WorldId/ChannelId topology identity
- `docs/architecture/DATA_OWNERSHIP.md`
- `docs/architecture/MODULE_CATALOG.md`
- `docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md`
- `docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md`
- `docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`
- `docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md`
- current Canary compatibility contracts under `docs/contracts/**`
- read-only accepted Oteryn-v2 Character Authority / GameNode execution-capacity-recovery / Platform Identity-Game Gateway-admission / cross-repository contract evidence
