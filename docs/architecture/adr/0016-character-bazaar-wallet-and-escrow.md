# ADR 0016 — Character Bazaar wallet, escrow and settlement

## Status

Accepted — 2026-07-28

## Context

Oteryn Platform already owns authenticated Identity, immutable Identity-to-Canary account binding, administrator RBAC/audit and Platform persistence. Canary owns character rows and game runtime. Existing shared-write contracts approve only account provisioning and character creation; generic Canary access remains read-only.

A Character Bazaar introduces two security-critical capabilities:

1. transferring an existing character between Canary accounts;
2. reserving and settling a currency balance under concurrent bids.

Leaving a listed character on the seller account would let the seller log in and change the advertised asset. Reusing Canary coin columns is not acceptable because the broad Canary contract records an unresolved tournament-coin schema/code conflict and no approved balance-write boundary. Platform and Canary use separate database connections, so one ACID transaction cannot cover both persistence owners.

## Decision

### 1. Introduce bounded Marketplace and Wallet modules

`Marketplace` owns auction policy, listing snapshots, bids, watchlists, seller/bidder histories and the recoverable cross-database settlement state machine.

`Wallet` owns Oteryn Coins balances, reservations and append-oriented ledger entries. Oteryn Coins are Platform business data and are not Canary `coins`, `coins_transferable`, `tournament_coins`, bank gold or a payment-provider balance.

Initial funding is restricted to an audited administrator adjustment. Buying coins, payment providers, refunds and chargebacks remain outside this ADR and require a later payment threat model.

### 2. Escrow characters before an auction becomes active

Each environment configures one Canary account ID dedicated to marketplace escrow. The account:

- has no Platform Identity binding;
- has no usable game-login credential exposed to the Platform or operators performing routine marketplace work;
- is created and protected out of band;
- may hold more than the normal player-facing character quota;
- is used only by the operation-specific transfer adapter.

A listing starts in `escrow_pending`. The transfer adapter moves `players.account_id` from the seller's exact server-resolved bound account to the escrow account. The public auction becomes `active` only after reconciliation confirms the character is owned by escrow and no `cluster_sessions` row exists for the player.

The delayed activation closes the bounded in-flight-login window between Canary's ownership check and cluster-session acquisition without modifying Canary. If quiescence cannot be proven, the auction remains unavailable and recoverable; the Platform never invents an active listing.

### 3. Use an operation-specific Canary transfer boundary

The dedicated `canary_character_transfer` principal receives only:

- column-level `SELECT` on the exact account, player and cluster-session fields required by the contract;
- column-level `UPDATE (account_id)` on `players`;
- no INSERT, DELETE, DDL, account credential, coin or unrelated player-field privilege.

Every transfer:

1. locks source and target `accounts` rows in ascending ID order;
2. locks the exact `players` row;
3. verifies active state and expected current owner;
4. rejects any existing `cluster_sessions` row for the player;
5. enforces the normal target character quota when the target is a buyer/seller account, but not when the target is the dedicated escrow account;
6. updates only `players.account_id` with an expected-owner predicate;
7. classifies an already-target-owned row as idempotent success;
8. returns bounded outcomes without exposing SQL details.

The evidence pin and exact grants are governed by `docs/contracts/CHARACTER_TRANSFER_CONTRACT.md`.

### 4. Freeze a public-safe character snapshot at listing

The auction stores an immutable JSON snapshot generated before escrow from an explicit allowlist of character fields. It contains display data such as name, level, vocation, sex, magic level, skills and outfit identifiers, but no account ID, credentials, IP data, sessions or private security fields.

The public detail page reads the snapshot, not live mutable Canary data. This keeps the advertised asset deterministic and prevents an external dependency failure from silently changing auction content.

### 5. Use a Platform append-oriented wallet ledger

Each Identity has one wallet row containing transactionally maintained `available_balance` and `reserved_balance`. Every mutation also appends one immutable ledger entry with:

- signed available/reserved deltas;
- operation type;
- unique idempotency key;
- optional auction reference;
- bounded non-secret metadata.

