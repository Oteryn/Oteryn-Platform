Status: PROPOSED revision 2 by Platform, pending acceptance by the game owner (coordinator).

# Public game projection families — G-4 extension proposal

## Revision 2 changes

Owner review verdict: **ACCEPT-WITH-NOTES**, relayed by the programme coordinator. Deaths, guild, house and skills are accepted in scope; equipment, outfit and achievements are owner-only. This revision remains proposed pending acceptance of the revised text; implementation qualification and activation are separate.

- Remove the activity family and its examples; G-8 online intervals are the single source of online time, with owner-only intervals and public world totals.
- Remove public achievement highlighting and public equipment/outfit preferences.
- Confirm v2 replaces the v1 candidate families, define the shared V2 watermark family enum, and preserve the closed v1 enum.
- Use `400` for every oversize request, correct the EquipmentSlot citation, and record the owner's answers on death identity/window, stable guild IDs/order and skills/equipment/outfit semantics.

## Purpose, authority and consumers

This is a proposed extension of [the public projection contract](OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md), not its replacement. It supplies a concrete producer/consumer proposal for G-4 in [the Rust portal plan](../architecture/OTERYN_PORTAL_RUST_PLAN.md), gate table and §10, including D11. No gate is closed by this document. Acceptance, producer allocation and activation remain separate decisions.

Game owns committed gameplay facts, canonical CharacterId and GuildId, source revisions, the transactional outbox and replay. Platform owns AccountId, WorldId, portal authorization, profile preferences, projection storage, indexing, notifications and privacy enforcement. The game owner/coordinator accepts or revises the producer obligations; the Platform owner accepts the consumer obligations. Neither side can accept for the other.

| Module | Required evidence from this extension | Limit of coverage |
|---|---|---|
| T24b | Skills/progression, owner-only earned achievements/equipment/outfit and houses | Profile visibility and owner authorization remain Platform policy |
| T24d | Deaths and recorded killers | No inferred creature cause or complete PvP kill history |
| T24e | Guilds, ranks and stable member references | No private guild functions |
| T24f | House catalogue/public occupancy | No bids, ACL or housing mutation |
| T48 | Privacy-filtered character/guild/house search | Index never becomes visibility authority |
| T66 | Ordered progression, skills and deaths | All online-time facts and historical calendars/session intervals remain G-8; gaps are explicit |
| T68 | Guild roster and joined profile/progression facts | Current snapshots alone do not prove complete membership history, wars or rankings |
| T70 | Accepted transitions for opted-in watches | Baseline/replay never emits duplicate alerts; presence still uses its existing family |

This contract does not deliver all of T66/T68/T70. Missing history must be unavailable, not reconstructed as if complete. It authorizes no code, workflows, database changes, runtime work, deployment or production enablement.

## References and status of existing conventions

Read together with:

- [Store wire](OTERYN_V2_STORE_WIRE_CONTRACT.md): canonical JSON, bounded internal HTTP, dedicated peer identities, stable retry keys and conflict handling. It is itself PROPOSED; citing its conventions does not accept its commerce semantics.
- [Public projections](OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md): authority, independent privacy floor, rebuilds, tombstones and fail-closed serving.
- [Character Authority commands](OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md): ownership checks, terminal versus ambiguous outcomes, reconciliation by the same operation identity.
- [Entitlement delivery](OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md): no inferred success, no new authority from cached/ambiguous state. No entitlement or commercial data enters this extension.

