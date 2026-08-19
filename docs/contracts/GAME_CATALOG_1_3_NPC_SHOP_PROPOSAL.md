# Game Catalog 1.3 NPC and Shop Contract Proposal

Status: architecture proposal only  
Contract: `oteryn.game-catalog`  
Proposed version: `1.3.0`  
Producer: `blakinio/canary`  
Consumer: `blakinio/Oteryn-Platform`

## Decision

Use schema version `1.3.0` for the first additional vertical slice:

- entity `npc`;
- relation `npc_buy_offer`;
- relation `npc_sell_offer`.

Do not include quests, missions, rewards, spawns, raids, map locations or reachability in this version.

Schemas `1.0.0`, `1.1.0` and `1.2.0` remain byte- and semantics-immutable. The architecture task does not register `1.3.0` in Platform configuration and does not authorize import, activation, public visibility, staging or production.

## Proposal artifacts

| Artifact | Purpose | SHA-256 |
|---|---|---|
| `docs/contracts/game-catalog/v1.3/game-catalog-npc-shop-extension.schema.json` | strict Draft 2020-12 validation of the proposed new entity and relation structures | `7e9699ecb04bbc777679e22cee9352ae49ff21220eff7294c042358f01d0571e` |
| `docs/contracts/game-catalog/v1.3/minimal-snapshot.proposal.json` | full synthetic snapshot showing retained item/creature/loot records plus NPC buy/sell records | `b603b4ef1ccbe763d6f5f7565f40d6604027e73101006d359ccfc4aae06f10ca` |

The extension schema is deliberately not the complete canonical `1.3.0` schema. It uses explicit legacy placeholders so architecture can validate only the new structures without copying and accidentally changing the immutable `1.2.0` definitions in this PR.

The Platform consumer implementation must construct the complete canonical schema by retaining all `1.2.0` item, creature, loot, snapshot and release constraints and adding the approved structures. That complete schema will receive its own final SHA-256 and must be byte-identical in both repositories before Canary producer support merges.

## Runtime authority

### NPCs

Canary is authoritative for the final loaded NPC registry:

- `Npcs` owns the final registry keyed by lowercase lookup key;
- Lua `NpcType(name)` resolves through `Npcs::getNpcType(name, true)`;
- each resulting `NpcType` contains final runtime name, type name, description, currency, callback/script and shop-vector state.

The producer must collect from this final registry. It must not implement a second selective parser for Lua or XML and claim runtime equivalence.

A separate Canary authority task must first expose or prove a bounded deterministic read-only iteration boundary. The current registry map is private and only name lookup is public.

### Shop offers

`NpcType::info.shopItemVector` is the confirmed final static registered shop vector. `NpcType::loadShop` stores deduplicated `ShopBlock` values. Each block preserves:

- item ID and runtime item name;
- item subtype;
- buy price;
- sell price;
- storage key and value;
- nested `childShop` records.

`NpcType::info.currencyId` is the exact runtime currency item ID. Canary supports custom item currency, so gold must not be assumed globally.

Canary runtime also supports player-specific `shopPlayers` overrides. Schema `1.3.0` is limited to the static final `NpcType` registry vector. Dynamic session/player-specific offers remain out of scope and must be reported as a completeness limitation rather than silently included or inferred.

## NPC entity

### Identity

Canonical form:

```text
npc:<stable-slug>
```

The canonical key is a reviewed contract identity, not a display-name slug generated independently on every export. The producer must fail on collisions. It must not merge two NPCs solely because runtime/display names match.

The entity additionally carries exact identifiers such as the final registry key. Display text is not the sole identifier.

### Required state

The proposal carries:

- `registry_key`: exact final registry lookup key;
- `runtime_name`: exact final `NpcType::name`;
- `display_name`: nullable reviewed display value;
- `type_name`: exact final runtime type name;
- `name_description`: nullable exact runtime description;
- `aliases`: exact runtime or reviewed aliases only;
- `registration_status`;
- exact currency endpoint and server ID;
- existing common version, completeness, runtime, enabled, identifier and source-path fields.

Aliases are empty when no exact authority exists. A similar name, file name or wiki name is not an alias proof.

### Registration status

Allowed values:

```text
runtime_registered
historical_only
unknown
```

For the first current-runtime producer, exported NPCs are expected to be `runtime_registered`. `historical_only` is reserved for later reviewed historical snapshots and must not be synthesized from current source absence.

### Availability

Allowed values:

```text
encounterable
quest_only
event_only
scripted_only
registered_only
admin_only
unreachable
unknown
```

