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

Production remains unavailable. The Store wire and Store purchase delivery
contracts are PROPOSED, and the Store owner amendment keeps COSMETIC disabled
pending the cosmetic authority read. G-9 revision 3 and G-11 revision 4 accept
the journal wires and semantics. The accepted revisions do not define a
season-pass metric mapping. The T89 plan reference to §5.8 now points to skull observations,
and the journal authority still refuses quest and achievement evidence pending
catalog or custody mapping. The configured event and reward paths fail closed.

`GET /{pl|en}/pass` renders account-private progress, challenges, free/paid
tracks and cosmetic grant keys. CSRF-protected POST routes at
`/{pl|en}/pass/{season}/buy` and `/claim` accept only operation/reward IDs; prices,
content, account and points are always server-owned. The generic router accepts
trusted adapters for integration. The configured router mounts only with
`OTERYN_SEASON_PASS_ENABLED=true`. With `OTERYN_ENV=testing`, it uses the shared
account eligibility lock, and `OTERYN_SEASON_PASS_WALLET_ENABLED=true` enables
the existing Wallet's audited pass debit. Test coins can come from the existing
sandbox checkout. Season definitions for browser qualification are disposable
fixtures. Outside testing, the mounted routes retain their unavailable adapters.
All responses are private/no-store.

PostgreSQL tests use isolated schemas, the full migration chain and synthetic
facts. They read `PORTAL_TEST_DATABASE_URL` and print a skip notice when unset.
They cover verified sources, bad input, time boundaries, unknown authority,
account refusal, purchase and claim replay/concurrency, insufficient funds,
failed wire rollback, paid eligibility, feature flags and the real wallet API.
