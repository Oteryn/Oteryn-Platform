# Character Bazaar operations

## Status

`CHARACTER BAZAAR RECOVERY RUNBOOK`

This runbook applies to the Character Bazaar implementation governed by ADR 0016 and `docs/contracts/CHARACTER_TRANSFER_CONTRACT.md`.

## Deployment prerequisites

Before exposing `/bazaar` in any environment:

1. deploy the exact Platform release and run Platform migrations;
2. create one Canary account dedicated to Character Bazaar escrow;
3. ensure the escrow account is not bound to any Platform Identity;
4. ensure the escrow account has no normal user-facing login credential path;
5. provision `oteryn_character_transfer` from `database/provisioning/canary-character-transfer.sql.template` using secret management;
6. set `MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID` to the escrow account ID;
7. set the `CANARY_CHARACTER_TRANSFER_DB_*` secrets;
8. run `php artisan canary:verify-character-transfer-db-privileges`;
9. run `php artisan production:verify-configuration` in production configuration;
10. run `php artisan marketplace:reconcile --limit=1000` before enabling traffic;
11. confirm the scheduler invokes `marketplace:reconcile` every minute and prevents overlap.

A missing escrow account, invalid escrow ID, excessive database grant or unavailable transfer dependency is a release blocker. Do not reuse the Canary game-process, read-only, provisioning or character-create database credential.

## Normal state machine

Listing:

`escrow_pending / escrow_requested`
→ Canary transfer to escrow
→ `escrow_pending / quiescence_wait`
→ second owner/session check after the configured quiescence period
→ `active / active`.

Settlement:

`active`
→ `settlement_pending / transfer_to_winner`
→ Canary transfer to winner
→ append-only buyer/seller wallet entries
→ `completed / done`.

No-bid expiry or seller cancellation:

`active` or `escrow_pending`
→ `cancel_pending / return_to_seller`
→ Canary return to seller
→ `expired / done` or `cancelled / done`.

Any state that cannot be classified safely becomes:

`recovery_required / recovery_required`.

## Routine reconciliation

The scheduler runs:

```text
php artisan marketplace:reconcile --limit=100
```

A manual bounded run is safe:

```text
php artisan marketplace:reconcile --limit=1000
```

The command is idempotent. It reads actual Canary ownership and Platform wallet ledger idempotency keys before continuing. Repeated execution must not duplicate reservations, proceeds or ownership transfers.

Exit outcomes:

- `0`: the bounded run completed and no processed auction remains in recovery;
- `1`: the command could not inspect or process the dependency;
- `2`: at least one processed auction requires explicit operator recovery.

## Operator recovery workflow

Use `/admin/marketplace` only with an authenticated, current, MFA-confirmed Identity holding `marketplace.manage`.

For each recovery auction:

1. record the auction ID, public character name, failure code and saga state;
2. check application/database health and correct the dependency before retrying;
3. verify the transfer credential with `canary:verify-character-transfer-db-privileges`;
4. verify the configured escrow account ID has not changed;
5. use **Run bounded recovery** once;
6. refresh and confirm the resulting state;
7. inspect the administrator audit event `marketplace.auction_recovery_requested`;
8. if recovery remains required, stop retries and investigate actual ownership plus ledger entries.

Never edit `players.account_id`, wallet balances, ledger rows, auction status or bid status manually as routine recovery.

## Ownership reconciliation matrix

For a recovery auction, actual Canary owner determines only these safe paths:

- escrow owns the character, there is a winning bidder: continue winner transfer/settlement;
- escrow owns the character, auction ended without bids: return to seller;
- escrow owns the character, auction is still within its end time: restore active state;
- escrow owns the character, public timing was never initialized: repeat quiescence before activation;
- original seller owns the character and there are no bids: complete cancellation;
- ready winning bidder owns the character: continue only the Platform wallet settlement step;
- any other owner, deleted character or cluster-session row: keep recovery required and escalate.

A cluster-session row blocks transfer regardless of its status or expiry field. Resolve the game session through normal Canary operations; do not delete session rows from the Platform credential.

## Wallet operations

