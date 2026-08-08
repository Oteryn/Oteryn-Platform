# Oteryn Platform ↔ Oteryn-v2 Architecture Delta Audit — 2026-08-08

## Result

`PASS WITH OPEN ARCHITECTURE OBLIGATIONS — NATIVE BOUNDARIES ARE COHERENT; FND-04 IS THE NEXT CROSS-REPOSITORY GATE`

This is a delta audit over `docs/agents/reports/OTERYN-20260808-platform-v2-architecture-reconciliation.md`, not a duplicate greenfield architecture review.

Repository under change: `blakinio/Oteryn-Platform`  
External repository: `blakinio/Oteryn-v2` — read-only evidence only  
Platform main at audit start: `3417086d02d275c2cf3154c5a0c9a65462202eb3`  
Oteryn-v2 live main observed during audit: `3c32fb08ddf52939159c0ace5fe607ca4fb18332`

No runtime implementation, production mutation, protected configuration change or external-repository write is authorized by this audit.

## Why a delta audit

The earlier same-day reconciliation already accepted the durable native-v2 versus Legacy Canary Compatibility split through ADR 0031. Since that review, Platform `main` has closed several previously open architecture gaps and Oteryn-v2 has advanced its foundation programme.

The correct question is therefore not whether the architecture should be redesigned. It is which gaps remain after the newer accepted contracts and live repository state are applied.

## Baseline open PR review

The following PRs were open in `Oteryn-Platform` before this audit branch/PR was created.

| PR | Classification | Evidence-based disposition |
|---|---|---|
| #923 — fail closed on staging/production mail readiness | `KEEP` | Adds provider-neutral mail-delivery readiness checks and preserves fail-closed staging/production posture without changing game/runtime authority. Exact-head workflows inspected were green except one still-running Deep System Validation snapshot; this does not prove real mailbox delivery. No Oteryn-v2 contract impact. |
| #541 — public-domain repair checkpoint | `REBASE` | Still represents useful owner-observed public-domain/password-recovery evidence, but its branch is materially diverged from current `main`. It must be refreshed rather than mechanically merged or closed. Remaining manual staging password-recovery proof is not replaced by generic mail configuration readiness. |
| #338 — inactive Game Catalog schema 1.3 NPC-shop consumer | `NEEDS_DECISION` | The work is explicitly inactive and historically depends on a Canary 1.3 producer. ADR 0031 now requires native Oteryn-v2 content/catalog ownership not to inherit Canary producer semantics accidentally. Decide whether this continues strictly as temporary `Legacy Canary Compatibility` with sunset criteria or is paused pending the native catalogue/content projection boundary. The branch also requires substantial refresh before any merge. |

No baseline PR meets the evidence threshold for `SUPERSEDED`, `DUPLICATE` or `OBSOLETE`.

**Destructive PR actions: none.**

## Current architecture verdict

The Platform core remains architecturally sound as a Laravel modular monolith with explicit bounded modules and a separately deployable Go Game Gateway. No microservice rewrite is justified by the current evidence.

The durable target remains two clearly separated integration modes:

```text
Oteryn Platform
├── Native Oteryn-v2 Integration
│   ├── versioned commands
│   ├── versioned queries
│   ├── events / projections
│   ├── runtime-status consumption
│   ├── pre-admission orchestration
│   └── explicit saga / reconciliation boundaries
│
└── Legacy Canary Compatibility
    ├── Canary numeric-ID mappings
    ├── direct read-only SQL contracts
    ├── narrow operation-specific legacy writes
    ├── Canary-compatible session/protocol adapters
    └── migration / rollback bridges
```

Legacy compatibility is real delivered behavior but is not the native domain model.

## Native authority map

### Oteryn Platform owns

- canonical native `AccountId` issuance;
- Identity, password/authentication policy, OAuth + PKCE, MFA, recovery and Platform sessions;
- one-time Game Login Ticket lifecycle;
- World Registry identity/policy/routing control plane;
- Go Game Gateway ticket redemption and pre-admission orchestration;
- Platform business/commercial truth, entitlements and customer-facing workflow state;
- Wallet/Marketplace/Bazaar Platform saga state;
- portal, CMS, admin/support workflows and Platform-local read models.

### Oteryn-v2 owns

- canonical `CharacterId` issuance;
- authoritative current `AccountId <-> CharacterId` ownership;
- character lifecycle and native game mutations;
- authoritative gameplay admission;
- character lease/fencing and canonical logical `GameSessionId` after final admission;
- `protocol-oteryn` gameplay semantics;
- ChannelRuntime/InstanceRuntime gameplay authority;
- native game persistence;
- authoritative gameplay/economy/security source facts for Game Intelligence.

Neither a browser claim, Platform cache, ticket, workflow row nor legacy Canary identifier can replace current game-domain authority.

## Current native admission path

