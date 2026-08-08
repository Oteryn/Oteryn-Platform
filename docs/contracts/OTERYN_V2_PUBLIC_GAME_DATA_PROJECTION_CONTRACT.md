# Oteryn-v2 Public Game Data Projection Contract

## Status

`ACCEPTED PLATFORM CONSUMER ARCHITECTURE CONTRACT — PRODUCER / WORKER / STORAGE / CUTOVER IMPLEMENTATION DEFERRED`

This contract defines the Oteryn Platform semantic boundary for consuming authoritative native Oteryn-v2 public game facts into rebuildable Platform read models.

It does **not** define or authorize the Oteryn-v2 producer transport, event broker, worker framework, projection database schema, migration implementation, staging rollout, production cutover or external-repository changes.

## Authority and purpose

This contract is a focused consequence of accepted architecture:

- ADR 0031: Oteryn-v2 owns authoritative native game/runtime source facts; Platform consumes them through explicit commands, queries, events or projections instead of native steady-state shared tables/cross-system SQL;
- `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`: world/channel runtime status, readiness, freshness, capacity/player-count aggregate facts and public runtime truthfulness already have a dedicated native projection contract;
- Game Catalog contracts/programme own items, creatures, loot, NPC/shop and other content/catalogue snapshot semantics;
- Platform `PublicGameData` owns public/general read-model composition and delivery, not authoritative game mutation or gameplay truth;
- Platform `CharacterProfiles` and Identity privacy settings own presentation/privacy policy that may further restrict what accepted game facts are exposed.

The target is resilient public presentation:

```text
Oteryn-v2 authoritative game facts
        |
        v
versioned accepted source evidence
        |
        v
Platform normalized/rebuildable projection
        |
        +----> Platform privacy/presentation policy
        |              |
        v              v
last-known-good public read model
        |
        v
HTTP / API / SSR / cache / CDN
```

Normal website reads must not require a synchronous call to the game runtime merely to render ordinary public pages.

## Core source-of-truth separation

For native Oteryn-v2:

- Oteryn-v2 owns the authoritative gameplay/game-domain fact;
- Platform owns the consumer projection, public allowlist, pagination/search indexing, presentation policy, cache/CDN behavior and truthful freshness/degraded state;
- persisted Platform projection rows are disposable/read-model state, not game authority;
- a successful Platform projection update does not transfer game-domain ownership to Platform;
- current direct Canary SQL/Redis readers remain `Legacy Canary Compatibility`, not the target native source-of-truth model.

A projection may be rebuilt, replaced or rolled back without writing derived state back into Oteryn-v2.

## Canonical native identity

Native projection records use accepted canonical cross-boundary identities wherever the fact is scoped to them:

- `CharacterId` for character identity;
- `WorldId` for world scope where applicable;
- `ChannelId` together with `WorldId` for channel scope where applicable.

Rules:

- Canary numeric player/account/world/channel identifiers are compatibility-only and cannot become native projection authority;
- character display name is presentation/search data, not stable identity;
- `AccountId` is not public merely because Platform possesses a character projection;
- account/ownership relationships belong to their accepted authority and privacy contracts rather than being inferred from public projection rows.

### Native guild identity prerequisite

A native guild/public-membership projection requires a stable game-owned canonical guild identifier before runtime implementation. Its exact representation remains `UNKNOWN` in this Platform contract.

Until that identifier is accepted:

- a mutable guild name cannot be treated as stable identity;
- a Canary numeric guild ID cannot be promoted into native authority;
- the Platform architecture contract may define guild projection semantics, but native producer/consumer implementation remains gated on the accepted game-owned guild identity contract.

This deferred identifier does not transfer guild authority to Platform.

## Projection family catalogue

### 1. Character public facts and search

Authoritative source owner: Oteryn-v2 Character/game domain.

Stable native identity: `CharacterId`.

Typical game-owned public facts may include, when the accepted producer contract exposes them:

- current character display name;
- world/channel placement facts that are intentionally public;
- level/progression/vocation/class-style presentation facts;
- game-owned public skills/statistics;
- public guild relation reference;
- public property/house relation facts;
- game-owned lifecycle/public-eligibility facts needed to remove or suppress obsolete records.