Registry presence proves only runtime registration. It does not prove a map placement, active script creation point, reachable location or player encounterability. Without separate evidence, current NPC records remain `registered_only` or `unknown`.

Location references and reachability are intentionally deferred to the spawn/map schema slice, which must reuse the existing Unified OTBM World Index and reachability tooling.

### Provenance

Version `1.3.0` reuses existing catalogue provenance rather than adding a second competing structure:

- snapshot Canary commit;
- nullable datapack commit;
- profile and release metadata;
- entity `source_path`;
- namespaced identifiers;
- artifact and schema hashes at transport/import boundaries.

Historical evidence details beyond this current snapshot model remain a later evidence-contract task.

## Shop relations

### Direction

Buy and sell are distinct relation types:

```text
npc_buy_offer
npc_sell_offer
```

One `ShopBlock` with nonzero buy and sell prices produces two relations. A zero price does not produce that direction. The producer must not reinterpret zero as a free public offer.

### Endpoints

Every relation has:

- `source`: an `npc:*` entity in the same snapshot;
- `target`: an `item:*` entity in the same snapshot;
- `data.currency.item`: an `item:*` currency entity in the same snapshot;
- `data.currency.server_id`: the exact runtime currency item ID.

Missing source, target or currency endpoints are blocking dangling references. The producer and consumer must reject the whole candidate rather than omit the relation or invent an endpoint.

### Price and amount semantics

Canary runtime multiplies the stored buy/sell price by the player transaction amount. Therefore the registered offer price is a unit price:

```text
priced_item_count = 1
price_amount = exact ShopBlock buy or sell value
```

`priced_item_count` is fixed to one in this proposal to prevent consumers from guessing bundle semantics. Transaction quantity selected by a player is not static catalogue metadata.

The proposal preserves `item_subtype` separately. No assumption is made that price plus item ID is globally unique.

### Conditions and restrictions

The confirmed current condition fields are the `ShopBlock` storage key and value. The proposal represents them as a nullable exact pair:

```json
{
  "key": 1000,
  "value": 1
}
```

The Canary authority task must prove the runtime sentinel semantics for default `(0, 0)` before the producer maps it to null. Until that proof exists, absence versus exact zero remains an implementation blocker rather than a value to guess.

No additional level, vocation, premium, quest or location requirement may be inferred from NPC text or selected scripts. Newly confirmed runtime restrictions require a later versioned contract or a reviewed bounded field addition before producer emission.

### Nested offers

`ShopBlock::childShop` is preserved through `runtime_path`, an ordered array of zero-based indexes from the root shop vector to the nested block.

Examples:

```text
[0]
[4, 1]
```

A flat row is permitted only because the exact nesting path is retained. Dropping nesting or assigning indexes after nondeterministic reordering is forbidden.

### Canonical relation identity

Canonical forms:

```text
shop:<npc-canonical-key>:buy:<item-canonical-key>:<dot-path>
shop:<npc-canonical-key>:sell:<item-canonical-key>:<dot-path>
```

Examples:

```text
shop:npc:fixture-merchant:buy:item:fixture-shield:0
shop:npc:fixture-merchant:sell:item:fixture-shield:4.1
```

Price is deliberately not part of identity. A price change changes relation data in a new immutable snapshot; it does not create a new logical offer identity. Direction, endpoint and runtime path distinguish offers.

Two records resolving to the same canonical key are a blocking collision. The producer must not silently choose one, sum prices or append a nondeterministic suffix.

### Relation availability

Allowed values:

```text
available
conditional
registered_only
disabled
unknown
```

A relation in a registered static shop vector proves registration, not that a player can reach the NPC, meet the requirement or possess the currency. Current export should normally use `registered_only`, `conditional` when the exact storage condition is preserved, or `unknown` when evidence is insufficient.

An NPC offer alone must not automatically promote:

- the NPC to `encounterable`;
- the item to `obtainable`;
- the currency to obtainable;
- the relation to publicly available.

Those are separate evidence claims.

## Ordering and determinism

The complete canonical contract must preserve existing deterministic rules and add:

- NPC entities sorted by `(type, canonical_key)` with existing entities;
- shop relations sorted by `(type, canonical_key)` with loot relations;
- aliases sorted deterministically when order is not runtime-semantic;
- `runtime_path` preserving runtime vector order;
- identifiers sorted by namespace/value;
- no unordered-map iteration leakage;
- identical inputs plus fixed `generated_at` produce byte-identical JSON and sidecar.

