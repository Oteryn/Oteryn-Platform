Status: CANDIDATE revision 1, proposed by Platform on 2026-11-12. Not accepted. The coordinator decided that Game content IDs are the source of truth; the Game side verifies every row below before acceptance. Until then this document authorizes no implementation: the T72 consumer keeps every family listed here unavailable (503).

# T72 journal definition mapping: `TypedDefinitionRef` to the Platform catalogue

## 1. Purpose and boundary

The G-10 items journal (`OTERYN_V2_GAME_JOURNAL_ITEMS_CONTRACT_PROPOSAL.md`, accepted revision 3) and the G-11 progress journal (`OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md`, accepted revision 4) carry Game content references: item definitions, creature races, bosses, quests, achievements, houses and blessings. Both contracts require that these references "resolve against the corresponding pinned Game catalog" and leave unknown mappings unavailable (G-10 *ItemRow*, G-11 §5 and §9). Neither defines the mapping. This document proposes it, family by family.

- **Game owns identity.** Every key and revision below is copied from Game content at the pinned commit in §2. Platform never mints, renames, normalizes, case-folds or aliases a Game key.
- **No wire change.** This document adds no member, type, bound, enum or result code to G-10 or G-11. It only states how Platform resolves bytes those contracts already carry, and what Platform answers when they do not resolve.
- **Fail closed.** A reference that does not resolve exactly is unavailable. Platform never guesses a definition, shows a key as a name, or treats an unresolved reference as empty.
- **No implementation authority.** Implementation waits for acceptance of this document and for a supported catalogue capability (see §4.3).

## 2. Pinned sources

