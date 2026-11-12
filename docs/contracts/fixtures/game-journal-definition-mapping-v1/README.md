# Game journal definition mapping, v1 (CANDIDATE)

Sample for `docs/contracts/OTERYN_V2_GAME_JOURNAL_DEFINITION_MAPPING_CONTRACT.md`.
It is not a catalogue snapshot and not a wire golden: it lists, per G-10/G-11
family, a few references that must resolve and a few that must not, with the
reason. No value in `mapping.json` is invented; every key, revision, threshold,
category and kind was read from Game content at the pin below. Reasons such as
`ALIAS_KEY` and `SYNTHETIC_GOLDEN` are labels of this sample, not wire codes.

## Source

Read-only, raw files at the full commit; nothing in Oteryn-Game was changed.

- Repository: `Oteryn/Oteryn-Game`
- Revision: `29eb08d392182a65da0304d8d29019588e1e7186` (`main` on 2026-11-12)

| Path at that revision | SHA-256 | Used for |
|---|---|---|
| `content/items/definitions/items-00000-00499.json` | `c29c94efdfb37a753a3b1a1ce0cdd28c74e0a1c0d5bff461a7e2f86870010d94` | Item keys `i10326`, `i10196`, `i100` (not materializable), stack and container facts |
| `content/items/aliases.json` | `128bc354816199702c26f83120e6fb7846fb2608fb5ee1c16085f63268e592f2` | Alias `oteryn:item.ammunition.arrow` to `oteryn:item.tibia.i3447` |
| `content/houses/houses-00000-00499.json` | `bd6193f9bca458a98ae87fb6dae836936d8665e0d6f73a58a45688fe5bbdcbb8` | House keys and kinds |
| `apps/game-server/migrations/0025_house_item_custody.sql` | not hashed | House custody key CHECK |
| `content/creatures/definitions/creatures-00000-00499.json` | `c97dea235b2c2747caee660f41295b36901d209d5c1b22f8300ccd8eed431ea7` | Bestiary race and Bosstiary boss keys, revisions, thresholds, category |
| `content/quests/missions/quest-state.json` | `5d826cb9365db753624d8e0b1cd33fc430848bf4c4db33cfa2ca9b4458620f8d` | Quest key and track keys |
| `content/achievements/achievements-00000-00499.json` | `d097a1833981e27ec326f274651c39b5f6dc6f06ea79afc6cb6123e953a81341` | Achievement keys, revisions, secret flag |
| `rulesets/character/blessings/catalogue.json` | `711e66f10180af340cc79083a4c70d4cc7642a969aa4d4f757726070956969b6` | Blessing storage and service keys |
| `content/creatures/bestiary/index.json` | `92bb33d31e7663e5794e9a3bed3a27f05cf0d3e16159e299a529fe74b6aa69b4` | `READY_UNPOPULATED`, so not the Bestiary source |
| `content/creatures/bosstiary/index.json` | `c9b25b048a0235ed50a3306d203e67c4efef3bc9414354140c488ef41c79c862` | `READY_UNPOPULATED`, so not the Bosstiary source |

The G-10 golden reference `ITEM` / `oteryn:content.item.backpack` / `rev-1`
appears only as a `SYNTHETIC_GOLDEN` negative case; it is synthetic in the G-10
contract and does not exist at this revision.

## Updating

Changing the Game revision is a contract change: re-read every path above at
the new full SHA, update the keys, hashes and this table, and revise the
contract's §2 in the same change.