Pinned Game reference: [Oteryn/Oteryn-Game at 857f15f376907c592bb28d854fccff18a15cdf87](https://github.com/Oteryn/Oteryn-Game/tree/857f15f376907c592bb28d854fccff18a15cdf87). All Game paths below refer to that commit, inspected read-only outside the Platform checkout:

| Game path | Evidence relied on |
|---|---|
| `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md`, §§2–8, 11–12 | Contract ID `oteryn-game-public-projections-v1`; push/mTLS; snapshots, epoch fence `F_pub`, watermarks, operator resync, privacy. Profiles/highscores/presence/watermark accepted for testing/preproduction; deaths/guild/house still CANDIDATE |
| `crates/gameplay/native-admission-source/src/public_projections.rs` | `Family`, `ProfileSnapshot`, `ProfileEntry`, `ProfileGuild`, `PublicSink`, `PublicProjectionDescriptor`, encoders, compiled paths, retry/hold bounds; purpose `OTERYN_GAME_PUBLIC_PROJECTIONS` |
| `crates/gameplay/durability/src/durability/public_projections.rs` | `PublicProjectionsStore`, `PROFILE`, `CLEAR_PROFILES`, `public_outbox_facts`, committed-cut reads and acknowledged-revision clearing |
| `apps/game-server/migrations/0091_public_projections.sql` | Existing epoch, revision/outbox and resync storage, not a schema to copy into Platform |
| `crates/gameplay/native-admission-source/src/character_commands.rs` | Existing `CreateCharacter`, command pull paths, `PendingEntry`, `Decision`, `Consumer`, mTLS purpose `OTERYN_GAME_CHARACTER_COMMANDS` |
| `crates/gameplay/durability/src/durability/character_command.rs` | Durable command decision/recovery boundary; public facts are not command receipts |
| `apps/game-server/migrations/0016_character_death_receipts.sql` | `death_occurrence_id`, `level_before`, committed receipt; milliseconds converted to seconds |
| `apps/game-server/migrations/0009_character_progression.sql`, `0030_character_build_state.sql` | `total_experience`, CharacterRevision, `magic_level`, seven named skills; base levels rather than temporary combat modifiers |
| `crates/gameplay/domain/src/domain/equipment.rs`, `crates/gameplay/domain/src/domain/death.rs`, `crates/gameplay/durability/src/durability/character_equipment.rs` | `EquipmentSlot` is defined at `crates/gameplay/domain/src/domain/death.rs:96`; equipment records use `EquippedItem` and `TypedDefinitionRef` and exclude instance/custody identities |
| `crates/gameplay/domain/src/domain/appearance.rs`, `apps/game-server/migrations/0093_character_appearance.sql` | `AppearanceSelection`, `DisplayedLook`, `outfit_key`, four `colours`, `addons`, optional `mount_key`; selected and displayed appearance differ |
| `docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md`, `OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md`, `crates/gameplay/durability/src/durability/account_achievement.rs` | Account-scoped earned facts, stable achievement keys, secret-unearned suppression; website export is not already authorized |

Existing Platform `legacy/app/GameProjections/{PublicProjectionFamily,PublicProjectionWire,PublicProjectionSnapshot,PublicProjectionWatermark,PublicProjectionIngestion}.php` corroborate the ingest boundary. Game and Platform both accept canonical decimal-string Unix times; Platform's `NativeAccountCharactersWire::unixTime` validates that string before converting to an internal integer. This proposal keeps that wire encoding and requires shared byte fixtures, without permissive coercion.

## Flow direction and transport

| Logical flow | Network direction | Contract action |
|---|---|---|
| Game → Platform fact ingest and watermark | Game POSTs to Platform internal listener | Complete replacement snapshots; no game mutation |
| Platform → Game command | Existing Game-initiated pending/read exchange delivers Platform intents; Game POSTs receipts | No new command or command variant in G-4; existing commands retain their own contract and operation IDs |
| Replay/resync | Game operator runs existing resync, Game republishes to Platform | No Platform-to-Game HTTP call or new resync command |
| Player visibility preferences | Browser → Platform under authenticated owner authorization | Platform preferences only; not a gameplay command |

Reuse the public-projection TLS 1.3 mutual-authentication listener, pinned server identity/trust, dedicated public-projection client identity and configured `source_authority` allowlist. Trust TLS metadata only from the qualified terminator, never request headers. Deny identity overlap with command, restore, entitlement/Store, native evidence, runtime-status and account-character surfaces. No browser calls ingest. No shared SQL, new broker, bearer token, HMAC or message-signature envelope is proposed. Existing projection and command HTTP exchanges authenticate through mTLS; signatures on other native evidence profiles remain confined to those profiles. A content digest is an equality check, not a signature or credential.

Reuse connect 1 s, handshake 2 s, exchange 5 s bounds and the existing publisher's stricter 3 s whole-exchange deadline, `Connection: close`, one in-flight publication per family/publisher. Responses are at most 256 bytes, `Cache-Control: no-store, private`, `Pragma: no-cache`. Apply existing per-identity/per-route limiting (configured 1–6000 requests/minute, 60 s window); `429` has `Retry-After` seconds ≥1. No arbitrary endpoint URL from payload data.

## Version and routes

Proposed extension profile: `oteryn-game-public-projections-v2`, `contract_version: 2`. V2 replaces the v1 deaths/guild/house candidates; it does not create parallel serving authority for those candidates. Existing accepted v1 paths, schemas, statuses, clients and the closed v1 watermark family enum remain unchanged. A new version is necessary because v1 has closed exact member sets. These are proposed compiled POST paths, not deployed routes:

| Family | Operation | Path under `/internal/v2/game-public/` | Maximum request bytes |
|---|---|---|---:|
| `character_deaths` | `PublishCharacterDeathsV2` | `character-deaths` | 32768 |
| `guild` | `PublishGuildV2` | `guild` | 393216 |
| `house` | `PublishHouseV2` | `house` | 2048 |
| `character_skills` | `PublishCharacterSkillsV2` | `character-skills` | 4096 |
| `character_achievements` | `PublishCharacterAchievementsV2` | `character-achievements` | 262144 |
| `character_equipment` | `PublishCharacterEquipmentV2` | `character-equipment` | 8192 |
| `character_outfit` | `PublishCharacterOutfitV2` | `character-outfit` | 2048 |
| watermark | `PublishPublicProjectionWatermarkV2` | `watermark` | 512 |

Guild bound increases to cover stable UUID member references. Achievement capacity is deliberately bounded; overflow refuses the whole snapshot, never truncates it. Do not redirect v1 callers to v2. Capability/version agreement is explicit per family and environment. Unknown version, operation, field or enum is rejected. Any change to a member, bound, response or meaning needs a new version after acceptance. This document proposes no permissive unknown-field parsing or version negotiation endpoint.

## Exact encoding and shared types

One raw UTF-8 JSON object, no BOM, trailing newline, insignificant whitespace, duplicate members, string escapes or `null`. Object members appear in the order specified below. Re-encoding with unescaped Unicode and slashes must produce the same bytes. At most five nested containers, as in Game v1; no floats, exponent notation or booleans standing in for integers. All members are required except the explicitly listed status-dependent members. Optional values use a 0..1 array, consistent with existing profile/guild/house records; `[]` is not a substitute for an unknown authoritative value.

`Content-Type: application/json`; `Transfer-Encoding` absent; `Content-Encoding` absent or `identity`. Content-Length, if present, is canonical nonnegative decimal, at most ten digits and within route bound. Actual nonempty body must also fit. Over-limit Content-Length, oversized actual bodies and malformed/oversize decoded wire all return `400`, aligned with G-8 and G-12. Refusals have empty bodies and no side effects.

| Type | Exact form, units and bounds |
|---|---|
| `id` | Lowercase non-nil UUIDv7: `[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}` |
| `u64` | JSON string, canonical decimal 1..18446744073709551615 |
| `u0` | Same with `"0"` allowed |
| `time` | `u0`, Unix seconds, ≤9223372036854775807; `time+` excludes zero. Source milliseconds use floor(ms/1000), never rounded up |
| `amount` | `u0` ≤9223372036854775807; gold units, XP units or elapsed seconds as specified |
| `int[a,b]` | JSON integer, inclusive range, canonical unsigned decimal |
| `authority` | Configured namespace, 1..128 ASCII bytes from `[A-Za-z0-9._:/-]` |
| `name` | Existing projection name rule: 1..64 UTF-8 bytes, NFC, no Cc/Cf, quote or backslash; Game producer's printable-ASCII/name-policy subset, no slash or dot-only names |
| `key` | Stable content key, 1..128 ASCII bytes from `[a-z0-9._:-]`; never display name or client numeric sprite ID |
| `revision_key` | 1..128 ASCII bytes matching `[A-Za-z0-9][A-Za-z0-9._:-]{0,127}` |
| `achievement_key` | `oteryn:achievement/` followed by 1..141 lowercase ASCII letters, digits or underscores, total ≤160 bytes; must resolve in Game catalogue |

Every snapshot has this exact ordered head: `contract_version` (integer 2), `operation` (route operation), `source_authority` (`authority`), then the subject members, then `projection_epoch` (`u64`), `projection_revision` (`u64`), `source_observed_at` (`time`), then the body member. Times are not ordering keys. Source observations more than 1 s into the future are refused. All identities and revisions are internal evidence, never public URL/API fields.

Subject members for G-4 snapshots: `character_id: id` for each G-4 `character_*` family; `guild_id: id` for guild; `world_id: id, house_key: key` for house. Character/Guild IDs are Game-owned; WorldId is Platform Registry-owned. House identity is exactly `(world_id, house_key)`.

A body is complete for its defined scope. `skills`, `achievements`, `equipment`, `outfit`, `guild`, `house` each hold 0..1 record. Zero records means absent/ineligible/hidden subject and immediately suppresses earlier data. A present subject with no achievements/equipment has a present record with an empty inner list. Deaths use their separate list convention below. Unknown dependencies stall publication/watermark or publish a safe suppression, never a fabricated empty known result.

## Family records

### Deaths with killers

Body `deaths`: 0..20 records in descending `(occurred_at, death_occurrence_id)` order; occurrence ID breaks same-second ties. Each record, in order:

| Field | Type and rule |
|---|---|
| `death_occurrence_id` | `id`, existing receipt identity; unique within character and immutable across replay |
| `occurred_at` | `time+`, receipt committed time converted from milliseconds |
| `level` | `u64` ≤4294967295, `level_before` |
| `pvp` | JSON boolean, committed classification |
| `killers` | 0..16 `name` strings; final-blow character first, then damage contributors and assist-only groups, each in name byte order |

Keep Game's historical-at-commit killer names; never resolve them to a current holder or infer account identity. Creature/environment causes are absent until a durable source and versioned amendment are accepted. An empty killers array means no recorded public character killer, not proof of an environmental cause. Hidden/deleted subjects publish `deaths: []`; serving also requires the current eligible profile and privacy floor. Read-time window is 30×86400 s. This is the latest 20 deaths within that window, not proof of a complete 30-day ledger or 10-week kill history. Occurrence IDs support deduplication for T70; no alert is emitted for baseline/imported historical records.

### Guilds and members

Body `guild`: 0..1, with exact record members:

`world_id: id, name: name, state: FORMING|ACTIVE|DISBANDING, founded_at: time+, ranks: array[3..20], members: array[0..2000]`.

Each rank: `level: int[1,20], name: name`, strictly ascending unique level. Each member: `character_id: id, name: name, rank_level: int[1,20], joined_at: time+`. Member IDs are unique; rank must exist; order is `(rank_level, character_id)` ascending, matching Game's stable GUILD-0 keyset convention. Member name is current presentation, never the join key. This deliberately versions the candidate v1 name-based member shape.

Disbanded, unclassifiable or hidden guild publishes `guild: []`. A valid guild with no visible members keeps its record and `members: []`. Exclude ineligible character profiles; Platform additionally filters each member through current profile/guild visibility before aggregating or indexing. Vocation/level/experience come from current same-World profile/progression by CharacterId; stale/missing joins do not count as zero. Aggregate only the permitted cohort and label it accordingly, never leak hidden members through totals. Full historical joins/leaves and authoritative guild rankings require separate retained evidence; successive roster snapshots are observations, not a complete event log.

### Houses

Body `house`: 0..1. Preserve the candidate Game v1 record exactly, with these ordered members:

`name: name, kind: private_house|shop|guildhall, town: key, size_sqm: u64, beds: u0, rent_gold: u64, status: VACANT|AUCTION|OWNED|MOVE_OUT_PENDING|UNAVAILABLE`, followed only by the applicable extras:

| Status | Exact extra members in order |
|---|---|
| `VACANT`, `UNAVAILABLE` | none |
| `AUCTION` | `current_bid: amount, auction_ends_at: time+` |
| `OWNED` | `owner_kind: CHARACTER\|GUILD, owner_name: name, paid_until: time+` |
| `MOVE_OUT_PENDING` | `owner_kind: CHARACTER\|GUILD, owner_name: name, paid_until: time+, move_out_at: time+` |

`size_sqm` is whole square metres, beds is a count, rent/current bid are whole gold units, each gold value ≤9223372036854775807. Retired or catalogue-absent house publishes `house: []`; disposition, hidden owner or ambiguous owner publishes `UNAVAILABLE`. Catalogue identity and static definitions remain Game Catalog authority. Preserve Game's name-handoff barrier; owner name is display only and cannot authorize owner-only access, commands or a persistent ownership join.

### Skills and progression

Body `skills`: 0..1 record, ordered:

`world_id: id, character_revision: u64, level: u64 (≤4294967295), total_experience: amount, magic_level: int[0,1000], fist_level: int[10,1000], club_level: int[10,1000], sword_level: int[10,1000], axe_level: int[10,1000], distance_level: int[10,1000], shielding_level: int[10,1000], fishing_level: int[10,1000]`.

Names match `game_character_build_state`; progression is from `game_character_progression_state`. All values are from one committed cut, excluding temporary equipment/spell/loyalty modifiers. Missing build rows may use only Game's established seed (magic 0, seven skills 10); missing/unreconciled progression is not zero XP. No raw tries, mana-spent or training internals are sent. T66 may retain permitted samples with explicit coverage dates; interpolations/predictions are labelled derived, never Game facts. Deaths can lower values; do not enforce monotonic level or XP.

### Earned achievements

Body `achievements`: 0..1 record, ordered:

`world_id: id, character_revision: u64, catalogue_revision: revision_key, earned: array[0..1024]`.

Each earned row: `key: achievement_key, earned_at: time+`, strictly ascending unique key. Only earned facts; no secret-unearned keys, hidden progress, grant provenance, granting character or account identifiers. Names, grade and points resolve from the exact compatible catalogue revision, not copied client text. Missing catalogue compatibility suppresses display rather than inventing points. A record over 1024 earned facts requires a new bounded paging/version proposal, not truncation.

**Owner-only.** The underlying Game facts are account-scoped; this envelope is a proposed character-scoped delivery view, not a claim that the character earned each fact. Current owner authorization is required for the earned list. No public exposure of achievement keys, earned times or totals is permitted; highlighting, account groupings and cross-character inference are prohibited. Export remains subject to qualification of the attribution/ownership safeguards below. Account transfer invalidates the delivery view before the new owner can see it; do not carry a previous owner's account facts with a sold character. Game binds `character_revision` to the current ownership relation read in the same committed cut. Platform must prove that relation still applies and that this revision meets the durable ownership-transition revision floor from Character Authority, independently of projection ordering. Until that floor and its binding can be established, deny the earned-list view even to a currently authenticated owner; a fresh family watermark alone cannot validate an older owner's snapshot.

### Equipment

Body `equipment`: 0..1 record: `world_id: id, character_revision: u64, slots: array[0..10]`.

Each slot record: `slot: enum, item_key: key, item_revision: revision_key`. Enum/order follows Game `EquipmentSlot`: `Head`, `Necklace`, `Container`, `Armor`, `RightHand`, `LeftHand`, `Legs`, `Feet`, `Ring`, `Ammo`; each appears at most once and records follow that order. Include occupied primary slots only, never duplicate a two-handed item into its reserved slot. An empty slots list means known unequipped. Resolve `item_key/item_revision` from `TypedDefinitionRef` and catalogue evidence; never infer from an instance ID. No instance ID, quantity, contents, charges, inscriptions, imbuements, valuation, durability, custody state or hidden modifiers. Read only committed equipment custody, never pending move intent. Unknown custody suppresses the record. Owner-only; no public equipment view is permitted.

### Outfit

Body `outfit`: 0..1 record: `world_id: id, character_revision: u64, content_revision: revision_key, outfit_key: key, colours: array[4] of int[0,65535], addons: int[0,255], mount: array[0..1]`.

Colours are head, body, legs, feet palette indices in that order, matching `AppearanceSelection`. Each mount entry has only `mount_key: key`. Export Game's public `DisplayedLook`, not unlock inventory or an uncommitted selection. While mount display is deferred, `mount` is empty even if selection stores a mount. Unsupported outfit/content, palette or addon combination suppresses the record; bounds alone do not prove content compatibility. Owner-only; no public outfit view is permitted.

### Online time boundary (G-8)

G-4 defines no activity family or last-login/cumulative-time schema: there is no verified Game source for the removed proposal. [G-8 online intervals](OTERYN_V2_STATISTICS_FAMILIES_CONTRACT_PROPOSAL.md) are the single source of online time. Character intervals are owner-only; only world totals may be public. No independent G-4 counter, public character online-time view or integration of `world_online` scans/watermark age may supply authoritative online time. G-8 owns source accounting, privacy, retention and qualification.

## Identity, delivery, ordering and replay

Idempotency is the existing projection key: `(source_authority, family, subject, projection_epoch, projection_revision)`. Source authorities are statically partitioned to prevent two publishers claiming the same subject. Do not add a random request UUID, receipt ledger or timestamp-based identity. Body content means subject members plus body; exclude common envelope, revision pair and source_observed_at. Equal-pair comparison uses SHA-256 over recursively member-sorted canonical content with list order preserved, as Platform ingestion does. Equal content accepts without refreshing its first observation time; different content is `409`, preserving the first record.

Per-subject pairs order lexicographically as integers. Lower pairs are superseded; revision gaps are allowed because these are full replacement snapshots and outbox coalescing is intentional. A missing expected subject, undelivered change or failed baseline is a completeness gap, not a synthetic tombstone. No cross-family atomicity is implied. Joins require current same-epoch, same-World eligible evidence; never assume equal revision numbers across families represent the same cut.

Each change of any exported field, eligibility or source visibility touches revision and outbox in the same authoritative fenced transaction. Achievement owner changes, equipment moves, appearance changes, guild membership/rank/name changes, and house occupancy/catalogue changes all need explicit source triggers. Publisher reads current revision/body together, retries without modifying a sent revision's content, and clears only outbox entries at or below the acknowledged revision. A later committed revision remains queued. Publication errors never roll back gameplay.

Reuse `F_pub`: only the Game operator raises epoch after restore/rollback, strictly above the persisted external fence. All v1/v2 family views share that authority epoch; a higher observed epoch fences every older family immediately. Game publishes nothing below its fence. Resync includes every subject and required suppression/tombstone, not just changed or present subjects; absence from a scan alone deletes nothing. During rebuild, baseline and tail reconcile off-path; activate only after completeness, same-epoch watermark and current privacy floors are proven. A prior generation cannot restore an older privacy allow.

Replay is authoritative full-state resync through the existing operator mechanism, not a new event archive or Platform command. Existing 60 s stopped-node resync deadline is preserved; adding families requires owner qualification of that bound before activation. Outbox retention is until accepted/superseded; poison rows stay queued. No promise of arbitrary historical event replay is made. A recovering Platform obtains a new full baseline plus tail and checks source-defined subject coverage/digests; row counts or silence alone prove nothing. T70 deduplicates on occurrence IDs or `(family, subject, epoch, revision, notification-kind)`, binds current privacy, and suppresses initial/rebuild/replayed notifications.

### Watermark schema

Exact ordered members:

`contract_version: 2, operation: PublishPublicProjectionWatermarkV2, source_authority: authority, family: V2Family, projection_epoch: u64, complete_through: time, observed_at: time`.

This contract defines the closed shared `V2Family` enum: `character_deaths`, `guild`, `house`, `character_skills`, `character_achievements`, `character_equipment`, `character_outfit`, `character_online_sessions`, `world_daily_kills`, `world_daily`. The first seven are defined here; `character_online_sessions` (the online-interval family) and `world_daily_kills` belong to [G-8](OTERYN_V2_STATISTICS_FAMILIES_CONTRACT_PROPOSAL.md), and `world_daily` belongs to [G-12](OTERYN_V2_WORLD_DAILY_FAMILY_CONTRACT_PROPOSAL.md). All new public families use `/internal/v2/game-public/*`, `contract_version: 2` and the V2 watermark route/operation above. Those contracts own their payloads, bounds and subject scopes; this enum grants no acceptance or activation of them. The v1 watermark enum remains closed and unchanged; never send these extensions to v1. Unknown V2 enum values return `400`.

Same meaning as Game v1: no committed change at/before complete_through remains undelivered; never advance past a poison row, failed delivery, incomplete resync or failed accounting scan. `complete_through ≤ observed_at`; neither may exceed Platform now+1 s. Emit at least every 10 s while healthy. Observation time is not completeness. A same/lower complete_through in the same epoch is superseded, never refreshes freshness.

Testing/preproduction freshness remains S=30 s with clock uncertainty 1 s. At every serve, cache, search and notification dispatch, deny if `now - complete_through + 1 > 30`, complete_through is more than 1 s in the future, epoch is fenced, source is unknown or privacy evidence is unproven. There is no stale-serving window. Production S, PKI, resync capacity and replay/recovery evidence require separate acceptance.

## Failure, refusal and timeout semantics

Responses use the existing closed projection vocabulary, with version 2: `200 {"contract_version":2,"result":"accepted"}` or `200 {"contract_version":2,"result":"superseded"}`. Both acknowledge only projection processing, never gameplay success or public visibility. Every non-200 body is empty.

| Condition | Result and required behavior |
|---|---|
| Disabled, misconfigured, unsupported environment, storage/dependency unavailable | `503`; no acknowledgement or authority |
| Wrong certificate/source authority | `401`; no storage; configured cross-purpose identity overlap disables the surface with `503` before authentication |
| Invalid schema/member/type/version/enum/bounds or future time | `400`; no storage |
| Equal key with conflicting content | `409`; preserve first, alarm, stall completeness |
| Rate refused | `429`, Retry-After; retain outbox |
| Timeout, disconnect, unexpected/oversize response | Unknown acknowledgement; retry same content/key; do not clear outbox |

Reuse Game's 1 s exponential backoff doubling to 60 s for `429`, `503`, timeouts/transport errors; honor a longer Retry-After. Definite `400/401/409` keeps row queued, alarms and holds that subject for 300 s (at most one retry per five minutes), as existing definite refusal handling does. No rewriting or truncation to make a poisoned revision pass. Quarantine diagnostics contain family/result class/timing only, no payload.

Unknown or ambiguous state denies access, disclosure, alerts and claimed completion. Missing, stale, disabled and invalid are unavailable, never zero, offline, empty success or not-found evidence. An authoritative eligible empty snapshot is distinct. Public unknown/hidden/deleted profile lookups use the same not-found response without existence-dependent timing; family outages use unavailable. Existing Platform → Game mutations retain their command operation identity and pending/ambiguous reconciliation: no G-4 snapshot, timeout or retry creates a new command or proves a mutation committed.

## Privacy, owner access and retention

Service ingest permission is not public-read permission. Platform stores normalized facts separately from public materializations. Current authenticated owner-only access uses server-resolved AccountId and current accepted Character Authority evidence; public projections, displayed owner names and stale account-character mappings never prove ownership. Ownership ambiguity or transfer denies the owner view. Preferences require normal authenticated authorization/CSRF protection and remain Platform-owned.

For every privacy-controlled field, require both Game eligibility and the current monotonic Platform privacy decision. New sensitive families default hidden; equipment, outfit and achievements remain owner-only regardless of public preferences. A restrictive decision establishes an immediate serve-time privacy floor across HTTP/API/SSR, cache/CDN, search, aggregates and queued notifications. Older allows cannot resurrect through replay, rollback, cache lag or fresh Game data. Failed purge/fence acknowledgement keeps output hidden. Missing policy dependency denies affected fields. Unknown or hidden member/killer identities are never guessed from names. No account co-listing, account grouping or cross-character achievement fingerprinting is permitted.

Proposed Platform retention ceilings, subject to owner acceptance:

| Data | Retention and purge |
|---|---|
| Normalized current skills/guild/house/equipment/outfit/achievements | One current snapshot per subject; superseded bodies purged within 24 h; no automatic history archive |
| Death bodies | At most latest 20 and 30 days; remove expired entries within 24 h and filter at serve time immediately |
| Permitted T66/T68 statistical samples | Maximum 365 days, opt-in and explicitly labelled coverage; achievement lists, equipment/outfit and G-8 interval values excluded from this history |
| Suppression/tombstone and anti-replay fences | Minimal subject/key/revision/digest retained while the feed can replay; purge bodies, never expire a fence while old evidence remains admissible |
| Diagnostic logs | 14 days, family/result/timing only; no names, IDs, payload bodies or owner association |
| Restricted/deleted/ownership-transferred public materializations | Fence immediately; remove stored variants within 24 h, backup copies expire within 30 days; restored backups reapply current fences before serving |

Outbox stays Game-owned until acknowledgement; Platform retention does not authorize deleting authoritative Game receipts. Removal/opt-out blocks presentation/alerts immediately and deletes optional history within 24 h. Backups remain inaccessible to product reads. Resync cannot import history beyond accepted retention/consent. Retention expiration does not turn missing evidence into an authoritative empty result.

Never send AccountId, Identity ID, other-character links, account achievements' granting character/source event, payment/entitlement/Wallet details, guild titles/motd/balance/invites/ACL, house bidder identities/maxima/interior/ACL, item-instance/custody identifiers, private text, credentials, certificate keys, session/lease/fencing tokens, IP/device data, ChannelId, node/map/position or private topology. No raw session intervals in G-4. Canonical internal subject IDs are not emitted in public URLs, pages, APIs or errors. Public lookups follow existing name-handoff/ambiguity rules, houses use World plus stable house key; reused names do not inherit old records.

## Default-off rollout and compatibility

Reuse Game `[platform.public_projections] enabled = false` and Platform `game-projections.public.enabled = false` as prerequisites, never changing existing defaults. Proposed additional per-family v2 producer, ingest and serving flags all default false; a missing flag is false. Equipment, outfit and achievement export each have independent owner-only activation. Visibility/owner preferences are additional gates. This proposal does not name new environment variables or authorize configuration changes.

Enable only on exact compatible revisions after both owners accept that family, its source semantics and shared fixtures. Shadow mode stores isolated evidence with no public output or alerts. v1 and v2 may coexist in shadow but only one family/version is serving authority per scope/generation. Do not merge v1 names with v2 IDs as partial authority. Rollback disables affected serving or uses an accepted compatible generation under current epoch/privacy floors, never lowers fences or silently falls back to shared tables/live queries. Acceptance of one family does not activate the rest or close all of G-4.

## Golden payloads

Each fenced line is one complete wire body (the fence newline is not part of transmitted bytes). Synthetic IDs and names only. These are proposed fixtures, not evidence that current encoders accept v2.

Valid death with recorded final-blow killer:

```json
{"contract_version":2,"operation":"PublishCharacterDeathsV2","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"3","source_observed_at":"1790000000","deaths":[{"death_occurrence_id":"01934f10-7c05-7001-805b-3b1122334401","occurred_at":"1789990000","level":"413","pvp":true,"killers":["Beren","Mira"]}]}
```

Valid guild, including stable member identity:

```json
{"contract_version":2,"operation":"PublishGuildV2","source_authority":"oteryn:game:primary","guild_id":"01934f10-7c06-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"5","source_observed_at":"1790000000","guild":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Red Rose","state":"ACTIVE","founded_at":"1789000000","ranks":[{"level":1,"name":"Leader"},{"level":2,"name":"Vice Leader"},{"level":3,"name":"Member"}],"members":[{"character_id":"01934f10-7c04-7001-805b-3b1122334401","name":"Aldric","rank_level":1,"joined_at":"1789000000"}]}]}
```

Valid vacant shop with zero beds:

```json
{"contract_version":2,"operation":"PublishHouseV2","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","house_key":"oteryn:content.house.example-shop","projection_epoch":"1","projection_revision":"12","source_observed_at":"1790000000","house":[{"name":"Example Shop","kind":"shop","town":"oteryn:content.area.city.example","size_sqm":"24","beds":"0","rent_gold":"5000","status":"VACANT"}]}
```

Valid committed skills/progression:

```json
{"contract_version":2,"operation":"PublishCharacterSkillsV2","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"7","source_observed_at":"1790000000","skills":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","character_revision":"19","level":"412","total_experience":"11400000000","magic_level":12,"fist_level":10,"club_level":10,"sword_level":10,"axe_level":10,"distance_level":90,"shielding_level":50,"fishing_level":10}]}
```

Valid owner-only earned-list view:

```json
{"contract_version":2,"operation":"PublishCharacterAchievementsV2","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"8","source_observed_at":"1790000000","achievements":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","character_revision":"19","catalogue_revision":"fixture-v1","earned":[{"key":"oteryn:achievement/fixture_earned","earned_at":"1789990000"}]}]}
```

The synthetic catalogue keys in these examples require matching synthetic catalogue fixtures for semantic validation; they are not assertions that production content exists.

Valid occupied primary equipment slot:

```json
{"contract_version":2,"operation":"PublishCharacterEquipmentV2","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"9","source_observed_at":"1790000000","equipment":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","character_revision":"19","slots":[{"slot":"Head","item_key":"oteryn:item.fixture-helmet","item_revision":"fixture-v1"}]}]}
```

Valid displayed outfit, no displayed mount:

```json
{"contract_version":2,"operation":"PublishCharacterOutfitV2","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"10","source_observed_at":"1790000000","outfit":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","character_revision":"19","content_revision":"fixture-v1","outfit_key":"outfit.citizen","colours":[1,2,3,4],"addons":0,"mount":[]}]}
```

Valid skills watermark:

```json
{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"character_skills","projection_epoch":"1","complete_through":"1790000000","observed_at":"1790000003"}
```

Valid suppression of the earlier skills snapshot:

```json
{"contract_version":2,"operation":"PublishCharacterSkillsV2","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"8","source_observed_at":"1790000001","skills":[]}
```

Invalid: zero epoch, wrong revision type, null body; `400`, no state change:

```json
{"contract_version":2,"operation":"PublishCharacterSkillsV2","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"0","projection_revision":7,"source_observed_at":"1790000000","skills":null}
```

Invalid: unknown/private account member even with a valid UUID; `400`, never store it:

```json
{"contract_version":2,"operation":"PublishCharacterAchievementsV2","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"8","source_observed_at":"1790000000","achievements":[],"account_id":"01934f10-7c01-7001-805b-3b1122334401"}
```

Invalid: completeness later than observation; `400`, preserve previous watermark:

```json
{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"character_skills","projection_epoch":"1","complete_through":"1790000004","observed_at":"1790000003"}
```

Invalid in sequence: after the valid occupied equipment revision 9 above, this equal key with changed content is `409`, not a correction or retry:

```json
{"contract_version":2,"operation":"PublishCharacterEquipmentV2","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"9","source_observed_at":"1790000000","equipment":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","character_revision":"19","slots":[]}]}
```

## Acceptance checklists

Game owner/coordinator, per family:

- [ ] Accept version/routes/status, schemas, capacity/byte bounds and exact source mapping; pin decimal-string time fixtures and resolve candidate source decisions.
- [ ] Confirm stable GuildId/CharacterId, death receipt identity and historical killer-name capture; settle exclusions and name-handoff behavior.
- [ ] Qualify achievement attribution and transfer invalidation, and owner-only equipment/outfit/achievement policy; otherwise keep those families unavailable. Online accounting qualification belongs to G-8.
- [ ] Demonstrate same-transaction revision/outbox touches for every source/privacy/lifecycle change, committed-cut reads and acknowledgement-safe clearing.
- [ ] Prove restore fence, resync completeness/capacity within existing deadline, watermark stall on poison/undelivered changes and no runtime impact from delivery failure.
- [ ] Publish shared canonical fixtures including maximum size/depth, one-over-bound, missing/extra/duplicate/null fields, bad IDs, milliseconds, unknown version/enum, same-second deaths and empty/tombstone distinctions.
- [ ] Qualify retry/refusal/timeout behavior, dedicated real mTLS peer isolation and default-off flags on exact Game/Platform revisions; record environment and family acceptance explicitly.

Platform owner, per family:

- [ ] Parse exact schemas without coercion, enforce byte/depth/collection bounds before storage and reject source/identity mismatch.
- [ ] Prove duplicate, conflict, superseded, restore-epoch, missing-baseline and resync paths; confirm equal retry cannot refresh old evidence.
- [ ] Prove freshness on every cache/index/notification path; stale/unknown never appears as zero/offline/empty success.
- [ ] Prove current owner/foreign-owner denial, ownership-transfer invalidation, independent privacy floors, restrictive cutoff, invalidation failure and rollback after deny.
- [ ] Test hidden guild-member aggregate/search leakage, reused killer/owner names, profile suppression, secret-unearned achievements and denial of all public achievement/equipment/outfit views.
- [ ] Prove T70 replay/baseline alert suppression and T66/T68 history coverage limitations; do not close unrelated G-8 or module criteria.
- [ ] Accept retention/purge/backup fences and payload-free diagnostics; record producer/consumer exact SHAs, per-family default-off rollout and reversible serving switch.

Neither checklist is satisfied by this draft or by unit tests alone; real composed peer/producer/consumer evidence and both owners' acceptance precede activation.

## Open questions for the game owner

### Answered owner decisions (closed)

- **Q1 — Answered:** V2 replaces the v1 candidates. Deaths, guild, house and skills are accepted with notes; the accepted v1 surfaces and closed watermark enum remain unchanged.
- **Q2 — Answered (visibility):** Achievements are owner-only, with no public highlighting. Attribution and ownership-transition evidence must still be qualified before export; this decision does not prove a producer exists.
- **Q3 — Answered:** Remove the G-4 activity family. Online time comes solely from G-8 online intervals: owner-only intervals, public world totals.
- **Q4 — Answered:** Use `death_occurrence_id` as the stable death key; retain the latest 20 deaths within 30 days. This does not supply creature/environment causes or a complete 10-week PvP history.
- **Q5 — Answered:** Use stable IDs and order guild members by `(rank_level, character_id)` (rank, ID); house owner names remain display-only.
- **Q6 — Answered:** Yes to base skill levels plus XP without tries/modifiers, ten primary equipment slots with catalogue references, and `DisplayedLook`. Equipment and outfit are owner-only.

### Remaining open question

- **Q7 — Operations/privacy/retention:** Accept the proposed ceilings and owner-only restrictions? Who qualifies source triggers, extended stopped-node resync capacity, privacy revocation fences, production PKI/freshness and exact-revision interoperability before activation? Source/attribution and ownership-transition evidence for achievement export remains part of this qualification.