Platform owns:

- public search/index shape;
- pagination and response composition;
- public allowlisting;
- freshness/degraded presentation;
- `CharacterProfiles` overlays such as owner public comment, main-character choice and visibility controls;
- account-association/status privacy upper bounds.

Game-originated facts must never bypass Platform privacy/presentation policy.

### 2. Highscores and rankings

Authoritative source owner: Oteryn-v2 game/statistics authority defined by the future accepted producer contract.

Stable subject identity: `CharacterId`; ranking scope may additionally include `WorldId` and versioned producer-owned category/season/ruleset identifiers where applicable.

Required semantics:

- ranking inputs are bound to an authoritative producer revision/snapshot or versioned source fact set;
- Platform never mixes ranking entries from incompatible source revisions as one authoritative table;
- ties/order must be deterministic under the accepted ranking contract;
- deletion/hide/rename/transfer changes must invalidate or reconcile affected entries by stable identity rather than display name;
- if Oteryn-v2 exposes authoritative ranking snapshots, Platform consumes those semantics rather than silently inventing a competing canonical ranking algorithm.

Exact categories, seasons and ranking algorithms remain producer/product authority.

### 3. Character activity: deaths and kills

Authoritative source owner: Oteryn-v2 game/activity authority.

Stable identity:

- one immutable source event/activity identifier for deduplication;
- `CharacterId` for character subjects/participants where identities are available;
- canonical scope identifiers where the event is world/channel scoped.

Required semantics:

- repeated delivery of one authoritative activity record is idempotent;
- `occurred_at` may be a presentation fact but is not sufficient ordering authority where causal ordering matters;
- source sequence/revision or another producer-owned applicability mechanism is required when ordering affects correctness;
- Platform may apply retention, pagination and public formatting policy without rewriting the authoritative event meaning;
- unknown/non-public counterpart identity must remain unknown/redacted rather than guessed from names.

### 4. Guild public facts and membership

Authoritative source owner: Oteryn-v2 guild/game domain.

Stable identity: accepted future game-owned canonical guild identifier plus `CharacterId` for members.

Typical public facts may include, when explicitly exposed:

- current guild name/public metadata;
- membership relation;
- public rank/role presentation;
- public active-member facts derivable under the accepted contract.

Required semantics:

- membership add/remove/update is reconciled by stable identities;
- rename does not create a second guild;
- character lifecycle/transfer changes cannot leave stale membership as current truth;
- Platform may index/search/paginate guild presentation but cannot become guild membership authority.

### 5. Individual character presence

Authoritative source owner: accepted Oteryn-v2 session/runtime presence authority.

Stable native identity: `CharacterId`, with `WorldId`/`ChannelId` when intentionally public and applicable.

This family may support:

- public online-character list;
- per-character online/offline/presence presentation where privacy policy permits it.

It must **not** duplicate `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`:

- world/channel runtime health/readiness remains there;
- capacity/player-count aggregate runtime facts remain there;
- a public individual-presence projection cannot infer authoritative world readiness or admission capacity;
- a runtime-status aggregate count cannot be expanded into invented individual characters.

Missing/stale presence evidence is not authoritative `offline` unless the accepted individual-presence contract establishes that conclusion.

## Explicit exclusions

This contract deliberately does not absorb:

- world/channel runtime health, readiness, capacity/player-count aggregate truth or admission evidence — use `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`;
- Game Catalog/content facts such as items, creatures, loot, NPC/shop, quest/catalogue or static content snapshots — use their owning Game Catalog/content contracts;
- authenticated Character Portfolio/current ownership authority — use Accounts/Characters/Character Authority contracts;
- Platform CharacterProfiles/Identity privacy preferences — these remain Platform-owned presentation policy layered over game facts;
- CMS/news/editorial content;
- gameplay session, lease/fencing or GameSessionId material;
- administrative/private game topology.

## Required source evidence semantics

The exact JSON, protobuf, gRPC, queue or event shape is deliberately deferred. Any accepted native producer path must provide enough semantic information for safe consumption.

