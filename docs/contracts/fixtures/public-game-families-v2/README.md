# Public game projection families v2 — frozen fixtures

Source: [`docs/contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md`](../../OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md),
ACCEPTED revision 3 (2026-11-13, PR #1854), section **Golden payloads**.

Each `.json` file is one fenced line of that section, copied byte for byte. The
contract says the fence newline is not part of the transmitted bytes, so no file
has a trailing newline. Do not reformat, re-encode or rewrite these files to match
code; a change needs a new contract revision.

| File | Contract example (in order of the section) | Expected result |
|---|---|---|
| `valid-character-deaths.json` | Valid death with recorded final-blow killer | `200` accepted |
| `valid-guild.json` | Valid guild, including stable member identity | `200` accepted |
| `valid-house.json` | Valid vacant shop with zero beds | `200` accepted |
| `valid-character-skills.json` | Valid committed skills/progression | `200` accepted |
| `valid-character-achievements.json` | Valid owner-only earned-list view | `200` accepted |
| `valid-character-equipment.json` | Valid occupied primary equipment slot | `200` accepted |
| `valid-character-outfit.json` | Valid displayed outfit, no displayed mount | `200` accepted |
| `valid-character-skills-watermark.json` | Valid skills watermark | `200` accepted |
| `valid-character-skills-suppression.json` | Valid suppression of the earlier skills snapshot | `200` accepted (after the skills example) |
| `invalid-character-skills-zero-epoch-wrong-type-null-body.json` | Invalid: zero epoch, wrong revision type, null body | `400`, no state change |
| `invalid-character-achievements-account-member.json` | Invalid: unknown/private account member | `400`, never stored |
| `invalid-character-skills-watermark-complete-after-observed.json` | Invalid: completeness later than observation | `400`, previous watermark kept |
| `conflict-character-equipment-revision-9-changed-content.json` | Invalid in sequence: equal key, changed content | `409` after `valid-character-equipment.json` |

## Cases the contract gives no golden payload for

The contract section has no example for the following; none was invented here.
Tests that need them build their inputs in code and say so.

- An oversized or over-count snapshot of any family (for example achievements
  with more than 1024 earned rows, or a body over the route byte bound).
- Watermarks for `character_achievements`, `character_equipment`,
  `character_outfit`, `character_deaths`, `guild` or `house`.
- Suppression (empty body) of achievements, equipment, outfit, deaths, guild or house.
- Status-dependent house records (`AUCTION`, `OWNED`, `MOVE_OUT_PENDING`, `UNAVAILABLE`).
- An outfit with a displayed mount, equipment with more than one slot, or a guild
  at its rank/member bounds.
- Maximum size/depth, one-over-bound, missing/extra/duplicate members, bad IDs,
  millisecond times, unknown version or enum and same-second deaths; the
  contract's Game acceptance checklist lists these as fixtures still to publish.
- The `200` response bodies; the contract states them in prose
  (*Failure, refusal and timeout semantics*), not as fixture lines.

## Note

`valid-house.json` uses the house key `oteryn:content.house.example-shop`, which
the contract's `key` type allows. The existing Platform houses consumer applies a
narrower task rule (`[a-z0-9_]` after `oteryn:content.house.`) and would refuse
this fixture; that consumer is outside these fixtures' scope.