```text
Rust client
  -> Platform Identity: OAuth Authorization Code + PKCE
  -> one-time Game Login Ticket
  -> Platform Game Gateway
  -> authoritative ticket redemption
  -> World Registry policy/routing
  -> Platform policy AND fresh applicable Oteryn-v2 runtime-status evidence
  -> short-lived native pre-admission material
       [NOT GameSessionId]
  -> Oteryn-v2 final admission + ownership/lifecycle checks
  -> character lease / fencing
  -> canonical game-owned GameSessionId
  -> protocol-oteryn gameplay
  -> ChannelRuntime / InstanceRuntime authoritative execution
```

A Game Login Ticket, Platform pre-admission material and canonical `GameSessionId` are three separate lifecycle objects.

## Cross-system read and mutation model

### Public / account reads

```text
Oteryn-v2 authoritative state
  -> versioned query / snapshot / projection / integration event
  -> Platform non-authoritative read model
  -> API / SSR / cache / CDN
```

Fresh, stale, unavailable and invalid evidence remain distinct. Missing or stale runtime/player-count data must not be fabricated as `offline` or `0 online`.

### Mutations

```text
Platform authenticated/business intent
  -> stable operation identity
  -> versioned game-owned command
  -> Oteryn-v2 authoritative validation/mutation
  -> typed durable outcome/receipt
  -> Platform saga completion or reconciliation
```

Timeout/transport loss is not proof of either success or failure. Exact retries reuse the same semantic operation identity. Native fallback to direct game SQL after an ambiguous result is forbidden.

### Entitlement delivery

Current architecture now distinguishes:

1. payment/order truth;
2. Platform entitlement truth;
3. Oteryn-v2 game delivery/enforcement truth.

A payment callback or active entitlement never directly proves game delivery. Durable game-affecting fulfilment uses a stable delivery identity and reconciliation/compensation policy.

## Material progress since the previous reconciliation

The previous reconciliation listed several P1 follow-ups. Current `main` has since materially narrowed that list:

- native gameplay protocol authority was repaired: the older Platform/Otheryn package is now explicitly historical/transitional evidence, while Oteryn-v2 owns current native protocol/game-session semantics;
- Platform native pre-admission semantics are now defined without pretending to own final admission/GameSession/lease authority;
- native runtime-status consumer semantics are defined: configured Platform policy and fresh current-owner game-runtime evidence are separate authorities and both are required for new native admission;
- native Character Authority command/result semantics are defined for create/rename/delete/restore/transfer families with stable operation identity, terminal/non-terminal outcomes and ambiguity reconciliation;
- native entitlement/game-delivery authority is defined with separate commercial, entitlement and gameplay truth domains;
- native public-game-data projection/privacy boundaries have been delivered and remain non-authoritative for ownership/mutation;
- Oteryn-v2 `FND-02` is accepted and merged;
- Oteryn-v2 `FND-03` runtime-execution architecture has now merged; it still authorizes no Rust runtime implementation.

The next Oteryn-v2 foundation architecture gate is therefore `FND-04` rather than `FND-03`.

## Remaining P1 architecture obligations

### P1-1 — FND-04 admission / Game Session / character lease

`CROSS-REPOSITORY ARCHITECTURE DECISION REQUIRED`

This is the highest-priority next architecture slice.

Owner split:

- **Oteryn-v2 owns** final admission, authoritative game-session lifecycle, canonical `GameSessionId`, lease/fencing, duplicate-session/takeover semantics and reconnect/recovery behavior;
- **Oteryn Platform owns** Identity/security policy, Game Login Ticket, World Registry, Gateway routing and pre-admission producer semantics.

The shared contract must reconcile at least:

- pre-admission capability envelope/version ownership;
- issuer/audience and service authentication;
- expiry and key/credential rotation;
- one-use/replay/atomic-consume semantics;
- security revocation after pre-admission issuance;
- route/runtime ownership-generation binding;
- character ownership/lifecycle revalidation;
- lease acquisition and duplicate-login races;
- exact point where canonical `GameSessionId` is created;
- relationship to FND-02 `connection_generation`, CommandId and reconnect fencing;
- admitted-session reconnect versus fresh login;
- Channel switch requiring a fresh destination admission/session;
- typed failure vocabulary without account/character enumeration;
- ambiguous admission and recovery semantics;
- mixed producer/consumer version rollout and rollback;
- logs/traces/correlation with bearer material redaction;
- exact-revision contract fixtures and native E2E acceptance.

Do not choose JWT, mTLS, a database lease primitive, heartbeat cadence or other mechanism until the owning FND-04 evidence requires it.

### P1-2 — Game Catalog native ownership versus Canary schema 1.3

PR #338 remains a real decision point.

If kept, it should be classified explicitly as `Legacy Canary Compatibility` with:

