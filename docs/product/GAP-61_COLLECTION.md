# GAP-61 Monster collection qualification

The Platform owns private card eligibility, deck management and turn-based PvE.
Open **World → Activities → Monster collection**, then choose a character. A
qualifying bestiary record displays the card as unlocked. Select one to three
different unlocked cards, save the deck and start a mirrored PvE battle.
The battle view shows the round, both rosters' health and the final result.
Decks and battles are Platform data. They grant no Game rewards or currency.

## Readiness and activation

Both `OTERYN_COLLECTION_ENABLED` and
`OTERYN_COLLECTION_JOURNAL_READ_ENABLED` default to off. The first mounts the
existing collection routes and navigation. The second admits the private journal
reader and owned-character selector. World card publication also requires
`OTERYN_WORLD_ENABLED=true`.

With Collection mounted, a verified operator with `admin.access` and
`game.world.admin` sees **Collection readiness** on the landing page. It reports
whether journal reads are enabled, whether the bestiary source is configured,
whether the T72 receiver accepts the mapping, and whether visible cards exist in
the active world publication. It does not expose another player's progress,
source credentials or source identifiers.

Card eligibility needs a fresh account character feed and custody proof, and a
complete G-11 bestiary watermark at the current epoch and recovery generation.
The reader considers the latest published values inside that custody cut.
A published card has to match its world version, race key, race definition
revision and final kill threshold. A matching incomplete kill count shows as
locked. Missing, stale or unmatched evidence shows the existing unavailable
state. The inventory and account bestiary update times are diagnostics only.

## Game-side qualification still pending

- Approve the revision-bound creature to bestiary mapping (race key,
  definition revision and threshold) and the presentation-appearance identifier
  for the pinned world release. A display-name match cannot establish identity.
- Qualify the producer's authenticated LCFA and G-11 bestiary delivery in the
  target runtime, including complete baselines, corrections, transfers,
  replay and restore. Configure the confirmed source authority through
  `PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY`.
- After approval, qualify the mapping against T72 admission and publish cards
  and verified sprites through the trusted Platform publisher
  (`Card::from_world` and `Collection::publish`). Until then, T72 keeps
  refusing bestiary payloads that need the pending mapping.
- Verify with two real accounts that an eligible owner can unlock, save and
  finish a PvE battle, a foreign account is refused, and turning off the read
  flag leaves the collection unavailable.

The Rust integration test exercises a published-cut reader fixture through
eligibility, deck save and a terminal battle. The browser spec covers the
owner and operator flows while the mapping is pending.
