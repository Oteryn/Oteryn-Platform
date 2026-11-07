# Wallet core

`Ledger` is a Rust API with no HTTP or player/staff UI. Construct it with a pool,
`WalletFlags` and explicit account-state, transfer-block and audit adapters.
`WalletFlags::default()` disables every API, including reads. No application flag
or role dispatch is wired by this task.

There is one transferable coin; all values are `i64` in the smallest unit.
Available and reserved balances are sums of immutable entry deltas, read under
`SELECT ... FOR UPDATE` on the account row. Transferable equals available.
Available, reserved and their total cannot be negative or exceed `i64::MAX`.
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
full. A reservation can be settled once, with exact operation replay allowed.
Capture does not credit another account or implement Bazaar settlement fees.
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

Runtime SQL uses bound parameters, without query macros or offline metadata.
The new migration preserves existing data; rollback means disabling the wallet
and reverting application code, preserving financial history. Runtime users must
not own schema or have privileges to disable triggers. Triggers reject updates,
deletes and truncation of entries and financial operation records.

## Validation

PostgreSQL tests create isolated, randomly named schemas and apply all migrations.
They read `PORTAL_TEST_DATABASE_URL`; if unset, each prints a skip notice and
returns. With the URL set, eight tests exercise concurrency, idempotency, account
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
