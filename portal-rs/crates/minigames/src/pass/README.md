# Season pass

T89 implements immutable 6–8 week seasons, independently verified journal and
portal challenges, one-time XP awards, free/paid cosmetic reward tracks,
transactional coin purchase through the public wallet API, and idempotent claims.
All feature flags default to false. Published seasons cannot overlap.

`Authority` must hold the current account/custody/erasure/restore fence for the
transaction and verify source facts. A browser cannot submit progress. Source
facts must be closed, owned, catalog-bound and immutable. Duplicate source IDs
with different payloads conflict. Challenge completion awards XP once.

`Wallet::new` classifies debits through the wallet's existing caller-owned
transaction API. Price and content come only from the immutable season. The
wallet must supply its own account and audit adapters and enabled flag.

`Rewards` is a narrow unresolved store issuance seam, not a second delivery
system. It must issue a nontransferable cosmetic entitlement and its DELIVER
operation atomically, binding operation/account/reward/content revision. Returned
IDs indicate enqueueing only; Game receipts and reconciliation remain store-owned.
No reward is granted locally. Unknown or failed enqueueing rolls the claim back.

Production remains unavailable: T29 exposes paid purchase, but no public free
cosmetic entitlement issuance API. The store owner amendment still defers Q4
(cosmetic authority read), and private journal authority is unavailable. The
read-only Game store codec at commit
857f15f376907c592bb28d854fccff18a15cdf87 recognizes COSMETIC/DELIVER; this alone
does not establish entitlement authority or journal production readiness.
No Game fixtures were copied. Laravel has no season-pass route/behavior to port.

`GET /{pl|en}/pass` renders account-private progress, challenges, free/paid
tracks and cosmetic grant keys. CSRF-protected POST routes at
`/{pl|en}/pass/{season}/buy` and `/claim` accept only operation/reward IDs; prices,
content, account and points are always server-owned. The generic router accepts
trusted adapters for integration. Production configuration enables mounting only
with `OTERYN_SEASON_PASS_ENABLED=true` and uses `Unavailable` for all adapters,
with paid/reward flags false. It therefore returns a localized 503 and never
charges or issues rewards. All responses are private/no-store.

PostgreSQL tests use isolated schemas, the full migration chain and synthetic
facts. They read `PORTAL_TEST_DATABASE_URL` and print a skip notice when unset.
They cover verified sources, bad input, time boundaries, unknown authority,
account refusal, purchase and claim replay/concurrency, insufficient funds,
failed wire rollback, paid eligibility, feature flags and the real wallet API.
