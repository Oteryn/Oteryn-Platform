# Collection PvP

T91 owns consent, battle persistence, revision checks, standings, round-robin
registration and fixed-content purchases. No game-server command is issued.

Both `OTERYN_COLLECTION_PVP_ENABLED` and `OTERYN_COLLECTION_SETS_ENABLED` default
to false; only the exact value `true` enables a surface. Browser mutations use
the existing session and synchronizer-token CSRF middleware. Account identity
comes from the session, never a submitted account field.

## Integration seam

The prerequisite package is merged. The T90 branch was absent during development;
there is no collection or PvP route implementation in the Laravel reference and
no matching collection contract in `docs/contracts`. `Collection` therefore owns
the narrow adapter boundary: legal owned deck snapshots, initial battle state,
validated turn resolution, and non-transferable exact card grants. The adapter
must pin world/content and rule versions in its snapshots and reject unknown,
unavailable, malformed, or out-of-turn states. It must perform database writes
through the supplied connection, without independent commits or external effects.

The configured production router uses `Unavailable`: gameplay is denied and
controls are hidden until an authoritative collection adapter is installed.
The public `router` constructor accepts that adapter and a `SetPayment` adapter.
`WalletPayment` bridges the public wallet purchase API, preserving its flags,
identity checks and audit policy. The default payment adapter also denies.

No random paid reward exists. Set contents are an explicit map of card keys to
positive integral counts, bounded to 100 keys and 100 copies per key. Contents,
version and price are immutable in PostgreSQL. Each purchase binds the previewed
version and price to a UUIDv7 idempotency key. Debit, grants and receipt share one
transaction. Cards cannot be transferred or redeemed by this module.

A tournament has 2–16 participants, no entry fee, stakes or transferable prize.
Joining consents to all fixtures with the pinned deck. Only its organizer starts
it. Every pair plays once. Standings award 3 points per win, 1 per draw, 0 per loss;
UUID order only orders equal-score rows and does not break the competitive tie.
A battle can score once. Completion is serialized per tournament.

## Validation

Tests use isolated schemas in `PORTAL_TEST_DATABASE_URL`; an unset variable prints
a skip notice. Four PostgreSQL tests cover consent/replay, tournaments, fixed-set
payment/grant atomicity, and real session/CSRF HTTP flow. Two unit tests cover
flags and invalid fixed contents. Fixtures are synthetic, with no copied game data.
