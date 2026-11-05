# Place cards

`/{locale}/wiki/places` supports Polish and English, purpose (`hunt`,
`bestiary`, `boss`), party size (1, 2, 4), and descending experience/profit
sorting. Missing metrics stay absent. Responses are bounded to 100 cards.

`PORTAL_WIKI_PLACES_ENABLED` defaults to false. The configured adapter also
returns 503 while public evidence is unavailable, even when the flag is true.
`PublicPlaces` is an internal trusted adapter boundary, not a browser input type.

## Activation blockers

- T81 currently exposes `PlacesStore::daily(actor)` to a verified authenticated
  account, rechecking source and privacy evidence. It has no qualified public
  card feed, party-size cohorts or kill-rate metric. Do not query its private
  source tables or repurpose an arbitrary account as a public actor.
- Resolve public world place visibility and approved editorial bindings through
  the world and wiki public APIs. Ambiguous bindings must deny publication.
- `docs/contracts/OTERYN_V2_GAME_JOURNAL_SESSIONS_CONTRACT_PROPOSAL.md`
  sections 8–9 keep `playerdata.hunts.share` false under
  v1. Reports must wait for an accepted share contract including owner consent,
  revocation, expiry, moderation and account-erasure semantics. A moderator's
  approval alone cannot authorize publishing a private session.
- Bestiary/boss points and completion estimates require reviewed world facts
  and qualified kill-rate evidence. Never infer party size, zero rates,
  encounter availability, or whether a boss spawned today.

No Game wire format or Laravel endpoint is introduced by this boundary. The
existing Laravel routes have no corresponding place-card surface. No fixture
from the Game repository is needed for this adapter-only implementation.

This is a blocked integration scaffold, not a completed T87 release. Tests
exercise rendering, escaping, filters, locale refusal and the default-off guard.