A source record/snapshot/query result must identify, where applicable:

```text
contract / projection family
source record, event or snapshot identity
source contract/schema version
producer/source revision
stable subject/scope identities
entity/scope revision or sequence when ordering matters
producer generation/epoch when stale-owner acceptance is possible
occurred_at / observed_at presentation time where meaningful
visibility/lifecycle/tombstone intent where meaningful
correlation/provenance metadata needed for reconciliation
```

Not every family needs one global sequence. Ordering authority should be the narrowest producer-owned ordering domain that preserves correctness, such as per-character, per-guild, per-ranking-scope or per-snapshot revision.

Wall-clock arrival time alone is not a safe substitute for source ordering when delayed records can overwrite newer state.

## Idempotency and repeated delivery

Native projection consumers must be safe under at-least-once/repeated delivery.

At minimum:

- source record/event identity plus relevant contract/revision scope is deduplicated;
- applying the same authoritative record twice yields the same projection state;
- a superseded revision cannot overwrite a newer accepted revision merely because it arrives later;
- duplicate activity events cannot create duplicate deaths/kills;
- snapshot reapplication is deterministic for the same source revision;
- an ambiguous consumer commit is reconciled from projection/source identity instead of fabricating a second logical fact.

Exactly-once transport is not required and must not be assumed.

## Evidence state versus content state

Platform must distinguish **projection evidence state** from **business/content state**.

Projection evidence state:

```text
fresh
stale
unavailable
invalid
rebuilding
```

Content/business state may include, depending on family:

```text
present
empty
not_found
hidden
tombstoned
```

Rules:

- `stale` is not `empty`;
- `unavailable` is not `not_found`;
- `invalid` is not zero rankings/deaths/members/online characters;
- no accepted source evidence is not proof that an entity does not exist;
- privacy `hidden` is not equivalent to game-domain deletion;
- a tombstone is an authoritative lifecycle/public-removal fact, not merely an expired cache entry.

## Freshness policy

Each projection family must have an explicit versioned freshness policy before implementation/cutover.

The policy may define:

- expected producer/update cadence;
- `fresh_for` or equivalent fresh-age bound;
- whether last-known-good state may remain serveable while stale;
- maximum stale-while-servable age;
- hard-expiry/unavailable behavior;
- which user-facing surfaces must expose age/degraded context.

Numeric durations are intentionally deferred until producer cadence and product requirements are accepted.

A cache/CDN TTL cannot extend source authority beyond the projection's accepted freshness policy.

Game-source freshness and Platform privacy-decision freshness are independent authority dimensions. A game projection that is `fresh`, or `stale` but still inside its accepted stale-while-servable window, is publicly serveable only when the applicable privacy decision is independently proven current for that public representation.

## Last-known-good and website availability

Normal public HTTP/API/SSR requests read Platform projection state.

Required behavior:

- do not synchronously query the game runtime as an ordinary request-path fallback;
- when the latest projection becomes stale but remains within accepted stale-while-servable policy, serve last-known-good content with truthful stale/degraded semantics where material **only for fields whose applicable privacy decision is still proven current**;
- a last-known-good game projection never grants permission to reuse an older or unproven privacy `allow` decision;
- after hard expiry or without a safe last-known-good state, return an explicit unavailable/degraded state rather than fabricating empty/not-found/zero;
- recovery accepts a fresh authoritative projection generation before clearing game-source degraded state and separately satisfies the current privacy decision floor before restoring privacy-controlled fields;
- CDN/browser cache behavior must not hide hard-expired or invalid source evidence, or privacy-controlled output below the current privacy decision floor, as current.

This decouples website availability from temporary game-runtime unavailability without pretending stale game data is live and without weakening a newer Platform privacy restriction.

## Lifecycle, tombstones and cross-surface reconciliation

Stable identity, not display name, drives reconciliation.

### Rename

- `CharacterId` remains stable;
- current-name search/index is replaced under the authoritative revision;
- an old-name redirect/alias/history policy, if the product adopts one, is Platform presentation policy and must be explicit;
- historical activity records are not blindly rewritten when their accepted semantics preserve event-time presentation.

