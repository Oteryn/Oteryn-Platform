# Wallet core

`Ledger` is a Rust API with no player/staff UI. Construct it with a pool,
`WalletFlags` and explicit account-state, transfer-block and audit adapters.
`WalletFlags::default()` disables every API, including reads. The Game wire is
mounted on the shared mTLS-protected router and defaults off independently.

There is one coin class (owner decision 2026-11-12); all values are `i64` in the
smallest unit. Available and reserved balances are sums of immutable entry deltas,
read under `SELECT ... FOR UPDATE` on the account row. Every credit (top-up,
reward, staff grant, referral, settlement) adds to available, and every spend,
reserve and transfer takes from it. Available, reserved and their total cannot be
negative or exceed `i64::MAX`.
Transfers create and lock both wallet rows in ascending UUID order before reading
balances. Account UUIDs have no foreign key to the future identity schema.

Every mutation accepts a caller-retained UUIDv7 operation key. A global operation
record binds the key to its complete payload. Concurrent exact replay returns the
original operation ID without repeating entries or auditing. Changed payloads or
cross-operation key reuse return `IdempotencyConflict`. Failed operations roll back
the claim and all entries. Replays still require an active account; a transfer
replay reports a prior committed result even if a transfer block was added later.

`reserve` moves available coins to reserved and uses the operation ID as the
reservation ID. `release` restores all reserved coins; `capture` consumes them in
full. The legacy capture consumes the remaining reserve once, with exact operation replay allowed.
`settle` atomically captures a partial amount and credits the buyer's balance;
concurrent fills and release serialize on account and reservation locks. It does
not implement gold movement or Bazaar fees. `reserve_offer` binds gold price into
idempotency without moving gold.
All methods own their transaction; a future purchase/settlement integration that
needs a shared transaction must add that composition explicitly before activation.

Identity adapters receive the transaction connection and must read authoritative
state, locking identity rows as appropriate to serialize security changes. Only
`Active` and explicit transfer `Allowed` permit new effects. Missing, ambiguous,
stale or failed lookups must deny. Both transfer parties are checked. Integrations
must keep their identity/wallet locking protocol consistent to avoid lock cycles.

The staff caller enforces authentication and RBAC. `AuditHook` must verify actor
authorization and insert audit evidence using the supplied transaction connection,
or deny. Its event contains actor, reason, operation, before and after balances.
Never send external effects from a hook; use a transactional outbox. This crate
has no permissive production adapters. The test adapters are local to the tests.
Viewer authorization for history and balance remains the caller's responsibility.
History uses descending entry UUIDs with a bounded limit and an exclusive cursor.
`snapshot_in` reads balance and a revision (operation count + 1) in a caller transaction.

Runtime SQL uses bound parameters, without query macros or offline metadata.
The new migration preserves existing data; rollback means disabling the wallet
and reverting application code, preserving financial history. Runtime users must
not own schema or have privileges to disable triggers. Triggers reject updates,
deletes and truncation of entries and financial operation records.

## Validation

PostgreSQL tests create isolated, randomly named schemas and apply all migrations.
They read `PORTAL_TEST_DATABASE_URL`; if unset, each prints a skip notice and
returns. With the URL set, twelve tests exercise concurrency, idempotency, account
and block denial, reservations, auditing, immutable history, bad input and overflow.
Run `cargo test --locked -p oteryn-wallet -- --nocapture` to see skip notices.
No browser testing applies because this task exposes only a Rust API.

Paid top-ups use the caller-owned adjustment transaction with `AdjustmentKind::TopUp`
(`top_up` in `wallet_operations`; entries inherit their operation's kind). This
classification accepts only positive, self-account credits with the stable
`wallet-top-up:<operation UUID>` reason. Binary composition validates the paid
intent and fixed checkout package before calling it. The intent UUIDv7 is the
operation key, independent of outbox delivery ID. Existing APIs keep their kinds.
Migration `20261107180001_wallet_topup` extends only the kind constraint; its
inverse refuses while any immutable `top_up` operation exists.

Audited reward and staff credits use `grant`, an explicit reward/staff kind, actor,
reason and the transactional audit adapter; they credit the one balance like a
top-up. The historical `wallet_entries.coin_class` column (migration
`20261109071943_t125_coin_classes`) is retired by
`20261112100012_wallet_single_coin`: new entries take its default and balances,
receipts and history ignore it, so rows of either class count. Every
mutation now returns a complete original receipt; `operation` reconstructs that
receipt from immutable entries through the operation's entry sequence, so later
activity cannot change a replay. UUID ordering is used only for compatibility with
the original pre-sequence schema in the existing additive-upgrade harness.
History includes the direction-aware counterparty. The coin wire keeps its
`coin-wire-v1` members: `transferable` and `transferable_after` carry the whole
available balance, `non_transferable` and `non_transferable_after` are `0` and
`coin_class` is `transferable`. Rollback disables flags and preserves financial
history; do not discard ledger rows or partial fills.

Coin debt (owner decision 2026-11-14): a held purchase reversal is debt, and
`coin_outstanding_debts` (migration `20261114100004_coin_debt_settlement`) is
the one definition of what is still owed. Every credit (transfer in, top-up,
positive adjustment, reward, staff grant, market fill) enters whole, then pays
the account's debt oldest first in the same transaction under the account lock:
one `debt_settlement` operation (`reference_id` = the credit) takes the paid
coins back, and each debt paid gets a `coin_debt_settlements` row and a
`wallet.coin_debt_settled` audit row. The receipt of the credit includes it.
Releasing a reservation returns own coins and settles nothing. While debt is
outstanding, `transfer` (sender) and `reserve_offer` refuse with
`Refusal::DebtOutstanding` (code `coin_debt_outstanding`; `transfer_blocked`
on the coin wire, which has no debt code). `outstanding_debt` reads the total.
A module that moves coins in its own transaction (bazaar) holds the wallet
account lock and calls `unlocked_in` before an outgoing effect and `credit_in`
for a credit, so its credits settle debt the same way.
PostgreSQL tests: `tests/debt.rs`.

`wire::configured_router` reads `PORTAL_COIN_TRANSFERS_ENABLED`,
`PORTAL_COIN_MARKET_ENABLED` and `PORTAL_GAME_COINS_IDENTITIES`. Both flags default
off. The identity value is a JSON object of exact certificate SANs to empty arrays;
other configured internal identity maps and SANs participate in overlap detection.
Production adapters read active, verified accounts and the existing D14 transfer
block in the wallet transaction; lookup errors deny. Audited credits are not exposed
on this consumer-only wire. See the accepted coin wire contract for bounds, error
bodies and Game-side name resolution and gold ownership. Two additional PostgreSQL
HTTP tests round-trip fixtures through real handlers; a transport test proves
default-off and missing verified peer refusals without a database.