The invariant is `available_balance >= 0` and `reserved_balance >= 0`. Business actions lock wallet rows before calculating or applying deltas. Ledger idempotency keys prevent duplicate effects during retries/reconciliation.

### 6. Reserve the current leading bid

Bids are direct ascending bids. A bid must:

- come from an authenticated Identity with a ready Canary binding;
- not come from the seller;
- arrive while the auction is active and before `ends_at`;
- meet the configured minimum start/increment;
- lock the auction, current leading bid and affected wallet rows deterministically;
- reserve the new leader's amount;
- release the former leader's reservation exactly once;
- update the auction's current price, leader and bid count in the same Platform transaction.

Buy-now uses the same reservation path and then moves the auction to settlement. No browser-supplied account ID is ownership evidence.

### 7. Settle through an idempotent saga

Settlement cannot be one cross-database transaction. The durable saga is:

1. lock and mark the auction `settlement_pending` in Platform storage;
2. transfer the character from escrow to the winner's server-resolved bound Canary account;
3. record the observed transfer result;
4. atomically convert the winner's reserved balance into seller proceeds and configured commission in Platform storage;
5. mark the bid won and auction completed.

Each step has a deterministic idempotency key. Reconciliation reads the actual Canary owner and the Platform ledger to continue safely after interruption. A partial state is visible to operators as `recovery_required`; it is never reported as completed to the user.

Cancellation is allowed only before a valid bid exists unless an explicit administrator recovery path is used. Normal cancellation returns escrow to the seller and releases reservations before becoming terminal.

### 8. Require exact privileged controls

Administrator balance adjustments and recovery actions require:

- authenticated current session;
- confirmed MFA;
- exact `marketplace.manage` permission;
- validated reason and bounded amount;
- append-oriented wallet ledger entry;
- bounded administrator audit event without character snapshot bodies, credentials or raw exception text.

### 9. Keep product defaults configurable and bounded

Initial defaults are configuration-owned and validated:

- allowed durations: 1, 3 or 7 days;
- minimum starting bid: 100 Oteryn Coins;
- minimum bid increment: 10 Oteryn Coins;
- commission: 10% (`1000` basis points);
- public bid history: latest 20 entries;
- escrow activation quiescence: 30 seconds.

Changing these values is a product/configuration change, not a database contract change. Invalid production configuration fails closed.

## Consequences

- Sellers cannot keep playing or mutate a character after an auction becomes active.
- Buyers and sellers see deterministic snapshot data even when Canary reads are temporarily unavailable.
- Concurrent bidding and retries preserve wallet integrity through locks and idempotent ledger entries.
- Cross-database interruption remains possible, but it is explicit, recoverable and observable instead of being misrepresented as atomic.
- The escrow account and dedicated database principal become deployment prerequisites.
- Oteryn Coins launch without purchasing; commercial funding remains a separately reviewed later phase.
- The initial snapshot is intentionally bounded. Additional inventory, store, outfit, achievement or quest presentation requires explicit allowlist/grant expansion and acceptance coverage.

## Rejected alternatives

### Leave the character on the seller account until auction close

Rejected. The seller could log in, consume resources, change equipment or otherwise alter the advertised asset.

### Copy the character into a Platform database

Rejected. Canary remains the semantic owner of game characters and dependent state keyed by `player_id`.

### Reuse Canary coin columns

Rejected. No approved balance-write contract exists, the tournament-coin shape is conflicted, and shared mutable balance fields would couple marketplace integrity to game-runtime assumptions.

### Attempt a distributed transaction across Platform and Canary

Rejected. The deployed components do not provide one proven distributed transaction coordinator. A durable idempotent saga is simpler to operate and test honestly.

### Activate the auction immediately after one offline check

Rejected. Canary ownership verification and session acquisition are separate operations. Delayed reconciliation is required before public activation.

## Follow-up

Implementation, grant verification, real-MariaDB transfer tests, wallet concurrency tests, reconciliation operations and responsive browser acceptance are owned by `OTERYN-20260728-character-bazaar` and draft PR #270.