### Deletion/finalization

- an authoritative lifecycle/public-removal result produces a tombstone/hide effect that removes current searchable/public authority;
- stale ranking/guild/presence/profile indexes cannot remain authoritative after the tombstone revision is accepted;
- final data retention is governed by accepted product/privacy/history contracts rather than accidental projection leftovers.

### Restore

- restore/public reappearance requires a newer authoritative lifecycle revision;
- an old pre-deletion record cannot implicitly resurrect the character after a tombstone.

### World/channel transfer

- scope-sensitive projections reconcile both old and new scopes;
- rankings/presence/search/guild presentation must not leave an obsolete old-scope record as current;
- Platform topology policy cannot fabricate the game-owned transfer result.

### Account ownership transfer

- public projections do not expose AccountId or infer current owner from stale projection state;
- Platform account-association presentation is recomputed under current accepted ownership/privacy evidence where that feature is enabled.

## Privacy and presentation overlay

Platform privacy/presentation decisions form an independent upper bound over accepted game facts.

The native pipeline is conceptually two-stage:

```text
authoritative normalized game projection
              |
              v
Platform privacy/presentation decision
              |
              v
public response/search/index/cache
```

Rules:

- game-originated `public=true` cannot override Platform privacy policy;
- Platform may hide information that is otherwise available from the game source;
- Platform may not fabricate a game fact merely because privacy permits displaying one;
- AccountId/current owner/private association is not published by default;
- current CharacterProfiles controls must eventually bind to canonical CharacterId rather than Canary numeric player IDs before native cutover;
- privacy changes must invalidate/rebuild affected public presentation/cache/search state without mutating authoritative game facts.

### Privacy decision authority and monotonic ordering

Privacy authority is Platform-owned and ordered independently from Oteryn-v2 source revisions.

Before native projection/cache cutover, every privacy-controlled public representation must be bound to privacy evidence strong enough to establish, for the affected privacy decision scope:

```text
canonical privacy subject/scope
Platform privacy policy/decision revision or generation
resulting allow/deny/restriction decision
applicability needed to compare it with later decisions
```

The exact persistence field names and encoding are deferred, but the ordering guarantee is not: a later accepted privacy decision must be distinguishable from and dominate an earlier decision for the same scope. Wall-clock response time, cache creation time, game-source revision and projection generation are not substitutes for that monotonic Platform privacy ordering.

A delayed or replayed older `allow` decision cannot overwrite, validate or resurrect output after a newer restrictive decision has been accepted.

### Restrictive change and public-visibility cutoff

A privacy decision that removes or narrows public visibility becomes authoritative at its accepted Platform privacy revision/cutoff. Cache purge, search reindex, projection rebuild and CDN invalidation are propagation mechanisms; they do not postpone the authority of the restrictive decision.

From that cutoff:

- HTTP, API, SSR, search, response caches and CDN/public variants must not serve affected content whose bound privacy evidence is older than the restrictive privacy revision;
- affected privacy-controlled fields are hidden, redacted or explicitly unavailable according to the owning product contract until the serving path proves it enforces the current restrictive revision;
- game-source freshness or stale-while-servable eligibility cannot extend the life of an older public `allow` variant;
- a restrictive decision is considered publicly converged only when every required serving/indexing layer is fenced at the new privacy revision or has equivalent proof that the old variant cannot be served;
- an ambiguous, delayed or failed invalidation/rebuild cannot be recorded as successful convergence.

An expansive privacy change may become publicly visible only after the selected serving path proves the newer permissive decision is applicable. Delaying re-publication is safe; prematurely publishing from an older or unproven `allow` decision is not.

### Invalidation acknowledgement and fail-closed serving

The eventual implementation must define deterministic acknowledgement or an equivalently strong revision fence for every materialized consumer of privacy-controlled output, including the applicable HTTP/API/SSR response cache, search index and CDN layer.

A safe design may use revision-aware cache keys/tags, serve-time privacy fences, versioned presentation generations, explicit purge acknowledgements or another mechanism, but it must preserve these semantics:

