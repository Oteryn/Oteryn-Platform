# Premium purchase qualification

Platform records Premium Time when an accepted Store purchase debits coins and writes its order, fulfilment unit and Premium grant in one transaction. The grant is effective according to its Platform ledger interval. The Game delivery acknowledgement is registration evidence, as specified by the [Premium Time purchase amendment](../contracts/OTERYN_V2_PREMIUM_TIME_STORE_PURCHASE_AMENDMENT_2026-11-18.md).

## Platform views

- With `PANEL_PREMIUM_PAYMENTS=true`, open **Premium** from the account navigation to see entitlement status, UTC expiry and paginated grant and revocation history. A purchase receipt opens from the applicable grant and lists the Store order, total coin debit and recorded Premium expiry. A single order can contain multiple Premium grants.
- **Payments** shows fiat payment history and links to the original invoice for a settled payment, including its payment reference and current payment state. A fiat top-up and a subsequent Premium purchase are separate transactions. Wallet coins are fungible, so the Premium receipt identifies the exact Store order and debit.
- On **Staff → Commerce**, enter a Platform account UUID to inspect its current Platform entitlement, UTC expiry and lifecycle revision. The existing staff guard, Premium management permission and `OTERYN_PREMIUM_ENABLED` apply. Unknown accounts return 404. Unavailable entitlement data returns 503.
- Account history and receipts use the signed-in account. A foreign receipt returns 404. A malformed receipt query returns 400. A rejected Premium ledger projection returns 503 before it can display status or receipts.

The panel, Store purchases, Premium management, Premium snapshots and loyalty exports retain their independent default-off flags. Enabling a browser view does not start a provider, publish a snapshot, advance a revision or enable Game delivery.

## Qualification in a disposable environment

Run `specs/gap-40.spec.mjs` with the repository browser launcher and its outbox harness. The spec funds an account through the test bank and existing worker, submits a Store purchase using the harness's local mTLS peer, and checks Platform reads in both locales and at 390 px. Verify both the fiat invoice and the coin order receipt, and confirm that a second account receives 404. Focused Rust coverage checks an invalid lifecycle revision and operator lookup permissions. Use the existing staff commerce spec to qualify operational grants and approvals.

## Game and provider decisions

- Game must qualify Premium snapshot consumption, expiry and revocation enforcement, including stale or conflicting revisions, retry and recovery. Verify that a Premium Time delivery registers the purchase without adding Premium again.
- Ratify the [loyalty snapshot and stream contract](../contracts/OTERYN_V2_LOYALTY_SNAPSHOT_CONTRACT.md) for the target deployment. Game must qualify monotonic imports, checkpoint replay and recovery before publishing ranks or consuming loyalty benefits. Pending this evidence, the Platform views show Game status as unavailable.
- Live payment-provider activation and the related settlement and refund rehearsal belong to GAP-41. Local test-bank evidence qualifies the Platform views and ledger only.