- no native Oteryn-v2 authority claim;
- no assumption that Canary NPC/shop schema becomes the native content contract;
- separate compatibility producer version pinning;
- migration/removal criteria;
- no production activation without independent evidence.

Otherwise pause the branch and design the native GameCatalog/content projection against Oteryn-v2's project-owned world/content model.

### P1-3 — Support / moderation game-enforcement command boundary

Platform support tickets, reports and enforcement records remain Platform workflow truth, not native game-ban authority.

Before Platform can mutate game sanctions/enforcement, define an explicit game-owned command/result contract with authentication, reason/actor evidence, idempotency, concurrency, audit, reconciliation, revocation/expiry and appeal implications. Admin tooling may not bypass game invariants through direct SQL.

### P1-4 — Platform PostgreSQL migration programme

Oteryn-v2 has accepted PostgreSQL with separate `oteryn_platform` and `oteryn_game` logical databases, owners, credentials and migration histories. Platform current compatibility can remain temporarily on existing storage, but target convergence requires a dedicated Platform migration architecture/task covering Laravel compatibility, conversion, locking/index differences, backup/restore, rollback and coexistence with temporary Canary/MariaDB adapters.

This is not an authorization to migrate production now.

## P2 architecture / governance obligations

1. define a unified cross-system correlation/security envelope where needed without collapsing request ID, correlation ID, causation ID, operation ID, GameSessionId and security generation into one identifier;
2. create a centralized Legacy Canary Compatibility sunset/removal inventory with owner, consumer, replacement, rollback and removal gate per adapter;
3. define mixed-version contract drift monitoring / compatibility matrices for critical Platform ↔ Oteryn-v2 contracts;
4. wait for Oteryn-v2 `ANL-01` before freezing Platform Game Intelligence consumer schemas, privacy classes or retention contracts; Platform must not derive authoritative analytics from raw gameplay tables;
5. reconcile stale progress-only documentation that still describes `FND-03` as the next gate or PR #102 as live; live Git/accepted merge evidence has advanced to `FND-04`;
6. improve high-level system-context discoverability so current Canary delivery topology is not mistaken for the native Oteryn-v2 target despite correct higher-precedence ADR/integration documents.

## Decision-backlog observation

`docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json` currently contains no records.

That is not itself a contradiction, because accepted ADRs and focused contracts may have resolved prior records. It does create a visibility risk while the repository still has genuine decision obligations such as PR #338 disposition and later support/moderation integration.

Do not populate the backlog mechanically from every future task. Add only unresolved architecture decisions with an explicit owner, dependency, impact and decision deadline.

## Documentation drift observed

- `docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md` still contains progress wording from before FND-03 completion. The live next gate is FND-04.
- `docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md` on `main` is behind the unmerged #541 checkpoint branch and should not be treated as the branch's latest owner state.
- Oteryn-v2 FND-03/current-status text retains some transition-safe candidate/live-PR wording even though live Git proves the contract merged and lifecycle closeout followed.
- `SYSTEM_ARCHITECTURE.md` correctly routes readers to higher-precedence authority, but its primary system-context diagram remains dominated by current Canary delivery. A later documentation reconciliation should add an equally prominent native target view rather than rewriting compatibility history.

These are source-of-truth usability/progress-state issues, not evidence that ADR 0031 ownership itself is inconsistent.

## Security assessment

No material native authority inversion was found in current Platform `main` after the same-day repairs.

The highest-risk future mistakes remain:

- treating pre-admission material as canonical GameSessionId;
- accepting stale runtime-owner evidence during failover;
- retrying ambiguous native mutation/delivery with a fresh operation identity;
- falling back from ambiguous native state to direct Canary/native SQL;
- allowing browser/client identifiers to establish ownership;
- making Platform entitlement/payment state directly mutate gameplay state;
- exposing game enforcement through admin raw SQL;
- enabling native routing before exact FND-04 producer/consumer compatibility and E2E evidence exist.

## Recommended next architecture slice

Start one bounded `FND-04` cross-repository architecture analysis using current Oteryn-v2 accepted FND-ID-01/FND-02/FND-03 contracts plus Platform ADR 0031, runtime-status and pre-admission semantic contracts.

Do not implement runtime code. The output should freeze one canonical admission/GameSession/lease contract with explicit ownership and cross-reference it from Platform rather than copy competing ADR text between repositories.

## Validation / safety classification

- architecture/documentation only: `PASS`;
- runtime/application implementation: `NOT_APPLICABLE`;
- browser/gameplay E2E: `NOT_APPLICABLE` for this audit;
- external repository writes: `NONE`;
- production/deployment mutations: `NONE`;
- baseline PR closures: `NONE`;
- baseline PR classifications: `#923 KEEP`, `#541 REBASE`, `#338 NEEDS_DECISION`;
- new accepted architecture decision: `NONE` — this audit reports current authority and remaining obligations rather than inventing a new ADR.