| Side | Repository | Revision | Use |
|---|---|---|---|
| Game content (source of truth) | `Oteryn/Oteryn-Game` | `29eb08d392182a65da0304d8d29019588e1e7186` (`main` on 2026-11-12) | Every key, revision and path in §5 and the fixture |
| Game catalog export contract | `Oteryn/Oteryn-Game` | `669d5af5261a60d347167b7c880a4498aebe72ed` | `CATALOG_EXPORT_V1` as pinned by `portal-rs/crates/world/tests/fixtures/catalog-669d5af5/SOURCE.md`; the export contract and notes are byte-identical at `29eb08d3` (SHA-256 `9bc87fba…1cad` and `495ab72d…abdd`) |
| G-10 / G-11 contract evidence | `Oteryn/Oteryn-Game` | `2253b997` | The accepted contracts' own pin; kept as is |
| Platform consumer | `Oteryn/Oteryn-Platform` | branch `claude/t72-journal-admission` (PR #1834) at `331c5c1a` | `portal-rs/crates/playerdata/src/authority.rs` and `authority/reads.rs`, §3 |

The Game revision was read read-only from raw files at the full SHA. No Game file was changed. Paths are relative to the Game repository root. Source SHA-256 values are in the fixture README.

## 3. Where Platform consumes the mapping

On PR #1834 the T72 admission authority refuses every record that needs a definition or custody mapping, before anything is stored:

- `authority.rs`, `DbLease::validate`: `ITEMS_PAGE`, `MARKET_EVENT` and `PLAYER_TRADE_EVENT` are refused as unproven ("item definitions need the accepted catalog mapping"); `ITEMS_BEGIN` is refused unless `houses` is the empty array; the six G-11 families in `UNPROVEN_PROGRESS` (`bestiary`, `boss_cooldown`, `quest`, `achievement`, `death_recap`, `house_rent`) are refused. The refusal is `Error::Unavailable`, which the ingest route answers with 503 and writes nothing.
- `authority/reads.rs`, `JournalEvidence::definition` returns `Unavailable` for every item; `items::store::Definition` needs `maximum_quantity`, `container`, `tradeable` and an optional estimated price.

When a row of §5 is accepted and a supported catalogue capability carries it, the matching guard can be replaced by the resolution rule of that row. Rows that stay UNKNOWN keep the guard.

## 4. Common rules

### 4.1 Identity form

A Game `TypedDefinitionRef` is `(family, production_key, revision_ref)` (`crates/gameplay/domain/src/domain/item_definition.rs`). Content records carry it as `identity: {family, key, revision}`; some records omit `family` and take it from the shard's top-level `family`. Item instances persist it as `definition_family`, `definition_production_key` and `definition_revision_ref` (`apps/game-server/migrations/0010_item_mint_ground.sql`, each 1–512 bytes).

Resolution compares bytes exactly. Matching on a key alone is allowed only where §5 says the wire carries no revision.

### 4.2 Resolution

A reference **resolves** when the active verified catalogue snapshot (§4.3) contains exactly one entity whose Game identity equals the reference, in a capability that is `supported` and `complete`. It **does not resolve** when:

1. the key or revision is absent from the snapshot;
2. the capability is `unsupported`, `partial` or `unknown`;
3. the key is a Game alias or a retired key (§5.1), even when the alias table names a successor;
4. the snapshot is stale, unavailable, or its source revision is not the pinned one.

A `partial` or `unknown` capability never proves that a key does not exist, so an unresolved reference is "unavailable", never "not found".

### 4.3 Platform catalogue target

Platform's only production catalogue intake is `CATALOG_EXPORT_V1` into `world_catalog_candidates` / `world_catalog_records` (`portal-rs/migrations/20261107120001_world_catalog_candidates.up.sql`, `20261107120002_world_catalog_records.up.sql`). An entity row is `(snapshot_id, record_class = 'entity', record_key = content_key, capability_id)`. The closed capability vocabulary is `item`, `creature`, `creature_loot`, `npc`, `npc_shop`, `spell`, `quest` and `achievement`.

At the pinned export revision **all eight capabilities are `unsupported/unknown`** (Game `docs/reference/CATALOG-CONTRACT-NOTES.md`, *Explicit unresolved capability gate*). So, today, no reference of any family resolves, and every family stays unavailable whatever this document says. There is no capability for houses, blessings, Bestiary entries or Bosstiary entries. The older `world_entities` table (kinds `House`, `Achievement`, from `portal-rs/crates/world/config/game-857f15f3.json`) is test infrastructure with no binary caller and is not a target.

The proposed target for a resolved reference is `world_catalog_records` with `record_key = <Game key>`, which requires `CATALOG_EXPORT_V1` `content_key` to be the Game identity key byte for byte. The export contract's examples (`oteryn:item.currency.crystal_coin`) use a key form that is now a retired alias of a `oteryn:item.tibia.i<id>` key (§5.1). Whether the export carries the canonical key and the revision is UNKNOWN (§7, Q1).

### 4.4 Refusals

| Where | When | Answer |
|---|---|---|
| Ingest (`/internal/v1/playerdata/journal/records`, `/internal/v1/playerdata/progress/record`) | A record carries a reference that does not resolve under §4.2 | **503**, existing unavailable body; nothing written; the stream does not advance. Game retries the exact bytes, as for any 503 (G-10 *Cursor, ordering, replay and failure*; G-11 §7). Not 400: the bytes are valid on the wire. Not 409: no conflict exists. Not a revision-4 terminal refusal: G-11 §8.2 closes that list to `CHARACTER_SCOPE_DELETED`. |
| Ingest | The family's row is UNKNOWN or the family is not yet accepted | 503, as today on PR #1834 |
| Owner read | A stored record whose reference no longer resolves (pin moved, capability became partial) | That entity or snapshot is shown as unavailable with its last update time; no name, value or estimate is invented (G-10 *ItemRow*; G-11 §9 "Unknown catalog revisions suspend the affected entity/family") |
| Valuation (T74) | Item reference does not resolve | No estimate; never zero |

The ingest 503 is a proposal. It holds a whole stream behind one unknown reference until Platform's pin catches up. The alternative, storing the record and keeping only the entity unavailable, is Q7 in §7.

### 4.5 Versioning

- **The pin.** The mapping is bound to one Game content revision: the 40-hex `source_revision` of the active `CATALOG_EXPORT_V1` snapshot, with its `authority_epoch`, `ruleset_id` and `content_profile_id`. A new Game content release reaches Platform only as a new verified snapshot, configured on purpose (`OTERYN_WORLD_CATALOG_*`, `portal-rs/crates/world/README.md`). Platform never follows Game `main`.
- **Stored bytes are kept.** Records keep the exact reference bytes Game sent. A pin change never rewrites them; it only changes whether they resolve.
- **Revision bump.** A Game change that bumps a definition's revision (for example `definition-r1` to `definition-r2`) makes the new revision a different reference. Records under the old revision resolve only while a retained snapshot that contains it is active. Platform does not treat two revisions of one key as the same definition.
- **Retired keys and aliases.** A key that the new pin lists as an alias or as `RETIRED_WITHOUT_SUCCESSOR` (§5.1) stops resolving. Platform does not follow the alias.
- **Within an epoch.** G-11 §9 requires one catalogue interpretation for baseline and increments within a projection epoch. A pin change that removes or bumps a reference used by an open stream leaves the affected entities unavailable until Game sends a new baseline (Q8).
- **This document.** A row moving from UNKNOWN to a rule, a new family or a changed refusal is a new revision of this document, accepted by both sides. Changing the pinned Game SHA in §2 and the fixture is part of that revision.

## 5. Family mapping

Each subsection gives: the reference kind, the Game source (all at `29eb08d3`), the Platform target, the validation rule, the refusal and the versioning note. Refusals and versioning follow §4.4 and §4.5 unless stated.

### 5.1 G-10 `ITEMS` (`ITEMS_PAGE` ItemRow), `MARKET` (`MARKET_EVENT`), `PLAYER_TRADE` (`PLAYER_TRADE_EVENT` ItemLine)

- **Reference kind:** `TypedDefinitionRef` family `Item`, as the wire triple `definition_family`, `definition_production_key`, `definition_revision_ref`.
- **Game source:** `content/items/definitions/items-*.json` (69 shards listed in `content/items/definitions/index.json`, `record_count` 34111), field `/records/*/definition/identity/{family,key,revision}`. Retired keys: `content/items/aliases.json` (decision `A12-ITEM-IDENTITY-TIBIA-ID-V1`, key rule `OTERYN_TIBIA_ID_KEY_RULE_V1`; 33972 `ALIAS` and 4590 `RETIRED_WITHOUT_SUCCESSOR` entries). Instance columns: `apps/game-server/migrations/0010_item_mint_ground.sql`.
- **Observed at the pin** (three shards read: `items-00000-00499`, `items-02500-02999`, `items-03000-03499`): `family` is `Item` (capital I, not `ITEM`); every key has the form `oteryn:item.tibia.i<decimal>`; every revision is `definition-r1`; most records have `materializable: false`. Market enforces `family == "Item"` (`crates/gameplay/economy/src/market.rs`, `Offer::validate`) and refuses keys with a non-production marker (`has_nonproduction_marker`: segments `test`, or `fixture`, `synthetic`, `evidence`, `test-only`).
- **Platform target:** `world_catalog_records`, `capability_id = 'item'`, `record_key = definition_production_key`. None yet: the capability is unsupported at the export pin.
- **Validation:**
  1. `definition_family` equals `Item` exactly.
  2. `(definition_production_key, definition_revision_ref)` equals one item entity's `(key, revision)` in the active snapshot.
  3. The key is not an `ALIAS` or `RETIRED_WITHOUT_SUCCESSOR` key in that pin's alias table.
  4. For ITEMS custody rows, the definition is `materializable: true` (UNKNOWN whether Game can ever export a non-materializable instance; Q2).
  5. `quantity` against the stack rule: `semantics.stack.value.stack_max` when `KNOWN`, 1 when `stackable` is `false`; `stack_class: Unknown` does not resolve. This feeds `Definition::maximum_quantity`.
- **Read-side `Definition` fields:** `maximum_quantity` as in item 5; `container` from `semantics.container` (`KNOWN` with `capacity`, `NOT_APPLICABLE` means not a container, `UNKNOWN` does not resolve); `tradeable` from `semantics.trade_restrictions`, whose value shape is UNKNOWN (Q3). In the three shards read, `semantics.container.state` is `UNKNOWN` for 19 of 21 materializable items, so most items would read unavailable today.
- **Golden note:** the G-10 golden `ITEMS_PAGE` uses `definition_family: "ITEM"` and `oteryn:content.item.backpack` / `rev-1`. The contract marks them synthetic; under this mapping they do not resolve. They stay valid wire goldens, not catalogue examples.
- **Versioning:** a Tibia-id key rule change or a new alias version (the table is append-only) is a new pin.

### 5.2 G-10 `BANK` (`BANK_ENTRY`)

- **Reference kind:** none. BANK carries gold amounts and Game identities only.
- **Mapping:** NOT_APPLICABLE. No catalogue lookup; T72 ingest needs no change for BANK under this document.

### 5.3 G-10 `ITEMS_BEGIN` `houses` and ItemRow `house_id`

- **Reference kind:** not a `TypedDefinitionRef`. Game custody uses `HouseId {world_id, house_key}` with no revision (`apps/game-server/migrations/0025_house_item_custody.sql`, `game_item_house_interior_locations.house_key`).
- **Game source:** `content/houses/houses-*.json` (index `content/houses/index.json`: 995 records, 878 private houses, 66 guildhalls, 51 shops), field `/houses/*/identity/key`; kind at `/houses/*/kind` (`private_house`, `guildhall`, `shop`). The content record also has `identity.revision` (`definition-r1`), which custody does not store.
- **Verified at the pin:** Game's custody CHECK is `house_key ~ '^oteryn:content\.house\.[a-z0-9_]+$'`, at most 512 bytes. Every key in `houses-00000-00499.json` matches it. This confirms the G-10 `house_key` type, which the contract marks "unverified (no game code reference yet)".
- **Platform target:** none yet. `CATALOG_EXPORT_V1` has no house capability.
- **Validation:** `house_key` equals one `identity.key` in the pinned house content, scoped by the envelope `world_id`. Whether every content house exists in every world (`game_house_properties(world_id, house_key)`) is UNKNOWN (Q4).
- **Refusal:** a non-empty `houses` list stays 503, as on PR #1834, until G-10 open question 2 (eligible house custody: owners, co-owners, guests, guild halls) is answered. Key resolution alone does not prove eligibility.
- **Depots (`town_id`):** custody, not a definition reference. The source of the u16 town ids and their Platform name mapping is UNKNOWN; no Platform table maps them. Depot rows carry no catalogue reference and are not gated by this document.

### 5.4 G-11 `bestiary` (§5.1)

- **Reference kind:** `TypedDefinitionRef` family `Creature`, carried as `race_key` plus `race_definition_revision`.
- **Game source:** `content/creatures/definitions/creatures-*.json` (4 shards, 1865 creatures), field `/records/*/definition/identity/{key,revision}`, restricted to records that carry `/records/*/authoring/profile/bestiary` (`kill_thresholds`, `charm_points`, `difficulty`). Game derives races there (`crates/gameplay/domain/src/domain/bestiary.rs`: "a race is the Creature definition key of a definition that carries a Bestiary block"; `crates/gameplay/content/src/content/project/bestiary.rs`, 824 races). Keys are at most 128 bytes. `content/creatures/bestiary/` is `READY_UNPOPULATED` and is not the source.
- **Platform target:** `world_catalog_records`, `capability_id = 'creature'`. None yet: the capability is unsupported, and no capability carries the Bestiary block.
- **Validation:** `(race_key, race_definition_revision)` equals one Creature identity that carries a Bestiary block. `final_kill_threshold` equals the last entry of that revision's `kill_thresholds`. A mismatch at an equal revision means the catalogue and Game disagree: unavailable plus reconciliation, never a correction. `kill_count` above the threshold is kept (G-11 §5.1).
- **Versioning:** a threshold change without a revision bump would be undetectable by key; Q5 asks Game to confirm that it always bumps the revision.

### 5.5 G-11 `boss_cooldown` (§5.2)

- **Reference kind:** `TypedDefinitionRef` family `Creature`, carried as `boss_key` plus `definition_revision`.
- **Game source:** the same creature shards, restricted to records with `/records/*/authoring/profile/details/bosstiary/category` (`bane`, `archfoe`, `nemesis`; 291 bosses per `project/bestiary.rs` tests). Progress storage: `apps/game-server/migrations/0445_character_bosstiary_progress.sql` (`boss_key ~ '^[A-Za-z0-9][A-Za-z0-9_.-]*:[A-Za-z0-9_./-]+$'`, at most 512 bytes, no revision column). `content/creatures/bosstiary/` and `content/encounters/bosses/` are `READY_UNPOPULATED`.
- **Platform target:** `world_catalog_records`, `capability_id = 'creature'`. None yet.
- **Validation:** `boss_key` equals one Creature identity key that carries a Bosstiary category, and `definition_revision` equals that identity's revision.
- **UNKNOWN:** Game stores no revision with boss progress or cooldowns, so what `definition_revision` will carry (the creature identity revision or something else) is UNKNOWN (Q5). Until answered the family stays 503.

### 5.6 G-11 `quest` (§5.3)

- **Reference kind:** `TypedDefinitionRef` family `Quest`, carried as `quest_key` plus `pinned_content_revision` and `definition_hash`; track keys are a second namespace.
- **Game source:** the served quest catalogue is `content/quests/missions/quest-state.json` (schema `OTERYN_QUEST_STATE_LOWERING/v1`, 96 quests), field `/quests/*/quest`, tracks at `/quests/*/tracks/*/key` with `min`, `max`, `initial`. Authoring definitions are `content/quests/definitions/quests-*.json` (`identity.key`, `identity.revision` `quest-r1`). Storage: `apps/game-server/migrations/0056_character_quest_state.sql` (`quest_key ~ '^oteryn:[A-Za-z0-9._:/-]+$'` and `track_key` the same, each at most 128 bytes; `pinned_content_revision` up to 128 bytes; `definition_hash` 32 bytes). Game's own notes say quests are not in the served WorldProject and the loader (`crates/gameplay/quests/src/quest/loader.rs`, `parse_quest_state(…, content_revision)`) is the owner.
- **Platform target:** `world_catalog_records`, `capability_id = 'quest'`. None yet.
- **Validation:** `quest_key` equals one `/quests/*/quest` of the pin; every `track_key` belongs to that quest's `tracks`; `definition_hash` equals the hash Game publishes for that quest at that pin.
- **UNKNOWN:** the production value of `pinned_content_revision` (it is a loader argument, not a content field); how `definition_hash` is computed and where Platform can read it (`crates/gameplay/quests/examples/quest_definition_hashes.rs` exists but no published table); which tracks are owner-visible (G-11 forbids internal flags; the lowering has no visibility field, and the definitions' `shown_in_quest_log` is per quest, not per track). Q6. The family stays 503.

### 5.7 G-11 `achievement` (§5.4)

- **Reference kind:** `TypedDefinitionRef` family `Achievement`, carried as `achievement_key` plus `definition_revision`.
- **Game source:** `content/achievements/achievements-*.json` (index: 571 records, 203 secret, 1 retired), field `/records/*/identity/{key,revision}`; secret flag `/records/*/secret`. Storage: `apps/game-server/migrations/0021_account_achievements.sql` (grants: `achievement_key` with the regex below, `achievement_revision`), `0574_character_achievement_progress.sql`.
- **Verified at the pin:** the G-11 key rule `^oteryn:achievement/[a-z0-9]+(_[a-z0-9]+)*$`, at most 160 bytes, is Game's CHECK on both tables, and every key in `achievements-00000-00499.json` matches it. Revisions there are `"1"`.
- **Platform target:** `world_catalog_records`, `capability_id = 'achievement'`. None yet.
- **Validation:** `(achievement_key, definition_revision)` equals one achievement identity at the pin.
- **UNKNOWN:** the G-2 public export omits secret achievements, so a granted secret achievement will never resolve against the public catalogue (Q9). The source of the character-form `target` (1–100000) is not a content field I found; UNKNOWN.

### 5.8 G-11 `death_recap` (§5.6)

- **`lost_items[].item_key`:** a bare item key, no family or revision. Resolves to the Item family (§5.1) by key alone. Game stores losses as instance ids (`lost_item_ids` in `apps/game-server/migrations/0016_character_death_receipts.sql`, at most 10 per `0536_player_death_item_selection.sql`), so `item_key` is the lost instance's `definition_production_key`. Confirming that, and that a key-only lookup is intended, is Q10.
- **`blessings_before` / `blessings_after`:** Game blessing storage keys, `oteryn:blessing.<stem>` (`rulesets/character/blessings/catalogue.json`, field `/blessings/*/storage_key`; checked by `crates/gameplay/content/src/content/blessing_catalogue.rs`). They are **not** the service keys `oteryn:service.blessing.<stem>` of `content/services/blessings/`. Storage CHECK: `^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$`.
- **Platform target:** none yet for either (no blessing capability; items as §5.1).
- **Validation:** each loss key equals one non-retired item key at the pin; each blessing equals one `storage_key` at the pin.

### 5.9 G-11 `house_rent` (§5.11)

- The family has no accepted payload. The house key would follow §5.3. Mapping: UNKNOWN until G-11 accepts the schema; 503 stays.

### 5.10 Families without a definition reference

`skill_history`, `pvp`, `skull`, `stamina`, `premium_end` and `game_login` carry no catalogue reference. NOT_APPLICABLE; this document does not gate them.

## 6. Summary

| Family / field | Ref kind | Game source at `29eb08d3` | Platform target | State |
|---|---|---|---|---|
| ITEMS, MARKET, PLAYER_TRADE item lines | `Item` triple | `content/items/definitions/items-*.json` `/records/*/definition/identity` | `world_catalog_records` `item` (unsupported) | Rule proposed; blocked by capability |
| BANK | none | — | — | NOT_APPLICABLE |
| ITEMS_BEGIN `houses`, `house_id` | house key (no revision) | `content/houses/houses-*.json` `/houses/*/identity/key` | none (no capability) | Key form verified; eligibility UNKNOWN |
| depots `town_id` | none (custody) | UNKNOWN | none | UNKNOWN |
| bestiary | `Creature` key + revision | creature shards with `authoring/profile/bestiary` | `world_catalog_records` `creature` (unsupported) | Rule proposed; blocked |
| boss_cooldown | `Creature` key + revision | creature shards with `authoring/profile/details/bosstiary` | `world_catalog_records` `creature` (unsupported) | Revision source UNKNOWN |
| quest | `Quest` key, content revision, hash, tracks | `content/quests/missions/quest-state.json` | `world_catalog_records` `quest` (unsupported) | Revision and hash UNKNOWN |
| achievement | `Achievement` key + revision | `content/achievements/achievements-*.json` | `world_catalog_records` `achievement` (unsupported) | Key form verified; secret grants UNKNOWN |
| death_recap losses | item key only | item shards | as ITEMS | Key-only intent UNKNOWN |
| death_recap blessings | blessing storage key | `rulesets/character/blessings/catalogue.json` | none | Rule proposed |
| house_rent | — | — | — | UNKNOWN (no schema) |

## 7. Questions for the Game side

1. Will `CATALOG_EXPORT_V1` `content_key` be the canonical Game identity key byte for byte (for items `oteryn:item.tibia.i<id>`, not the retired `oteryn:item.<category>.<name>` form shown in the export contract's examples), and will each entity carry its `revision`?
2. Can an ITEMS custody row ever reference a definition with `materializable: false`? If not, may Platform treat that as unresolved?
3. What is the value shape of `semantics.trade_restrictions`, and is it the source for "tradeable"?
4. Does every house in `content/houses/` exist in every world's `game_house_properties`, or is the per-world set different? Which house kinds are eligible private custody (G-10 open question 2)?
5. For `bestiary` and `boss_cooldown`: does a threshold or category change always bump the creature identity revision? What exactly will `boss_cooldown.definition_revision` carry, given that boss progress stores no revision?
6. For `quest`: what is the production `pinned_content_revision`, how is `definition_hash` computed, where will Platform read the expected hash, and which tracks are owner-visible?
7. Should an unresolved reference at ingest be 503 (stream held until Platform's pin catches up, proposed here) or should Platform store the record and mark only the entity unavailable?
8. When a Game content release removes or bumps a reference that an open G-10/G-11 stream uses, will Game start a new projection epoch or send a new baseline?
9. A granted secret achievement is absent from the public catalogue. Should Platform show it as "secret, granted" without a name, keep it unavailable, or should Game supply a separate owner-only definition?
10. Is `death_recap.lost_items[].item_key` the lost instance's `definition_production_key`, and is a revision-free lookup intended?
11. What is the source of depot `town_id` values and their names?

## 8. Fixture

`docs/contracts/fixtures/game-journal-definition-mapping-v1/mapping.json` holds a small sample per family, built only from keys and revisions present at Game `29eb08d3`, plus negative cases that must not resolve. Its README names the repository, SHA, paths and the SHA-256 of each source file read. The fixture is a mapping sample, not a catalogue snapshot and not a wire golden.

## 9. Validation scope

Documentation and fixture only; no Rust code changes. Checks: every JSON file parses; every key and revision in the fixture was matched against the pinned Game files; `python3 tools/validation/legacy_token_ratchet.py` reports `ratchet ok`. Runtime/E2E is NOT_APPLICABLE: no behaviour changes until acceptance.