- the current restrictive privacy revision is the minimum privacy floor for affected output;
- an old object below that floor is ineligible to serve even if it still physically exists;
- a failed or unknown purge/invalidation state keeps the affected output fail closed and remains observable;
- retry/reconciliation may repeat propagation idempotently without re-authorizing the old object;
- successful convergence is based on enforcement proof, not merely on dispatching an invalidation request.

### Privacy dependency unavailability

Privacy-policy evidence failure is distinct from game-source failure.

When the serving path cannot prove that its privacy decision is current for a privacy-controlled field:

- it must not silently reuse an unproven cached `allow` decision;
- the affected field fails closed to hidden/redacted/unavailable presentation according to its product contract;
- independently public fields whose privacy applicability is already proven may continue under their game-source freshness policy;
- dependency recovery must reconcile to the latest accepted privacy revision before re-enabling affected output.

A cache can therefore preserve game-source availability without becoming a second privacy authority.

## Rebuild and projection generations

A projection must be rebuildable from authoritative evidence rather than requiring manual row repair.

Preferred semantic rebuild model:

1. allocate a new Platform projection generation/schema version;
2. obtain an authoritative baseline/snapshot at a known source revision/high-watermark;
3. build the candidate projection off-path from normal reads;
4. apply the authoritative tail/replay after the baseline watermark when the producer supports replay;
5. validate identity, revision, count/digest/invariant and gap checks available under the source contract;
6. reconcile unresolved gaps before activation;
7. bind/recompose privacy-controlled presentation against the current Platform privacy decision floor and reject activation of a public generation that is below that floor;
8. atomically switch the Platform read pointer/generation only after source and privacy applicability gates pass;
9. retain the prior known-good game projection for bounded rollback, while treating any public presentation derived under an older privacy revision as rollback-ineligible until recomposed or fenced to the current privacy floor;
10. retire old generations under retention policy after cutover evidence is complete.

If replay is not available, the producer contract must provide another bounded authoritative reconciliation/query/snapshot mechanism. Platform must not declare a rebuild complete merely because no more messages arrived.

Game-projection rollback never rolls back the Platform privacy authority. A rollback target that predates a newer restrictive privacy decision may supply known-good game facts, but its privacy-controlled public representation must be recomposed or fenced using the current privacy decision before it can serve.

## High-watermarks, gaps and reconciliation

Platform maintains enough internal provenance per projection family/scope to answer:

- which source contract/schema version produced this state;
- which baseline/snapshot revision was applied;
- the last accepted ordered revision/sequence where one exists;
- whether a gap, invalid record or quarantine blocks full freshness;
- which Platform projection generation currently serves reads;
- which Platform privacy decision revision/floor governs the privacy-controlled representation and whether required invalidation/fencing has converged.

Gap handling:

- detect non-contiguous ordered revisions where the source contract promises continuity;
- mark the affected scope stale/reconciling instead of advancing blindly;
- request/consume the accepted authoritative replay, targeted query or replacement snapshot;
- unrelated independent scopes may continue when causality is not shared;
- reconciliation success must be observable and auditable.

Periodic reconciliation may compare source-defined counts/digests/revisions or bounded authoritative queries. It must not infer equality from matching row counts alone unless the producer contract defines that proof.

Privacy reconciliation is independently monotonic: delayed/out-of-order privacy events below the current privacy floor are rejected as authority, and a failed privacy invalidation remains a privacy convergence failure even when game-source reconciliation is healthy.

## Invalid and poison source records

Malformed, unauthenticated, contradictory or semantically impossible source evidence fails closed.

Required behavior:

- quarantine the exact source record/reference with bounded diagnostic reason;
- do not expose its payload as authoritative public data;
- do not advance a causally required high-watermark past it merely to keep the consumer green;
- preserve independent-scope progress where safe;
- alert/metric the quarantine and projection age;
- recover through corrected replay/replacement snapshot/reconciliation rather than manual fabricated rows;
- logs/artifacts omit credentials, session secrets and private topology.

## Provenance and observability

