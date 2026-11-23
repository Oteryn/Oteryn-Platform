# GAP-58 Atlas catalog qualification

## Player flow

Open **Quest delivery and item sources** from the World menu. Choose an item on
a quest step to see its visible NPC offers and creature drops. A spawn link
opens Atlas at the exact coordinates in the same imported world version.
The page uses the existing Atlas delivery and world read flags. Both default
to off. The map viewer separately requires `OTERYN_ATLAS_ENABLED=true` and the
approved `OTERYN_ATLAS_DIRECTORY`. All these reads are local to Platform.

The delivery view uses the active WorldStore import and the Platform visibility
list. Quest item keys are exact, including G-13 keys such as `oteryn:item.*`.
The reader accepts the existing stage `item` and `target_items` shapes.
For an approved reward projection, `reward_items` contains entries with an
exact `item` key and an optional `count`. Rewards appear with their own label.
An unresolved marker, missing target or denied item yields an unresolved step.
A typed key is never recovered from a localized name or objective. A version
selected by an Atlas link must still be active.

## Reviewing an imported world

Each item, quest, NPC, currency, offer, loot record, spawn and sprite mapping
requires its own visibility decision. Producer visibility vetoes remain in
force. Public offers require approved endpoints. Existing
world import and staff review supply the version and approval. No migration is
needed.

- `NPC.ShopOffer` is the Platform public projection of an approved, currently
  reachable unconditional offer. Its body has `direction` (`npc_sell_offer`
  means the NPC sells to the player, `npc_buy_offer` means it buys),
  `npc`, `item`, `currency`, `unit_price` (positive u32), and
  `item_subtype` (i32). Endpoint values are exact world keys. Use the
  producer relation identity as the record key. Check release applicability,
  reachability, currency ID and subtype before allowing the record. Keep
  conditional, registered-only, disabled, unknown and player-specific offers
  out of this projection. Raw v1.3 proposals and generic native cache relations
  are outside this mapping.
- `Item.Sprite` uses the exact Item key as its record key and
  `appearance_id` and `manifest_sha256` in its body. Review the item's
  appearance ID against the pinned release. Publish only licensed, reviewed
  public T42 output under `<static_dir>/world-sprites/<source_commit>/<manifest_sha256>/`.
  The page checks the commit, manifest digest, canonical thumbnail request
  and thumbnail digest. Missing or inconsistent evidence appears unavailable.
- The existing T40 minimap pipeline writes
  `<atlas_directory>/<source_commit>/<floor>/manifest.json` and its tiles.
  Keep the source commit aligned with the active WorldStore import. Atlas
  checks the tile digest when serving it. The viewer reports unavailable when
  the requested world or its map evidence cannot be resolved.

## Operator status

Use **World catalog** from the staff home. Access requires the existing World
Admin switch, staff authentication with MFA, `game_catalog.access` and
`game_catalog.snapshots.view`. The panel displays the verified candidate
matching the configured snapshot ID, artifact SHA-256, source revision, epoch,
ruleset and profile. It shows the current published wiki capabilities and
cached shop and quest record counts. Cache counts describe transport evidence
only. In particular, a zero count cannot prove a complete empty capability.
An absent or mismatched source shows unavailable. Existing world review retains
import errors, comparisons and visibility management.

## Game evidence still required

- The native CATALOG_EXPORT_V1 path accepts schema `1.0.0`. Game needs to
  supply an accepted producer revision and capability payloads for exact NPC
  offers, item references, drops and placements. A pinned snapshot must pass
  import and publication qualification. Unrecognized versions remain refused.
- The separate [Legacy Engine Compatibility v1.3 proposal](../contracts/GAME_CATALOG_1_3_NPC_SHOP_PROPOSAL.md)
  needs a byte-identical canonical consumer and producer schema, a deterministic
  final registry export, storage-sentinel proof, currency/subtype and nesting
  evidence, and the prescribed inactive import, staging activation and rollback
  qualification. Public projection requires its own visibility review.
- Game needs to emit G-13 stable keys for uniquely resolved objectives and
  rewards, and explicit resolution state otherwise. Native reward mapping
  awaits an accepted producer payload. Game also needs to supply
  release-aligned minimap chunks, exact appearance mappings, and evidence
  that the proposed visible spawns and shops are reachable.
