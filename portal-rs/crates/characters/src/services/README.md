# Character services

T46 implements G-3 v2 with the owner's amendments: the game chooses the deletion
deadline, which must be strictly later than scheduling. Cancellation is strictly
before that deadline; Game rechecks session eligibility in its deciding transaction.
Released-name holds are Game's 30-day policy. Sex change and world transfer have no
submission path and appear unavailable.

The browser route is `/{lang}/account/characters/{character}/services` (GET/POST,
`pl` or `en`). Web's session middleware enforces synchronizer CSRF and origin checks
before form extraction. POST also checks sudo against that same live session.
Current account permission, independent LCFA/service freshness, the shared epoch,
restore readiness, lifecycle and pending conflicts are checked again in the request
transaction. Old owners, absent facts and inconsistent projections are denied.
Owner responses are private/no-store. The page lists the latest 20 operation states.

`web::internal_character_services_router` composes the five proposed v2 routes, v1 commands and restore notices on
the dedicated verified TLS listener. It takes the existing identity map: command
peers use `OTERYN_GAME_CHARACTER_COMMANDS`, service publishers use the separate
`OTERYN_GAME_CHARACTER_SERVICES` group. Never mount this as an unverified public
endpoint. Composition probes v2 exchange readiness before allowing restore to
reopen work. The existing v1/restore implementation remains unchanged; v1
pending/read cannot reveal v2 operations. No new worker or outbound Game call is
needed: Game pulls the shared durable command outbox.

## Activation

Every flag defaults to false. `Settings` can be supplied explicitly by listener
composition. Browser configuration reads:

- `OTERYN_CHARACTER_SERVICES_ENABLED`
- `OTERYN_CHARACTER_SERVICES_SCHEDULE_ENABLED`
- `OTERYN_CHARACTER_SERVICES_CANCEL_ENABLED`
- `OTERYN_CHARACTER_SERVICES_RENAME_ENABLED`
- `OTERYN_CHARACTER_SERVICES_PROJECTION_ENABLED`
- `OTERYN_CHARACTER_SERVICES_EXCHANGE_ENABLED`
- `OTERYN_WALLET_ENABLED`
- `OTERYN_CHARACTER_SERVICES_COMMAND_AUTHORITY`
- `OTERYN_CHARACTER_SERVICES_PROJECTION_AUTHORITY`
- `OTERYN_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS`
- `OTERYN_CHARACTER_RENAME_PRODUCT_ID`
- `OTERYN_CHARACTER_RENAME_PRODUCT_VERSION`
- `OTERYN_CHARACTER_RENAME_PRICE` (positive integer coin units)

No price is invented. The browser quote must match the transaction's configured
price; a stale quote refuses. Browser freshness is qualified only in `OTERYN_ENV`
`testing`/`preprod`, with a 30-second bound and explicitly configured clock
uncertainty. Production has no implicit clock/freshness allowance. Producer support,
private listener identities, product approval, clock/retention policy and exact
revision qualification remain activation prerequisites, never inferred from a flag.

Scheduling also requires cancellation enabled. Turning off scheduling leaves
cancellation available. Stop new submissions with per-action/master flags while
keeping exchange/projection and Wallet available to drain and reconcile work.
An unavailable account or Wallet during settlement holds the receipt/reservation
for retry, never fabricates financial success.

## Atomicity and restore

One transaction holds restore, account/character and issuer serialization, reserves
coins with Wallet's public transaction API, and inserts immutable intent and price
metadata. The 40-command backlog includes both wire versions. Exact request replay
returns its original operation; a changed request conflicts. At most ten new
orders per account per minute are accepted. Different uncertain operations for one
character cannot reserve concurrently. Expiry and timeout never release funds.

Receipt binding, Wallet capture/release, command terminality and the existing
restore history trigger share a transaction. Settlement has an immutable UUIDv7
idempotency key. Invalid/conflicting receipts leave funds held. Receipt facts never
replace a service snapshot or bump LCFA's v1 revision. A higher shared epoch
invalidates older facts on both read sides; equal-key contradictory snapshots are
quarantined. Post-restore submissions require independently refreshed feeds.

A reopened paid operation keeps its original ledger history. A surviving commit
cannot charge again. If authoritative reconciliation replaces its lost commit with
a refusal, the original order grants one free replacement rename. The replacement
holds that credit until a receipt; refusal releases it for retry. A contradictory
later restore while that credit is in use denies for reconciliation.

PostgreSQL tests use isolated schemas and `PORTAL_TEST_DATABASE_URL`; when unset,
they print an explicit skip notice. Coverage includes insufficient funds,
concurrent reservations, double-settle, transaction rollback, receipt replay,
refusal, restore/free replacement, projection/epoch conflicts, owner denial,
cancellation boundary, browser CSRF/recent authentication and both wire versions.