Internal projection state must preserve sufficient provenance to diagnose freshness and cutover safely.

Recommended observable signals include:

- current projection generation and contract/schema version;
- source revision/high-watermark by family/scope;
- age of last accepted authoritative evidence;
- current privacy decision revision/floor for privacy-controlled scopes;
- privacy invalidation/fence convergence state and oldest unresolved restrictive-change age;
- rejected superseded privacy decision count;
- duplicate/replay count;
- out-of-order/superseded rejection count;
- gap/reconciliation count and oldest unresolved gap age;
- quarantine/invalid record count;
- rebuild duration/result;
- projection switch/rollback events;
- current source authority mode (`native_oteryn_v2` vs explicit legacy compatibility) without exposing private infrastructure to the public API.

Public output should expose only the minimum age/degraded context needed for truthful presentation and must not expose internal privacy/security generations unless a separate public contract requires a safe representation.

## Legacy Canary Compatibility and migration

Current delivered PublicGameData may continue to read Canary SQL/Redis under existing compatibility contracts while native replacement is unimplemented.

Every projection family has an independent cutover gate.

Before switching one family to native authority, prove:

1. accepted Oteryn-v2 producer/source contract and exact revision;
2. canonical identity mapping for all required subjects/scopes;
3. projection schema/generation and idempotent consumer behavior;
4. baseline/rebuild and replay/reconciliation path;
5. freshness, stale, unavailable, invalid, empty/not-found and tombstone negative paths;
6. Platform privacy/presentation overlay correctness, monotonic privacy decision evidence, restrictive visibility cutoff and fail-closed invalidation/fencing behavior;
7. shadow/diff evidence where practical without making the shadow source authoritative;
8. reversible Platform read-source/generation switch that cannot cross a newer privacy deny;
9. exact-revision producer/consumer compatibility evidence.

During migration:

- tag provenance explicitly as `legacy_canary` or `native_oteryn_v2` internally;
- do not merge incompatible records from two authorities into one apparently authoritative row set;
- shadow comparison may report differences but must not silently choose values field by field;
- cutover is per family/scope only after its gate passes;
- rollback returns Platform game-fact reads to the previous known-good source/generation, but privacy-controlled output remains fenced by the newest accepted Platform privacy decision and may not reuse an older `allow` variant;
- a post-cutover legacy fallback, if temporarily retained, must be explicit policy and must not reinterpret Canary IDs as canonical native IDs or bypass the current Platform privacy floor.

Legacy compatibility retirement requires a separately evidenced remaining-consumer review and removal criteria.

## Security boundary

Public projection inputs and outputs are not equivalent trust zones.

Implementation must ensure:

- authenticated/authorized service-to-service source where the producer contract requires it;
- source record/schema validation before projection application;
- public allowlist at presentation time;
- no AccountId, Identity ID, private account association, credentials or security generation in public output unless a separate public contract explicitly authorizes a value;
- no GameSessionId, gameplay lease/fencing material, ticket/pre-admission secrets or session tokens;
- no private GameNode identity, management endpoint, internal route or sensitive topology;
- bounded payload/collection sizes and resource limits;
- safe escaped/typed presentation of game-originated text;
- audit/correlation metadata kept internally where useful without leaking sensitive infrastructure.

## Compatibility with current Platform presentation

The current `PublicCharacterProfileService` pattern contains one architectural property worth preserving during native migration: game facts and Platform-owned privacy/presentation state are composed at the Platform boundary.

The legacy implementation currently keys parts of that composition through `canary_account_id` / `canary_player_id`. Native cutover must replace those compatibility identifiers with accepted canonical identities before treating the path as native.

The current Canary-compatible direct-read path is not classified as defective by this contract repair merely because it does not already persist the future native privacy-revision fence. Its existing composition reads Platform-owned privacy state at the Platform boundary; the monotonic privacy floor defined here is a prerequisite for future native projection/cache/CDN cutover where privacy-controlled variants may be materialized or served asynchronously.

The target is **not** to copy the current SQL schema into a new projection database. The target is to preserve product semantics while changing authority and integration boundaries.

## Versioning and change control

