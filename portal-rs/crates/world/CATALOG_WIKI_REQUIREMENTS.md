# Wiki requirements for the Game native catalogue adapter

Evidence: CATALOG_EXPORT_V1 at Game
`fd4be4e214ef33ad65d3ce465dab7160377e62f5` (lane branch, re-pin on merge). The committed input declares every
capability unsupported/unknown and contains no entities. Raw native content is
not an integration fallback. The consumer verifies the artifact, builds inactive
capability-scoped entity/relation/tombstone records, and refuses complete reads
for unsupported, partial or unknown capabilities.

## Exact capability scope

The current wiki has seven entity families covered by the v1 vocabulary:

| Export capability | Current wiki route | Minimum presentation required by the current renderer | Recognized optional facts/filters |
| --- | --- | --- | --- |
| `item` | `/wiki/items` | Stable native key and authoritative name | `category`; `attack`, `weight`, `grade` |
| `creature` | `/wiki/creatures` | Stable native key and authoritative name | `category`; `health`, `experience`, `speed`, `armor`, `defense`, with explicit health visibility semantics |
| `npc` | `/wiki/npcs` | Stable native key and authoritative name | `category` |
| `spell` | `/wiki/spells` | Stable native key and authoritative name | `category`; `cooldown_ms` |
| `quest` | `/wiki/quests` | Stable native key, authoritative name and classification | `category`; `classification` filter |
| `achievement` | `/wiki/achievements` | Stable native key, authoritative name and release/secret policy | `category`; `points` |
| `creature_loot` | `/wiki/loot` | Stable native record identity and authoritative display label | Current generic detail renderer does not yet render drop entries or relation graphs |

These names are **Platform renderer requirements**, not a newly invented Game
payload schema. Game must define accepted capability-specific payload revisions,
field meanings, units, visibility rules and binding of native types to capabilities;
Platform must then implement matching validation/presentation. The v1 generic
`data` object alone does not define any of those authoritative fields. Native
`type`, `content_key` and `capability_id` already exist in the v1 carrier.

The renderer currently accepts authoritative `names.pl`/`names.en`, `display_name`
or `name`; search matches the resulting name and native key. This describes the
current renderer, not permission for the consumer to infer or synthesize a name.
Game may choose another payload structure through an accepted payload contract.
No display names are reconstructed from keys in the native cache.

Source references:

- `crates/wiki/src/entities.rs`: `KINDS`, `world_kind`, `name`, `row`, `collect`, `detail`.
- [Contract vocabulary](tests/fixtures/catalog-fd4be4e2/CONTRACT.md#L54).
- [Payload evolution gate](tests/fixtures/catalog-fd4be4e2/CONTRACT.md#L222).

## Deliberately separate gaps

- **`npc_shop` is not required by the current wiki renderer.** It has no dedicated
  route or shop-entry presentation today. It is a separate adapter/UI increment
  for future NPC shop inventory, prices and conditions; it must not block the
  basic supported NPC list/name view. The generic cache can retain its supported
  carriers without claiming shop semantics.
- **Areas are not a v1 capability.** `/wiki/areas` currently exists, but Game must
  define an additive/versioned area capability and its payload contract. Do not
  repurpose another capability or read raw area JSON. Unknown capabilities are
  rejected until the contract and consumer are upgraded together.
- Atlas needs a **Game-side atlas export**. V1 carries no map/tiles/placements
  capability. This consumer does not render B3/terrain substitutes.
- Today additionally needs an accepted G-12 producer. Catalogue names do not
  supply a daily selection or authorize proposed live publication.
- G-2 is resolved: the shared public view omits hidden identities, hidden NPC shop
  relations and secret achievements. Exported records are public by construction.
  An explicit `visibility` member must be the string `public`; another string,
  null or a wrong type rejects the whole artifact. Absence is the normal wire form.
  Independent `world_visibility` approval still applies to every public record.

## Coverage and activation

Each declared capability has independent support and completeness. Supported
partial/unknown records may be cached as explicitly non-authoritative evidence;
only `supported + complete` passes a complete-capability read. Unsupported records
are invalid at verification. Unknown/incomplete inventory cannot prove absence,
removal, or an empty public catalogue.

The real lane still marks all native categories unsupported/unknown, as explicitly
recorded in [lane notes](tests/fixtures/catalog-fd4be4e2/CATALOG-CONTRACT-NOTES.md#L38).
Its negative input cannot publish. With the opt-in production publication switch,
complete categories project verified cache records through a separate wiki pointer;
home/search aggregate only complete categories and refuse when none exists.
Per-category routes refuse unsupported/partial/unknown inventory and categories
missing required presentation fields (names in both languages or a common name;
quest classification). An empty complete category is valid. Area stays 503
pending the scheduled additive category. Publication never grants an approval.

Test-only supported vectors exercise the **production** publisher and HTTP routes
for all seven current wiki categories, independent approvals/denials, idempotence,
empty complete replacement and configured-source mismatch. No test publication
adapter is compiled into serve. No native positive fixture is fabricated.
