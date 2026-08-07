# Audit report — Marketplace terminal recovery monotonicity

## Scope

Repository-only audit of Character Bazaar settlement/recovery concurrency and terminal-state integrity. Marketplace runtime was inspected at `main@1ab8d90be35745f8020b2026d6d75ed777ccf76f`; before packaging, current main was refreshed to `7dbb35e2257bd3265d4dc75a1723bf6a315afa80` and the intervening commit was proven not to touch Marketplace runtime/tests.

No runtime, workflow, production, staging, Canary database or external-repository mutation was performed by this audit package.

## Verdict

`FINDING — HIGH / P1 / PROVEN`

Finding: `OPA-REC-0001` / Issue #804.

## Path reviewed

```text
CharacterAuction settlement/cancellation reconciliation
  -> external Canary ownership transfer
  -> Platform DB terminal transaction
  -> concurrent/stale worker exception
  -> ReconcileCharacterAuctions::markRecovery()
  -> unconditional status = recovery_required
```

## Primary evidence

1. `reconcile()` catches both Marketplace and generic failures and calls `markRecovery()` with only the auction ID and failure code.
2. `markRecovery()` updates by primary key and does not lock/re-read the current row or constrain the update to eligible non-terminal states.
3. Settlement, begin-close and cancellation success paths do use row locks and current-state checks.
4. `CharacterAuction` explicitly defines `completed`, `cancelled` and `expired` as terminal.
5. Canary transfer itself is idempotent for the same target and uses account/player row locks; dedicated MariaDB concurrency coverage proves competing transfers serialize.
6. A late worker-local/transient failure can therefore execute the generic Platform recovery fallback after another worker has successfully committed `completed` or another terminal state, regressing that terminal state to `recovery_required`.
7. After successful settlement, the winning bid is already `won`. Recovery of the regressed row can recognize the winner as actual owner, but the normal settlement path requires a `leading` bid, so the false recovery state cannot simply replay the already-finalized settlement path.
8. Existing recovery tests do not cover the stale-error-after-terminal interleaving.

## Expected invariant

Terminal Character Bazaar saga states must be monotonic. A stale error must not supersede a newer successful terminal decision.

## Actual behavior

The generic failure fallback can write `recovery_required` over any current auction status because it has no current-state guard.

## Impact

A character can already belong to the winner and wallet/bid settlement can already be finalized while the Platform auction row is later represented as `recovery_required`. That creates contradictory operational truth across character ownership, wallet ledger, bid state and auction state, and can strand the record in manual recovery.

The audit did not find evidence that this path duplicates character transfer or wallet coins: the inspected Canary transfer and ledger operations have separate idempotency mechanisms. The material defect is terminal-state regression and recovery consistency at a value-transfer boundary.

## Duplicate analysis

Open and closed Issue searches for Marketplace/Character Bazaar settlement recovery concurrency and terminal regression found no independently actionable duplicate. Completed Issue #269 is the broader Bazaar implementation owner, not this concrete root cause.

## Remediation handoff

Issue #804 owns remediation. The repair should make recovery transition atomically conditional on current eligible non-terminal state and add deterministic stale-worker/concurrent-terminal regression coverage without broadening into a Marketplace redesign.

## Audit delivery E2E

`NOT_APPLICABLE`: documentation/evidence-only audit package.