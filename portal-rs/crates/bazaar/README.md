# Bazaar core

T57 provides a Rust API, pure integer auction rules and PostgreSQL storage. It performs no HTTP,
game or wallet-service I/O. The existing workspace `crates/*` member includes this crate.

`Bazaar::new` requires `Accounts`, `Reservations`, `Evidence` and runtime `Flags`. Both flags default
to false. There is no binary registration or enabled production adapter in this task. Account UUIDs
have no foreign key. `Accounts::state` must return current unambiguous state; unknown, inactive,
unverified or bazaar-blocked accounts deny trade. Listing publication and offer acceptance also
require MFA and sudo. Authentication and actor extraction remain the caller's responsibility.

Every mutation uses one SQL transaction. Existing listings and pending offers are locked with
`FOR UPDATE`; bid and settlement retries serialize on the listing. Reservation adapters receive that
transaction's connection and must never commit it, use another connection or make network calls.
All relevant wallet accounts are locked in UUID order before holds change. Adapters must reserve
from the one coin balance, enforce available balance including other holds, and implement immutable,
replay-safe ledger effects. Gold requires the additional flag and an adapter for durable verified
game escrow; no game call is allowed inside the transaction. Account-state adapters must use their
own account-state locks consistently before wallet locks. The core reads participating accounts in
UUID order. Other domain callers must follow the same lock order.

Auction increments round 1% of the current clearing price up to a smallest unit, with floors of
1 coin unit or 1,000 gold. Earlier maxima win ties. A leading bidder can raise their maximum without
raising the clearing price. Every submitted maximum must be fully funded, including immediately
outbid bids. Only the leader's full maximum remains reserved. Bids received less than five minutes
before the locked end time add five minutes to that end time. Replays return the original outcome
and do not extend again. Request keys are scoped to a listing; changed input under the same key is
refused. Internal read models contain private maxima and must not be serialized to public clients.

Advertisements run fourteen days. Fixed-price ads can require the exact advertised price or allow
negotiation. Pending buy offers reserve funds, and rejected/expired/competing offers release them.
Want-to-buy responses reserve the listing owner's funds at acceptance and reverse settlement roles.
Evidence must verify the selected seller's deposit before acceptance. Responding accounts are limited
to twenty pending offers across listings and one pending offer per listing.

States: draft → active → matched (ads) → awaiting delivery → completed. Auctions enter awaiting
delivery directly after their end. Empty active listings can expire; the owner can cancel drafts or
active listings only before any bid or offer. SQL enforces the state graph and appends audit records
within the transaction. Bids and audit history reject update/delete.

Settlement is unique per listing and has an idempotency key. Commission is 10% for coins and 2.5%
for gold, rounded down; the seller receives the exact remainder. The adapter consumes only the
clearing price, releases excess reservation, and holds seller proceeds pending verified delivery.
Completion is replay-safe and requires durable delivery evidence. Failed adapters or SQL statements
roll back the listing, ledger, request log and audit together.

## Validation

Run the workspace acceptance commands with Rust 1.97.0. Queries use bound runtime SQL, not query
macros, so no offline metadata is needed; builds also work with `SQLX_OFFLINE=true`.

`tests/postgres.rs` reads `PORTAL_TEST_DATABASE_URL`, prints a skip notice when unset, and otherwise
creates a UUID-named isolated schema per test on PostgreSQL. Each schema uses the real migration.
The in-memory ledger in `tests/support` computes balances, holds, pending proceeds and commission.
Its SQL adapter snapshots that state through the caller's transaction so rollback and concurrency
can be tested without the parallel wallet implementation. It is test support, not a production
wallet. Successful tests drop their schemas; failed tests retain schemas for diagnosis.

Integration work left for T58: verified escrow/snapshot and gold evidence adapters, game commands,
outbox delivery, receipts and reconciliation. Player/staff pages, moderation, notifications and
statistics remain their separate planned tasks. Optional buy-now auctions, ad renewal and tradable
item/character cooldown checks must be composed with the later game and presentation tasks; this
core currently supports auction bids and advertisement offers only. No live game or money feature
is enabled by this crate.
