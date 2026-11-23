# Account closure qualification (GAP-20)

## Player and operator views

Players open **Account closure** from account navigation or Privacy. The page
shows the latest request, its reference and the grace deadline while it is open.
A request or cancellation requires the account password, a recent security
confirmation, and a valid browser form token. Cancellation leaves a history
entry. If Game work was already issued, the page shows restoration pending until
the matching accepted Game receipt and projection are present.

On the admin host, **Account closure status** looks up an exact closure
reference, including after the Platform account has closed and its sessions have
been revoked. The panel requires an active staff session with MFA,
`admin.access` and `audit.view`. It displays the Platform and Game states and
supports historical references. Share the reference with support before the
account closes. The lookup has no retry, override or disposition controls.

## Interpreting the status

| Platform state | What the player or operator can establish |
|---|---|
| Scheduled | The request is recorded and the grace period is still open. |
| Pending | The grace period elapsed and the Platform account is still open. A verified close receipt can be present while further checks are pending. |
| Cancelled | The Platform request was cancelled. If work was issued, follow the Game restoration status independently. |
| Finalized | The Platform closed the account and revoked its credentials. Follow the Game finalization acknowledgement independently. |

A failed Game operation comes from the retained refusal for the current attempt.
Reconciliation can issue another attempt under the existing retry rules.
Unavailable means the necessary Game evidence is absent or no longer matches
the current projection epoch. Conflicting retained evidence requires operator
review through the existing reconciliation process. A terminal Game confirmation
in the view requires the matching receipt and projection. It acknowledges the
closure decision and does not attest to physical erasure.

Recorded requests remain visible while intake is paused.
`PORTAL_ACCOUNT_TERMINATION_ENABLED` and `PORTAL_ACCOUNT_CLOSURE_ENABLED` default
to false and both are required for new requests. Pausing intake alone allows the
configured Game adapter to drain existing work. With the Game adapter off, only
cancellations that require no Game command can proceed.

## Retention

Platform finalization revokes credentials, removes account recovery addresses,
closes open support tickets and replaces the account email and public display
name. The account tombstone and closure history remain. Financial records and
records subject to legal retention stay with their owning domains. Immutable
commands, receipts and generations remain available for reconciliation and
replay protection. The closure-decision audit payload expires after 90 days
from the original Game decision under the existing audit retention worker.

## Remaining Game qualification

- Demonstrate the real Game producer pulling the immutable closure intents,
  authenticating publications and reporting closure receipts, closure
  projections and independently complete character projections.
- Exercise active-session fencing, reversible quarantine, restoration after
  cancellation, and the exclusive final disposition decision across restart,
  replay and recovery-generation changes.
- Confirm the owner-domain disposition and retention of characters, items,
  bank balances, guilds, houses and market obligations before launch. Platform
  finalization has no Game-owned physical erasure step.