Breaking semantic changes require explicit contract revision when changing any of:

- source-of-truth owner for a projection family;
- stable identity rules;
- privacy/presentation ownership, privacy decision ordering or revocation cutoff semantics;
- stale/unavailable/empty/not-found truthfulness;
- tombstone/lifecycle behavior;
- idempotency/ordering guarantees;
- rebuild/reconciliation requirement, including the rule that game-projection rollback cannot cross a newer privacy deny;
- rule that ordinary public reads do not synchronously depend on game runtime;
- migration rule prohibiting silent mixed authority.

Transport/schema fields may evolve compatibly when these semantic guarantees remain intact.

## Deferred producer/implementation details

The following remain `UNKNOWN` until accepted by owning contracts/evidence:

- exact Oteryn-v2 event/query/snapshot names and wire schemas;
- exact transport or broker;
- exact producer authentication mechanism;
- exact native canonical guild identifier representation;
- exact ranking category/season/ruleset contracts;
- exact per-family numeric freshness/SLA values;
- exact replay retention/window;
- exact Platform projection tables/indexes/storage engine;
- exact Laravel worker/job topology;
- exact persistence/encoding of the monotonic Platform privacy revision/floor;
- exact cache/search/CDN invalidation or serve-time fence implementation that satisfies the frozen privacy semantics above;
- exact staging/production rollout and cutover order.

These unknowns do not authorize shared native SQL, synchronous runtime fallback or weakening the privacy revocation floor.

## Validation requirements before implementation/cutover claims

A future implementation must prove, as applicable:

1. accepted producer/source identity and exact contract version;
2. canonical CharacterId/WorldId/ChannelId validation and accepted stable guild identity where needed;
3. duplicate/replayed input is idempotent;
4. out-of-order/superseded game input cannot overwrite newer game state;
5. baseline + tail/replay or equivalent bounded reconciliation rebuilds one full generation;
6. a detected game-source gap/poison record causes explicit stale/reconciliation state and deterministic recovery;
7. stale/unavailable/invalid game evidence never renders as authoritative empty/zero/not-found;
8. last-known-good game behavior obeys hard freshness limits and never bypasses the current privacy decision floor;
9. rename/delete/restore/transfer/account-transfer scenarios reconcile every affected projection/index;
10. CharacterProfiles/Identity privacy decisions have monotonic/versioned ordering and a delayed, replayed or out-of-order older `allow` cannot override a newer deny;
11. a restrictive privacy change immediately makes every older affected public variant ineligible, including HTTP/API/SSR caches, search and CDN copies, before propagation is declared converged;
12. invalidation/fencing failure or ambiguous acknowledgement keeps affected privacy-controlled output fail closed and produces observable recovery state;
13. cache/search/CDN lag after a deny cannot serve the older `allow` variant, and successful recovery proves the current privacy floor before re-publication;
14. concurrent game-projection refresh and privacy restriction preserves the restrictive decision regardless of arrival/commit order;
15. privacy-policy dependency outage is distinct from game-source outage and cannot silently reuse an unproven cached `allow` decision;
16. projection generation switch or rollback after a deny cannot resurrect presentation below the current privacy floor;
17. normal website reads remain available from Platform projection when the game producer is temporarily unavailable for fields whose game and privacy evidence remain valid;
18. runtime-status aggregate semantics remain consistent with their dedicated contract without duplicate authority;
19. Game Catalog/content authority remains separate;
20. legacy-vs-native shadow evidence is attributable and cutover/rollback never silently mixes authority or bypasses current privacy restrictions;
21. public output contains no private account/session/topology material or internal privacy/security generation unless explicitly authorized;
22. exact-revision producer/consumer E2E passes before any production activation.

## Non-authorization

This contract authorizes no:

- Oteryn-v2 repository or producer implementation;
- Laravel projection model/migration/worker/job implementation;
- message broker/event bus selection;
- replacement or removal of current Canary readers;
- schema/data migration;
- public route/API behavior change;
- cache/search/CDN implementation or configuration change;
- staging deployment;
- production cutover or mutation;
- Game Catalog or runtime-status authority change.