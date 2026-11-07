# Wiki requirements for the Game native catalogue adapter

Evidence: CATALOG_EXPORT_V1 at Game
`d34807f5bd234ee8318e57ba454ba39be0df113d`. The committed input declares every
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
- [Contract vocabulary](tests/fixtures/catalog-d34807f5/CONTRACT.md#L54).
- [Payload evolution gate](tests/fixtures/catalog-d34807f5/CONTRACT.md#L222).

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
- G-2 is accepted as a `public|hidden` content-schema field. Its exported field
  path and omission semantics must come from the CATALOG schema/producer pin;
  the supplied unmerged branch currently returns 404. Publication stays closed
  until that mapping is available. Existing independent `world_visibility`
  approval remains an additional gate and must never release a hidden entry.

## Coverage and activation

Each declared capability has independent support and completeness. Supported
partial/unknown records may be cached as explicitly non-authoritative evidence;
only `supported + complete` passes a complete-capability read. Unsupported records
are invalid at verification. Unknown/incomplete inventory cannot prove absence,
removal, or an empty public catalogue.

The real unsupported fixture can be retained only as an inactive candidate; its
complete-capability checks fail and it never changes the active world pointer.
Even a supported carrier is not production activation authority: v1 still requires
accepted payload profiles, authenticated publication/current-epoch evidence, exact
profile selection and Platform authorization. No production activation toggle or
allow-all adapter is supplied by this task.

A **test-only** Item publication setup consumes the Game producer's existing
synthetic supported vector through the real cache read API, exercises independent
admin allow/deny, and proves HTTP 200 with `Training Sword` on wiki home, item
list/detail and search in both languages. The same setup refuses the real
unsupported fixture before touching public rows. Synthetic vectors and that
publication setup exist only under test paths/`cfg(test)` and do not authorize
native availability in the binary.
