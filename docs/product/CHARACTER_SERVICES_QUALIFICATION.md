# GAP-19 character services qualification

## Platform

From the account overview, an owner can open Character services for a character
listed in the current account projection. The link uses the stable character ID
and appears when `OTERYN_CHARACTER_SERVICES_ENABLED=true`. The page requires an
active eligible account and current ownership and service evidence. Missing or
stale evidence produces the localized unavailable state. A browser submission
requires a live session with recent authentication, CSRF and explicit confirmation.

The existing v2 Platform issuer supports rename, schedule and cancel. Scheduling
also requires the cancel flag. The cancel flag can stay on while scheduling is
paused. Use the existing `OTERYN_CHARACTER_SERVICES_*` browser flags and
`PORTAL_CHARACTER_SERVICES_*` private listener flags. They default to off.
Rename also requires `OTERYN_WALLET_ENABLED` and a valid configured product for
paid orders. The private exchange and qualified source projection are required
for submissions. The current exchange and service projection are qualified for
testing and preproduction. Keep the exchange open to drain accepted work when
new submissions stop.

The owner page distinguishes a queued request, a verified receipt, and the
current Game projection. A committed receipt can arrive before its projection.
The receipt can be shown as history while new requests wait for a matching or
later revision. A deletion deadline comes from Game evidence and is displayed in
UTC with the time remaining at page load. The local offer to cancel closes when
the database clock plus configured clock uncertainty reaches that deadline. Final deletion requires Game evidence. The cancel request binds
the exact schedule operation and must reach Game before its effective time with
the character out of session.

Platform reserves the agreed coins and queues the intent in one transaction.
A verified commit captures the reservation and a verified terminal refusal
normally releases it. Receipt replay leaves the existing settlement intact.
Pending or contradictory outcomes require reconciliation using the existing
operation. A paid service lost after a verified restore can grant a single
replacement without a second charge. The owner history identifies these
financial states from stored order evidence.

## Operator status

Accounts holding both `admin.access` and `game.player.manage` can open Character
service operations from the account overview or visit
`/{lang}/admin/character-services`. A live MFA grant is required. The read-only counts cover pending requests,
requests pending for at least 300 seconds, reopened receipts awaiting a new
receipt, operations in the reconciliation queue and unused replacement credits.
The view remains available while submissions are paused and shows aggregate
counts only. An aged operation is a signal to reconcile the same operation via
the existing authorized recovery process.

## Game qualification before activation

- Pin the accepted producer and consumer revisions, command schemas, scope
  identities and current capability advertisement for rename, schedule and
  cancel. Demonstrate a durable, immutable receipt for each applied mutation
  and same-ID republication after response loss.
- Verify production PKI, clock and freshness bounds, independent service
  projection revisions and the shared LCFA epoch. Exercise both receipt and
  projection arrival orders, interruption, resync and restore at the selected
  backup retention horizon.
- Verify Game chooses and enforces its deletion deadline, offline eligibility
  and the 30-day name lock. Exercise cancellation before the deadline, refusal
  at the deadline and race with finalization.
- Qualify the configured rename product and the verified commit, refusal and
  recovery paths with the real Game producer and Platform wallet. Record
  acceptance and obtain separate activation approval.
- World and channel transfer and sex change require separate service contract
  agreement, including destination eligibility and confirmed effects. These
  services are unavailable in the portal.

The v2 proposal records the owner decisions and remaining qualification
checklists. [Character services command contract proposal](../contracts/OTERYN_V2_CHARACTER_SERVICES_COMMAND_CONTRACT_PROPOSAL.md)