The complete export remains serialized inside the existing dispatcher-safe export operation unless the Canary authority task proves another safe boundary.

## Required semantic validation

Schema validation is necessary but not sufficient. Producer and consumer must additionally verify:

- exact supported contract/version and pinned schema hash;
- entity and relation declared counts;
- unique entity and relation canonical keys;
- all version references and ranges;
- NPC registry-key and canonical-key collision policy;
- buy/sell direction consistent with canonical key;
- canonical path equal to `runtime_path`;
- nonzero exact price for the emitted direction;
- source NPC endpoint exists and is type `npc`;
- target and currency endpoints exist and are type `item`;
- currency server ID resolves to the currency item snapshot identifier;
- deterministic ordering;
- no partial persistence or publication.

The architecture fixture validation also requires rejection tests for duplicate canonical keys, dangling target, dangling currency, canonical/path mismatch and declared-count mismatch.

## Platform persistence proposal

The current importer cannot accept `1.3.0` safely because it maps every non-item entity to creature persistence and every relation to loot persistence.

The consumer implementation must add explicit type dispatch. A bounded typed design is preferred:

- immutable NPC snapshot rows keyed by snapshot and entity;
- immutable shop-offer rows keyed by snapshot and relation;
- direction, relation availability, exact unit price, currency endpoint, subtype, storage condition and runtime path preserved as typed columns or constrained JSON where appropriate;
- persisted typed counts checked against the source document;
- unknown type rejected fail closed.

The implementation task owns the exact migration/table design after current database constraints and query needs are tested. It must not overload creature or loot tables.

## Administrative preview

Before any public work, administrator review should show at least:

- schema version and final schema hash;
- artifact digest and producer/datapack provenance;
- NPC count;
- buy-offer count;
- sell-offer count;
- registered-only, conditional, unknown and unverified counts;
- duplicate/dangling/collision findings;
- diff against the active snapshot;
- rejection reason and audit history.

Import remains inactive. Architecture support does not imply candidate or production activation.

## Public projection

Public NPC and shop routes are a separate task after staging evidence and availability policy review.

The public task must define allowlisted fields, pagination, search, visibility filters, localization/display policy, authorization for admin-only data and the relationship between active profile boundaries and NPC/shop visibility.

No public route is introduced by schema or inactive consumer support.

## Child tasks

1. `OTERYN-20260730-game-catalog-schema-1-3-consumer`
   - complete canonical schema and final hash;
   - parser and semantic validation;
   - typed persistence and counts;
   - transactional inactive import;
   - rollback preservation;
   - administrator preview only.

2. `CAN-20260730-game-catalog-npc-runtime-authority`
   - deterministic read-only registry iteration;
   - static versus per-player shop scope;
   - default storage sentinel proof;
   - stable identity/collision rules;
   - dispatcher/Lua concurrency proof.

3. `CAN-20260730-game-catalog-schema-1-3-producer`
   - byte-identical complete schema and fixture;
   - final-registry NPC/shop collector;
   - exact-value, nested-path, dangling and determinism tests;
   - export-only runtime smoke and previous-output preservation.

4. `OTERYN-20260730-game-catalog-npc-shop-staging`
   - exact artifact/sidecar intake;
   - MariaDB inactive import;
   - diff and candidate review;
   - candidate activation and rollback proof;
   - no production deployment.

5. `OTERYN-20260730-game-catalog-npc-shop-public`
   - separate public projection and UI acceptance;
   - only after staging and reviewed visibility policy.

## Rollout gate

The rollout is `atomic-required` and consumer first:

1. merge this architecture proposal;
2. merge Platform complete `1.3.0` inactive consumer while retaining `1.0.0`–`1.2.0`;
3. merge Canary runtime-authority task;
4. merge Canary producer with byte-identical complete schema/fixture;
5. generate one immutable candidate artifact;
6. prove cross-repository staging import, candidate activation and rollback;
7. separately review public projection;
8. leave production blocked until explicit approval of one exact artifact and deployed commits.

An older consumer must reject `1.3.0` fail closed. Canary must not route `1.3.0` output to an older consumer.

## Explicit unknowns

- safe final NPC registry iteration API;
- whether `(storageKey, storageValue) == (0, 0)` always means no condition;
- complete aliases and reviewed display-name policy;
- dynamic/player-specific shop completeness;
- NPC map placement and reachability;
- complete historical introduction/removal ranges;
- live staging and production snapshot/profile state.

None of these may be filled by assumption.