Oteryn Coins are Platform-owned. They are not Canary account coins, transferable coins, tournament coins or character bank gold.

Administrator funding uses `/admin/marketplace` and requires:

- target Platform Identity email;
- signed non-zero amount within the configured hard limit;
- operational reason between 10 and 500 characters;
- generated idempotency request ID;
- confirmed MFA and `marketplace.manage`.

The wallet mutation and administrator audit event commit in the same Platform transaction. The ledger stores a hash of the reason rather than unrestricted reason text.

Never update `wallet_accounts` directly. The append-oriented `wallet_ledger_entries` idempotency key is the authoritative mutation trace.

## Failure codes

Common bounded failure codes:

- `binding_not_ready`, `seller_binding_not_ready`, `winner_binding_not_ready` — repair the Platform Identity-to-Canary binding before retry;
- `invalid_escrow_configuration` — correct the escrow account configuration and verify it exists;
- `character_online_or_session_active` — wait for or resolve the normal Canary logout/session lifecycle;
- `ownership_conflict` — actual owner is not the expected seller, escrow or winner; stop automated retry;
- `character_deleted` — character is not eligible; investigate and return funds/ownership only through reviewed recovery;
- `target_character_limit` — winner or seller target account is full; coordinate account capacity before retry;
- `dependency_unavailable`, `platform_persistence_unavailable` — restore database/network dependency and rerun reconciliation;
- `auction_state_conflict`, `wallet_reservation_conflict` — stop and inspect auction, bids and ledger as one unit.

Public responses must not expose SQL messages, credentials, account IDs or exception traces.

## Monitoring and alerts

Alert on:

- any `recovery_required` auction older than five minutes;
- an increasing count of `escrow_pending` auctions older than the quiescence interval plus two scheduler cycles;
- `settlement_pending` older than five minutes;
- repeated `dependency_unavailable`, ownership or wallet conflict failure codes;
- reconciliation command exit `1` or `2`;
- transfer privilege verification failure;
- negative-balance invariant attempts;
- scheduler overlap or scheduler absence.

Useful non-secret dimensions are auction ID, saga state, bounded failure code, duration in state and correlation ID. Do not log snapshots, account IDs, credentials, session IDs or raw database errors.

## Backup and restore

Platform backups must include together:

- `character_auctions`;
- `character_auction_bids`;
- `character_auction_watches`;
- `wallet_accounts`;
- `wallet_ledger_entries`;
- `admin_audit_events`.

After restoring Platform data:

1. keep marketplace writes unavailable;
2. verify the restored schema and exact release SHA;
3. verify the transfer credential and escrow configuration;
4. run `marketplace:reconcile --limit=1000` repeatedly until no immediately processable rows remain;
5. review every recovery-required auction against actual Canary ownership;
6. enable marketplace traffic only after wallet invariants and ownership reconciliation pass.

Canary and Platform backups are not transactionally simultaneous. Restore never assumes historical `players.account_id` from a Platform snapshot; actual current Canary ownership is inspected during reconciliation.

## Rollback

Application rollback is safe only when the target release understands every persisted marketplace state. Do not roll back to a release without Character Bazaar support while pending auctions exist.

Before a code rollback:

1. disable new listings, bids, purchases and administrator adjustments;
2. allow reconciliation to drain pending operations;
3. confirm no `escrow_pending`, `settlement_pending`, `cancel_pending` or `recovery_required` rows remain;
4. back up Platform marketplace/wallet/audit tables;
5. retain the Character Bazaar migrations and data even if routes are disabled;
6. deploy the prior compatible release;
7. verify read-only history and wallet invariants.

Never solve rollback by deleting auctions, ledger entries or the escrow account.

## Incident escalation evidence

Collect only:

- exact Platform release SHA;
- auction ID;
- status, saga state and bounded failure code;
- timestamps and lock version;
- actual Canary owner classification: seller, escrow, expected winner or other;
- presence/absence of a cluster-session row;
- matching wallet ledger idempotency keys and signed deltas;
- administrator audit action IDs;
- reconciliation command result.

Redact Canary account IDs, credentials, email addresses, session identifiers, IP addresses and character snapshot bodies from shared incident records.
