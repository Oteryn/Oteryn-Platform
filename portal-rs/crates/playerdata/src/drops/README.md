# Drops, personal records and character chronicle

T75 derives private projections from the `Journal` adapter implemented by T72/T73.
No T73 branch existed when this task started. The base also had no playerdata crate.
The adapter is deliberately the only input: this module does not decode a second,
competing version of the proposed G-9 wire protocol or accept request-owned facts.

`Repository::ingest` loads the source session within the same transaction as its
immutable projection write. `Repository::read` checks the authenticated actor,
current cut and source session again before returning any history. `page` uses
that same read path and renders escaped HTML with Fluent Polish/English messages.
The host must supply its session-derived actor and private no-store headers.
No route is mounted without the upstream receiver and session adapter. Both
capabilities default to false. Enabling reads does not enable ingest.

The upstream adapter must hold appropriate authority locks through transaction
completion; prove durable logout, contiguous applied revisions, every segment and
rare-drop count, catalogue rarity rules, ownership, erasure and consent; and suppress
expired or quarantined sessions. Source lookup failures abort rather than expose
cached projections. Session IDs never refer to runtime sessions or character leases.

## Conservative interpretations and integration requirements

- The owner's G-9 revision-2 direction overrides older proposal wording: sessions
  span game entry to durable logout; area exits and 15-minute inactivity end segments.
- Use the transfer receipt's character_revision, projection_revision,
  projection_epoch and recovery_generation cut. Any changed or unproven cut hides
  old projections. No seller-history exception is implemented until privacy and
  transfer policy are accepted consistently across G-9, G-10 and G-11.
- Rates use complete active duration. A zero denominator yields unavailable.
  A missing price is unavailable. Values with distinct price bases never compete.
  Negative profit remains negative. Rare drops never augment measured loot totals.
- Records describe available, proven history, not an asserted all-time baseline.
  T73 must retain sufficient aggregates before pruning source details; until then
  pruned source evidence hides projections rather than manufacturing records.
- G-9 kill counts and G-11 current bosstiary counts/last occurrence cannot prove a
  first boss kill across missing history. The first-boss field stays unavailable
  pending an authoritative first-occurrence receipt and complete history boundary.
- G-10 inventory and trade snapshots cannot establish rare drops. Only the pinned
  G-9 catalogue rarity rule is used; optional T74 value highlighting is not inferred.
- Account deletion, retention and anti-resurrection remain upstream authority.
  The adapter must deny erased data and the coordinator must connect projection
  purge/export hooks before activation. No user or game mutation is introduced.

Tests run against isolated schemas using the exact task migration. They read
PORTAL_TEST_DATABASE_URL and print a skip notice if it is absent. Two tests exercise
PostgreSQL: replay/conflict/owner/cut fencing and incomplete/invalid input rollback.
No SQL query macros or offline query metadata are needed.
