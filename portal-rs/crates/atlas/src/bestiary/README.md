# Bestiary map

The default-off `OTERYN_ATLAS_BESTIARY_ENABLED` flag registers
`/{lang}/atlas/bestiary/{character}`. Browser sessions provide the account;
requests never select an account or bypass owner checks for staff. Responses
(including errors) are private and not stored. Invalid filters return 400,
anonymous requests 401, ownership refusal 403 and unknown authority 503.

`journal` calls the public T77 ProgressStore for both G-11 families. `Source`
is the narrow trusted T72/browser adapter: it must revalidate ownership, transfer,
erasure and recovery fences around its reads and supply one active visible world
version. Main does not yet supply that trusted authority. The configured source
therefore denies even when the flag is enabled. No public projection or private
SQL ownership guess substitutes for it. Production activation is pending that
adapter and approved G-11 producer coverage; the pure map and authenticated SSR
path are independently testable with injected sources.

SOURCE: docs/contracts/OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md,
revision 2, sections 5.1 and 5.2; Game read-only reference
857f15f376907c592bb28d854fccff18a15cdf87, monster authoring schema and
content/world/spawns. The fixed policies in `spawns-54000-54673.json` and
`spawns-boss-spawn1.json` use exact numerator/denominator ratios, validated against
`crates/world-bundle/src/spawn.rs`. Fixed boss points have no ordinary respawn
interval. `spawn_policy` decodes this closed shape without floating point values.
Only an explicitly approved fixed-at-reset policy can produce a boss marker. Held encounter bosses never appear. This
module neither invents policy probabilities nor exposes today's spawn result.
Laravel has catalog bestiary fields but no corresponding private map route;
there is no existing T86 response body to reproduce.

Charm awards remain Game's committed value. Boss stages sum each reached stage's
catalog points. Missing records, unknown revisions and cooldowns never become
zero progress or ready bosses. Recommendations use one finite positive rate
qualified for the character's level and ownership cut; no public individual
statistics or fabricated default rate is used. Locations are grouped by creature
with individually linked coordinates and text plus colored state indicators.
