# Evidence — OTERYN-20260807 marketplace terminal-recovery audit

## Target

- Repository: `blakinio/Oteryn-Platform`
- Audited Marketplace runtime: `1ab8d90be35745f8020b2026d6d75ed777ccf76f`
- Package base/main refresh: `7dbb35e2257bd3265d4dc75a1723bf6a315afa80`
- Finding: `OPA-REC-0001` / Issue #804
- Risk / priority: HIGH / P1

## Main-delta evidence

`main` advanced by one commit while this bounded audit was running. The delta from `1ab8d90...` to `7dbb35e...` changes CI workflow/routing files and the CI repair task only. No `app/Marketplace/**`, `app/CanaryIntegration/**` or Marketplace test path changed, so the Marketplace evidence below remains current on the package base.

## Failure-path evidence

- `ReconcileCharacterAuctions::reconcile()` catches `MarketplaceException` and generic `Throwable` and sends both to `markRecovery()`.
- `markRecovery()` updates the auction by primary key only, setting `status = recovery_required` and `saga_state = recovery_required` without a row lock, current-status read or eligible-status predicate.
- Success paths use transactions and `lockForUpdate()` and explicitly inspect current auction state.
- `CharacterAuction::isTerminal()` defines `completed`, `cancelled` and `expired` as terminal.

## Cross-database concurrency evidence

- Settlement transfers character ownership before the Platform wallet/status transaction.
- `CanaryCharacterTransfer::attemptTransfer()` locks the relevant accounts and player.
- If ownership is already the requested target, Canary transfer returns `ALREADY_TRANSFERRED`.
- `CanaryCharacterTransferConcurrencyMariaDbTest` proves competing transfers serialize at the Canary boundary.
- Therefore the confirmed defect is not a duplicate-transfer primitive; it is the later unconditional Platform recovery write.

## Falsified terminal-state invariant

A stale/failing reconciler may enter `markRecovery()` after another reconciler has already committed a terminal state. Because the fallback update is unconditional, the late error can overwrite `completed`, `cancelled` or `expired` with `recovery_required`.

For completed settlement this is especially inconsistent: the winning bid is already `won`, wallet ledger effects are finalized and the character is already owned by the winner. `RecoverCharacterAuction` can route such a regressed row back toward settlement, but `settle()` requires a `leading` bid and therefore cannot complete the already-settled `won` bid through its normal path.

## Test evidence

- `MarketplaceSettlementRecoveryTest` proves ordinary exactly-once settlement and explicit recovery scenarios.
- `CanaryCharacterTransferConcurrencyMariaDbTest` proves Canary transfer serialization.
- No inspected test covers a stale reconciliation failure that executes the generic recovery fallback after another worker has made the Platform auction terminal.

## Duplicate search

Open and closed Issue searches for Character Bazaar settlement/recovery concurrency and terminal-state regression found only completed parent Issue #269, not this root cause. No open PR owned the Marketplace recovery paths when Issue #804 was created.

## Safety

No Marketplace runtime, wallet state, Canary database, production/staging environment or external repository was mutated by this audit.