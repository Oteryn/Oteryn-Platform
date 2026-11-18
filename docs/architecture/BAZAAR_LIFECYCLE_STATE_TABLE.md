# Bazaar lifecycle state table

Bazaar remains off by default. The listing, sale, command, custody, funds, and
rebuild records form one lifecycle. A Game projection can describe custody
without authorizing a local financial transition. Keep independently sourced
receipts, decision audits, and status projections separate until their bindings
have been verified.

## Persisted state

| Dimension | Values | Durable authority |
| --- | --- | --- |
| Listing | Absent, Draft, Active, Matched, AwaitingDelivery, Completed, Cancelled, Expired | `bazaar_listings.state`. Matched can carry a legacy moderation hold |
| Item delivery | Absent, Pending (staged or dispatched), Committed (receipt pending audit), Audited (verified commitment), Rejected (receipt, with or without current audit), Drift | Sale ID plus listing ID and Deliver command form the canonical issuance key. The outbox, dispatch, receipts, decisions, conflicts and `bazaar_delivery_evidence` audit rows carry its state |
| Character custody | None, Lock, Bind, CC1, Escrow, Returned | Audited Lock and Bind, bound CC1 transfer receipt and current STATUS. Return includes the verified release leg |
| Item custody | None, Escrow, Delivered, Returned, Unknown, Drift | Audited Escrow, Deliver or Return and matching STATUS. Missing status leaves custody unknown |
| Funds | None, Held, Settled, Refunded | Immutable sale/reservation, coin hold/disposition and wallet ledger, or Game gold hold/payout/release receipts |
| Gold bid | Pending, Releasing, Accepted, Refused | `bazaar_authority_gold_bids.phase`. A verified hold establishes the reservation. A release required by the bid remains in flight until its outcome is audited |
| Offer | Pending, Accepted, Rejected, Expired | `bazaar_offers.state`. The item offer service reserves coins for fixed-price offers and refuses gold |
| Certification | Missing, Rebuilding, Current, Obsolete, Fenced | Marker and progress bound to projection epoch, recovery generation, scan version and sorted configured worlds. Proof also requires current evidence and a fresh watermark |
| Work | No key, Dirty, Waiting (0–8 attempts), Resolved, Quarantined | Coalesced delivery or listing recheck key, rotating scan cursor, rebuild listing/operation cursors and durable delivery audit |

## Transition rules

A Draft requires current audited custody and qualification before publication.
For characters, Lock anchors custody, Bind authorizes the particular CC1
transfer, and the transfer receipt and STATUS establish the escrow or return.
A rejected character deposit can still need an audited unlock before it is
closed. The corresponding item deposit requires audited Escrow.

An eligible bid holds funds. Matching binds the reservation to an immutable
sale ID. The winning sale can enter AwaitingDelivery only from Active or
Matched. An unbid expiry with Game custody starts a verified return.
Ordinary listings without Game custody can expire without a return command.

Receipt admission, audit admission and STATUS intake are independent. A
receipt is pending authority until its decision audit is current and bound
to the same command. An audited Deliver authorizes payout only with matching
delivery evidence and settlement funding. An audited Deliver rejection
authorizes the seller-return leg. A buyer's loss of trade eligibility only
permits a return before possible handoff or after audited proof of no effect.
Legacy item details can supply the persisted asset and world binding when
the authority-asset row is absent.

The issuer lock serializes Game evidence, scheduling keys and rebuild
checkpoints. A page can advance past unresolved subjects while their durable
keys continue to fence certification. A rebuild rolls back a failing
transaction. The next worker resumes from the last committed cursor and
requires the current scan version and configured worlds before serving.

The v5 scan binds the sorted worlds to the marker through its progress row.
It raises the database minimum writer version to 5 under the issuer lock.
Older intake and rebuild writes fail at the database boundary, including
checkpoint changes from an alternating older worker. Legacy checkpoints
without an evidence epoch require a fresh scan.

Each transaction that changes custody or delivery evidence advances the
issuer evidence epoch once. A scan records that epoch at the start and
checks it in the transaction that writes the marker. A mismatch clears both
enumeration cursors, keeps the dirty keys, and reports changed evidence.
After three consecutive resets the scan waits for two successive rebuild
calls to observe the same epoch before trying the first page again.
Watermarks advance independently of the evidence epoch.

An older marker or progress row becomes obsolete as soon as the fence,
version or world set changes. A reader checks that binding under the issuer
lock even when a worker has yet to commit an invalidation. A current marker
also needs a fresh watermark and no unresolved listing or terminal sale
keys. During ordinary operation an awaiting sale uses its own custody and
funding gates while its delivery key waits for evidence. During a rebuild,
all enumerated keys must resolve before the final operation scan can certify.

The rebuild tracks listing enumeration separately from the durable recheck
queues. A missing listing status can retain a key and let the next page
proceed. Delivery rechecks resolve the world from the authority asset, then
from retained item details. A missing world or configured policy pauses
retry charges and drift reports until the world is available. The delivery retry budget belongs to the sale, so restarting
the scan or rediscovering the same key preserves the attempts already charged.

## Executable transition rows

The PostgreSQL acceptance test uses five products plus funding and recovery scenarios.
Each product fixes the orthogonal records in its fixture. The listing writer
product exercises enum writes in isolation. The listing control product uses
the browser and staff callers. The projection product uses Game intake.
The delivery evidence and sale STATUS products exercise the rebuild writer after
Game intake.

### Listing state writer

Every allowed write adds one listing audit entry. Every impossible write is
rejected by the listing writer. Funds, Game custody and sale records are
preserved in every row. The special AwaitingDelivery-to-Cancelled transition
runs through the compensation scenarios with a proven return.

| Row ID | Current | Requested state | Next | Disposition |
| --- | --- | --- | --- | --- |
| `LW-Draft-Draft` | Draft | Draft | Draft | Impossible because an enum write requires a state transition |
| `LW-Draft-Active` | Draft | Active | Active | Allowed |
| `LW-Draft-Matched` | Draft | Matched | Draft | Impossible because the lifecycle has no direct edge |
| `LW-Draft-AwaitingDelivery` | Draft | AwaitingDelivery | Draft | Impossible because the lifecycle has no direct edge |
| `LW-Draft-Completed` | Draft | Completed | Draft | Impossible because the lifecycle has no direct edge |
| `LW-Draft-Cancelled` | Draft | Cancelled | Cancelled | Allowed |
| `LW-Draft-Expired` | Draft | Expired | Draft | Impossible because the lifecycle has no direct edge |
| `LW-Active-Draft` | Active | Draft | Active | Impossible because the lifecycle has no direct edge |
| `LW-Active-Active` | Active | Active | Active | Impossible because an enum write requires a state transition |
| `LW-Active-Matched` | Active | Matched | Matched | Allowed |
| `LW-Active-AwaitingDelivery` | Active | AwaitingDelivery | AwaitingDelivery | Allowed |
| `LW-Active-Completed` | Active | Completed | Active | Impossible because the lifecycle has no direct edge |
| `LW-Active-Cancelled` | Active | Cancelled | Cancelled | Allowed |
| `LW-Active-Expired` | Active | Expired | Expired | Allowed |
| `LW-Matched-Draft` | Matched | Draft | Matched | Impossible because the lifecycle has no direct edge |
| `LW-Matched-Active` | Matched | Active | Matched | Impossible because the lifecycle has no direct edge |
| `LW-Matched-Matched` | Matched | Matched | Matched | Impossible because an enum write requires a state transition |
| `LW-Matched-AwaitingDelivery` | Matched | AwaitingDelivery | AwaitingDelivery | Allowed |
| `LW-Matched-Completed` | Matched | Completed | Matched | Impossible because the lifecycle has no direct edge |
| `LW-Matched-Cancelled` | Matched | Cancelled | Matched | Impossible because the lifecycle has no direct edge |
| `LW-Matched-Expired` | Matched | Expired | Matched | Impossible because the lifecycle has no direct edge |
| `LW-AwaitingDelivery-Draft` | AwaitingDelivery | Draft | AwaitingDelivery | Impossible because the lifecycle has no direct edge |
| `LW-AwaitingDelivery-Active` | AwaitingDelivery | Active | AwaitingDelivery | Impossible because the lifecycle has no direct edge |
| `LW-AwaitingDelivery-Matched` | AwaitingDelivery | Matched | AwaitingDelivery | Impossible because the lifecycle has no direct edge |
| `LW-AwaitingDelivery-AwaitingDelivery` | AwaitingDelivery | AwaitingDelivery | AwaitingDelivery | Impossible because an enum write requires a state transition |
| `LW-AwaitingDelivery-Completed` | AwaitingDelivery | Completed | Completed | Allowed |
| `LW-AwaitingDelivery-Cancelled` | AwaitingDelivery | Cancelled | AwaitingDelivery | Impossible without the compensation writer and verified return |
| `LW-AwaitingDelivery-Expired` | AwaitingDelivery | Expired | AwaitingDelivery | Impossible because the lifecycle has no direct edge |
| `LW-Completed-Draft` | Completed | Draft | Completed | Impossible because the listing is terminal |
| `LW-Completed-Active` | Completed | Active | Completed | Impossible because the listing is terminal |
| `LW-Completed-Matched` | Completed | Matched | Completed | Impossible because the listing is terminal |
| `LW-Completed-AwaitingDelivery` | Completed | AwaitingDelivery | Completed | Impossible because the listing is terminal |
| `LW-Completed-Completed` | Completed | Completed | Completed | Impossible because an enum write requires a state transition |
| `LW-Completed-Cancelled` | Completed | Cancelled | Completed | Impossible because the listing is terminal |
| `LW-Completed-Expired` | Completed | Expired | Completed | Impossible because the listing is terminal |
| `LW-Cancelled-Draft` | Cancelled | Draft | Cancelled | Impossible because the listing is terminal |
| `LW-Cancelled-Active` | Cancelled | Active | Cancelled | Impossible because the listing is terminal |
| `LW-Cancelled-Matched` | Cancelled | Matched | Cancelled | Impossible because the listing is terminal |
| `LW-Cancelled-AwaitingDelivery` | Cancelled | AwaitingDelivery | Cancelled | Impossible because the listing is terminal |
| `LW-Cancelled-Completed` | Cancelled | Completed | Cancelled | Impossible because the listing is terminal |
| `LW-Cancelled-Cancelled` | Cancelled | Cancelled | Cancelled | Impossible because an enum write requires a state transition |
| `LW-Cancelled-Expired` | Cancelled | Expired | Cancelled | Impossible because the listing is terminal |
| `LW-Expired-Draft` | Expired | Draft | Expired | Impossible because the listing is terminal |
| `LW-Expired-Active` | Expired | Active | Expired | Impossible because the listing is terminal |
| `LW-Expired-Matched` | Expired | Matched | Expired | Impossible because the listing is terminal |
| `LW-Expired-AwaitingDelivery` | Expired | AwaitingDelivery | Expired | Impossible because the listing is terminal |
| `LW-Expired-Completed` | Expired | Completed | Expired | Impossible because the listing is terminal |
| `LW-Expired-Cancelled` | Expired | Cancelled | Expired | Impossible because the listing is terminal |
| `LW-Expired-Expired` | Expired | Expired | Expired | Impossible because an enum write requires a state transition |

### Listing controls

These events leave the listing enum and custody unchanged. The bidder hold,
cancel request and moderation hold are the effects shown below. No custody
drift report is emitted by these controls.

| Row ID | Current | Event | Next | Effects | Disposition |
| --- | --- | --- | --- | --- | --- |
| `LC-Draft-Bid` | Draft | Bid | Draft | Preserve all | Impossible because bids require Active |
| `LC-Draft-Cancel` | Draft | Cancel | Draft | Record cancel request | Allowed |
| `LC-Draft-Suspend` | Draft | Suspend | Draft | Record moderation hold | Allowed |
| `LC-Active-Bid` | Active | Bid | Active | Hold funds and set leader | Allowed |
| `LC-Active-Cancel` | Active | Cancel | Active | Record cancel request | Allowed |
| `LC-Active-Suspend` | Active | Suspend | Active | Record moderation hold | Allowed |
| `LC-Matched-Bid` | Matched | Bid | Matched | Preserve all | Impossible because bids require Active |
| `LC-Matched-Cancel` | Matched | Cancel | Matched | Preserve all | Impossible because the request requires Draft or Active with no leader |
| `LC-Matched-Suspend` | Matched | Suspend | Matched | Preserve all | Impossible because this fixture lacks a suspendible pre-handoff state |
| `LC-AwaitingDelivery-Bid` | AwaitingDelivery | Bid | AwaitingDelivery | Preserve all | Impossible because bids require Active |
| `LC-AwaitingDelivery-Cancel` | AwaitingDelivery | Cancel | AwaitingDelivery | Preserve all | Impossible because the request requires Draft or Active with no leader |
| `LC-AwaitingDelivery-Suspend` | AwaitingDelivery | Suspend | AwaitingDelivery | Preserve all | Impossible because this fixture lacks a suspendible pre-handoff state |
| `LC-Completed-Bid` | Completed | Bid | Completed | Preserve all | Impossible because delivery evidence awaits terminal recheck |
| `LC-Completed-Cancel` | Completed | Cancel | Completed | Preserve all | Impossible because the request requires Draft or Active with no leader |
| `LC-Completed-Suspend` | Completed | Suspend | Completed | Preserve all | Impossible because this fixture lacks a suspendible pre-handoff state |
| `LC-Cancelled-Bid` | Cancelled | Bid | Cancelled | Preserve all | Impossible because bids require Active |
| `LC-Cancelled-Cancel` | Cancelled | Cancel | Cancelled | Preserve all | Impossible because the request requires Draft or Active with no leader |
| `LC-Cancelled-Suspend` | Cancelled | Suspend | Cancelled | Preserve all | Impossible because this fixture lacks a suspendible pre-handoff state |
| `LC-Expired-Bid` | Expired | Bid | Expired | Preserve all | Impossible because bids require Active |
| `LC-Expired-Cancel` | Expired | Cancel | Expired | Preserve all | Impossible because the request requires Draft or Active with no leader |
| `LC-Expired-Suspend` | Expired | Suspend | Expired | Preserve all | Impossible because this fixture lacks a suspendible pre-handoff state |

### Game custody projection intake

Game intake changes the durable projection independently of financial and
listing effects. A conflict creates a projection quarantine and schedules a
custody recheck in the same transaction. An applied change also requeues
listings an unfinished rebuild has already scanned. The sale recheck
determines whether conflicting sale evidence creates a drift report.

| Row ID | Current status | Incoming revision | Next status | Effects | Disposition |
| --- | --- | --- | --- | --- | --- |
| `CP-locked-Newer` | locked | Newer | locked | Replace projection | Allowed |
| `CP-locked-Older` | locked | Older | locked | Preserve and supersede incoming | Impossible to apply because revision is older |
| `CP-locked-Replay` | locked | Replay | locked | Preserve and acknowledge duplicate | Allowed |
| `CP-locked-Conflict` | locked | Conflict | locked | Preserve prior and quarantine subject | Quarantined |
| `CP-ESCROWED-Newer` | ESCROWED | Newer | ESCROWED | Replace projection | Allowed |
| `CP-ESCROWED-Older` | ESCROWED | Older | ESCROWED | Preserve and supersede incoming | Impossible to apply because revision is older |
| `CP-ESCROWED-Replay` | ESCROWED | Replay | ESCROWED | Preserve and acknowledge duplicate | Allowed |
| `CP-ESCROWED-Conflict` | ESCROWED | Conflict | ESCROWED | Preserve prior and quarantine subject | Quarantined |
| `CP-DELIVERED-Newer` | DELIVERED | Newer | DELIVERED | Replace projection | Allowed |
| `CP-DELIVERED-Older` | DELIVERED | Older | DELIVERED | Preserve and supersede incoming | Impossible to apply because revision is older |
| `CP-DELIVERED-Replay` | DELIVERED | Replay | DELIVERED | Preserve and acknowledge duplicate | Allowed |
| `CP-DELIVERED-Conflict` | DELIVERED | Conflict | DELIVERED | Preserve prior and quarantine subject | Quarantined |
| `CP-RETURNED-Newer` | RETURNED | Newer | RETURNED | Replace projection | Allowed |
| `CP-RETURNED-Older` | RETURNED | Older | RETURNED | Preserve and supersede incoming | Impossible to apply because revision is older |
| `CP-RETURNED-Replay` | RETURNED | Replay | RETURNED | Preserve and acknowledge duplicate | Allowed |
| `CP-RETURNED-Conflict` | RETURNED | Conflict | RETURNED | Preserve prior and quarantine subject | Quarantined |
| `CP-UNLOCKED-Newer` | UNLOCKED | Newer | UNLOCKED | Replace projection | Allowed |
| `CP-UNLOCKED-Older` | UNLOCKED | Older | UNLOCKED | Preserve and supersede incoming | Impossible to apply because revision is older |
| `CP-UNLOCKED-Replay` | UNLOCKED | Replay | UNLOCKED | Preserve and acknowledge duplicate | Allowed |
| `CP-UNLOCKED-Conflict` | UNLOCKED | Conflict | UNLOCKED | Preserve prior and quarantine subject | Quarantined |
| `CP-INVALID-Newer` | INVALID | Newer | INVALID | Replace projection | Allowed |
| `CP-INVALID-Older` | INVALID | Older | INVALID | Preserve and supersede incoming | Impossible to apply because revision is older |
| `CP-INVALID-Replay` | INVALID | Replay | INVALID | Preserve and acknowledge duplicate | Allowed |
| `CP-INVALID-Conflict` | INVALID | Conflict | INVALID | Preserve prior and quarantine subject | Quarantined |

### Deliver receipt and audit admission

Each row starts with an awaiting sale, its canonical dispatched Deliver, held
buyer funds and an ESCROWED projection. Committed and Rejected here describe the
stored receipt. The audit event supplies the exact retained receipt digest or,
if no receipt exists, the expected commitment digest. Receipt inputs after an
existing receipt either replay its exact bytes or contradict its outcome.
An authenticated conflicting replay at the same fence preserves the original
receipt and immediately fences the sale. A later-generation first rejection of
a prior commitment is admitted for audit and cannot authorize a return. Its
current audit confirms the reversal and triggers the sale drift report.
Every row preserves the funds, escrow and canonical operation. Reconciliation
reports Drift once and fences an unresolved rebuild.

| Row ID | Current | Event | Next | Intake effect | Work and report | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `DA-Pending-Commitment` | Pending | Commit receipt | Committed | Store receipt | Keep dirty and charge one attempt | Allowed |
| `DA-Pending-Rejection` | Pending | Reject receipt | Rejected | Store receipt | Keep dirty and charge one attempt | Allowed |
| `DA-Pending-Audit` | Pending | Decision audit | Pending | No write because receipt is absent | Keep dirty and charge one attempt | Impossible until receipt is admitted |
| `DA-Committed-Commitment` | Committed | Commit receipt | Committed | Acknowledge duplicate | Keep dirty and charge one attempt | Allowed |
| `DA-Committed-Rejection` | Committed | Reject receipt | Drift | Keep original and record conflict | Report once and fence certification | Quarantined |
| `DA-Committed-Audit` | Committed | Decision audit | Audited commitment | Store audit | Wait for matching DELIVERED and charge one attempt | Allowed |
| `DA-Rejected-Commitment` | Rejected | Commit receipt | Drift | Keep original and record conflict | Report once and fence certification | Quarantined |
| `DA-Rejected-Rejection` | Rejected | Reject receipt | Rejected | Acknowledge duplicate | Keep dirty and charge one attempt | Allowed |
| `DA-Rejected-Audit` | Rejected | Decision audit | Audited rejection | Store audit | Resolve delivery work and enable certification | Allowed |
| `DA-AuditedCommitment-Commitment` | Audited commitment | Commit receipt | Audited commitment | Acknowledge duplicate | Wait for matching DELIVERED and charge one attempt | Allowed |
| `DA-AuditedCommitment-Rejection` | Audited commitment | Reject receipt | Drift | Keep original and record conflict | Report once and fence certification | Quarantined |
| `DA-AuditedCommitment-Audit` | Audited commitment | Decision audit | Audited commitment | Store matching attestation | Wait for matching DELIVERED and charge one attempt | Allowed |
| `DA-AuditedRejection-Commitment` | Audited rejection | Commit receipt | Drift | Keep original and record conflict | Report once and fence certification | Quarantined |
| `DA-AuditedRejection-Rejection` | Audited rejection | Reject receipt | Audited rejection | Acknowledge duplicate | Resolve delivery work and enable certification | Allowed |
| `DA-AuditedRejection-Audit` | Audited rejection | Decision audit | Audited rejection | Store matching attestation | Resolve delivery work and enable certification | Allowed |
| `DA-Drift-Commitment` | Drift | Commit receipt | Drift | Acknowledge the retained commitment | Preserve one report and fence certification | Quarantined |
| `DA-Drift-Rejection` | Drift | Reject receipt | Drift | Record conflict against retained commitment | Preserve one report and fence certification | Quarantined |
| `DA-Drift-Audit` | Drift | Decision audit | Drift | Store matching attestation | Preserve one report and fence certification | Quarantined |

### Sale evidence and STATUS during a rebuild

Here **Audited** means a completed sale with a current audited commitment.
**Rejected** means an awaiting sale with a current audited rejection and no
audited return yet. **Refunded** means a cancelled sale with an audited Return.
STATUS for the matching terminal outcome is bound to its actual operation. A
nonmatching claim is provided without an audited operation for that claim.
These rows preserve the current funds, listing enum and canonical Deliver
identity. Verified evidence can clear the key and complete certification if
the remaining scans are complete.

| Row ID | Sale evidence | STATUS event | Next evidence | Work and report | Disposition |
| --- | --- | --- | --- | --- | --- |
| `SS-Pending-Escrowed` | Pending | ESCROWED | Pending | Keep dirty and charge one attempt | Allowed |
| `SS-Pending-Delivered` | Pending | DELIVERED | Pending | Keep dirty and charge one attempt | Allowed |
| `SS-Pending-Returned` | Pending | RETURNED | Pending | Keep dirty and charge one attempt | Allowed |
| `SS-Pending-Missing` | Pending | Missing | Pending | Keep dirty and charge one attempt | Allowed |
| `SS-Committed-Escrowed` | Committed | ESCROWED | Committed | Keep dirty and charge one attempt | Allowed |
| `SS-Committed-Delivered` | Committed | DELIVERED | Committed | Keep dirty and charge one attempt | Allowed |
| `SS-Committed-Returned` | Committed | RETURNED | Committed | Keep dirty and charge one attempt | Allowed |
| `SS-Committed-Missing` | Committed | Missing | Committed | Keep dirty and charge one attempt | Allowed |
| `SS-Audited-Escrowed` | Audited | ESCROWED | Drift | Keep report count at one and fence certification | Quarantined |
| `SS-Audited-Delivered` | Audited | DELIVERED | Audited | Clear dirty key and enable certification | Allowed |
| `SS-Audited-Returned` | Audited | RETURNED | Drift | Keep report count at one and fence certification | Quarantined |
| `SS-Audited-Missing` | Audited | Missing | Audited | Keep dirty and charge one attempt | Allowed |
| `SS-Rejected-Escrowed` | Rejected | ESCROWED | Rejected | Clear dirty key and enable certification | Allowed |
| `SS-Rejected-Delivered` | Rejected | DELIVERED | Drift | Keep report count at one and fence certification | Quarantined |
| `SS-Rejected-Returned` | Rejected | RETURNED | Rejected | Keep dirty and charge one attempt | Allowed |
| `SS-Rejected-Missing` | Rejected | Missing | Rejected | Keep dirty and charge one attempt | Allowed |
| `SS-Refunded-Escrowed` | Refunded | ESCROWED | Drift | Keep report count at one and fence certification | Quarantined |
| `SS-Refunded-Delivered` | Refunded | DELIVERED | Drift | Keep report count at one and fence certification | Quarantined |
| `SS-Refunded-Returned` | Refunded | RETURNED | Refunded | Clear dirty key and enable certification | Allowed |
| `SS-Refunded-Missing` | Refunded | Missing | Refunded | Keep dirty and charge one attempt | Allowed |
| `SS-Drift-Escrowed` | Drift | ESCROWED | Drift | Keep report count at one and fence certification | Quarantined |
| `SS-Drift-Delivered` | Drift | DELIVERED | Drift | Keep report count at one and fence certification | Quarantined |
| `SS-Drift-Returned` | Drift | RETURNED | Drift | Keep report count at one and fence certification | Quarantined |
| `SS-Drift-Missing` | Drift | Missing | Drift | Keep report count at one and fence certification | Quarantined |

### Gold bids and item offers

Gold bid phases advance through Runtime bid admission and its gold bid worker.
An accepted proxy bid may leave an incumbent as leader. A release needed to
resolve a bid keeps the request in Releasing until Game supplies the audited
outcome. The item offer service holds coins while a fixed-price offer is Pending.

| Row ID | Current | Event | Next | Funds and work | Disposition |
| --- | --- | --- | --- | --- | --- |
| `GOLD-BID-SUBMIT` | Absent | Submit eligible gold auction bid | Pending | Dispatch one Hold and preserve the existing leader | Allowed |
| `GOLD-BID-REFUSE-HOLD` | Pending | Audited Hold refusal | Refused | No hold and no Release | Allowed |
| `GOLD-BID-ACCEPT-HOLD` | Pending | Audited Hold and eligible winning bid | Accepted | Keep the hold and set the leader | Allowed |
| `GOLD-BID-RELEASE-REFUSE` | Pending | Audited Hold followed by refusal of an ineligible bidder | Releasing, then Refused on audited Release | Hold until Game confirms one excess release | Allowed |
| `GOLD-BID-RELEASE-ACCEPT` | Pending | Audited Hold and an eligible bid that leaves the incumbent as leader | Releasing, then Accepted on audited Release | Hold until Game confirms one outbid release | Allowed |
| `GOLD-BID-RELEASE-RETRY` | Releasing | Release expires and its rejection is audited | Releasing until replacement Release commits | Preserve the obsolete intent and dispatch an immutable replacement | Allowed |
| `OFFER-SUBMIT` | Absent | Submit fixed-price coin offer and replay the request | Pending | Reserve the buyer's coins once | Allowed |
| `OFFER-ACCEPT` | Pending | Seller accepts | Accepted, listing Matched | Preserve the reservation and freeze one sale | Allowed |
| `OFFER-REJECT` | Pending | Seller rejects | Rejected, listing Active | Restore the reservation once | Allowed |
| `OFFER-EXPIRE` | Pending | Listing expires, then Game returns the item | Expired, listing Expired | Preserve the hold until the audited return, then restore it | Allowed |
| `OFFER-GOLD-REFUSED` | Absent | Submit a fixed-price gold item offer | Absent | Preserve the wallet and write no offer | Impossible through the coin-only item service |

### Recovery and cross-record transitions

These rows seed the linked records needed to exercise a complete lifecycle
path. A row can include a second input to prove that a later replay or retry
preserves the first outcome. The PostgreSQL acceptance test compares the
executed row IDs with the required IDs declared in these tables. BZ-13 also
runs as a standalone SQL guard test.

| Row ID | Current | Event | Next | Funds | Custody | Work and report | Disposition |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `REC-01` | Absent | Create | Draft | None | Stage deposit | Publish one escrow intent | Allowed |
| `REC-02` | Draft, no Game effect | Lock pending | Draft | None | Await Lock | Preserve operation | Allowed |
| `REC-03` | Draft, Lock pending | Audited Lock rejection | Cancelled or awaiting release of surviving Lock | None | Close only with verified no effect or unlock | Record refusal | Allowed |
| `REC-04` | Draft, Escrow pending | Audited Escrow rejection | Cancelled | None | No deposit effect | Record refusal | Allowed |
| `REC-05` | Draft, audited Escrow | Advance | Active | None | Escrow | Preserve proof | Allowed |
| `REC-06` | Draft, audited Escrow | Cancel requested | Cancelled after audited Return | None | Returned | Dispatch one return | Allowed |
| `REC-07` | Active, no bid | Expiry | Cancelled after audited Return | None | Returned | Dispatch one return | Allowed |
| `REC-08` | Active, moderation hold | Expiry | Active | Preserve | Escrow | Preserve hold with no Return dispatch | Allowed to defer |
| `REC-09` | Active, held funds | Expiry | AwaitingDelivery | Held | Escrow | Bind one sale | Allowed |
| `REC-10` | Active, moderation hold | Approved invalidation | Cancelled | Release if reserved | Returned | Resolve hold after return | Allowed |
| `REC-11` | Matched, legacy hold | Retry tick | AwaitingDelivery | Preserve reservation | Escrow | Retire hold once and resume settlement | Allowed |
| `REC-12` | AwaitingDelivery, dispatched Deliver | Suspend | AwaitingDelivery | Preserve | Escrow | Record moderation hold after dispatch fence | Allowed |
| `REC-13` | AwaitingDelivery, dispatched Deliver | Audited commitment and Delivered status | Completed | Settle once | Delivered | One completed sale. A direct repeat write after completion is refused | Allowed |
| `REC-14` | AwaitingDelivery, dispatched Deliver | Audited rejection and audited Returned status | Cancelled | Refund once | Returned | Record one compensation. Refuse a Deliver operation and a direct repeat insert after cancellation | Allowed |
| `REC-15` | AwaitingDelivery, canonical and alternate staged | Second connection publishes alternate, then canonical publishes | AwaitingDelivery with canonical dispatched | Held | Escrow | Exactly one dispatch | Alternate impossible because its key cannot authorize publication |
| `BZ-08` | AwaitingDelivery, receipt without audit | BZ-08 retry bound | Drift, listing unchanged | Held | Quarantine | Queue survives eight retries. Report once and keep custody fenced after late audit | Quarantined |
| `REC-17` | AwaitingDelivery, Return dispatched after audited Deliver rejection | Return receipt remains missing through retry bound | Drift, listing unchanged | Held | Quarantine | Report once. Deliver rejection does not clear unresolved Return work | Quarantined |
| `REC-18` | AwaitingDelivery, Return dispatched before Deliver | Return receipt remains missing through retry bound | Drift, listing unchanged | Held | Quarantine | Report once even without a Deliver operation | Quarantined |
| `REC-19` | AwaitingDelivery, Return dispatched after audited Deliver rejection | Audited Return rejection | Drift, listing unchanged | Held | Quarantine | Report once. Preserve return fence and require operator reconciliation | Quarantined |
| `BZ-11` | AwaitingDelivery, unpublished alternate | BZ-11 buyer becomes ineligible | Cancelled after audited Return | Refund once | Returned | Ignore staged alternate. Independently evidenced handoff still fences return | Allowed |
| `REC-21` | Completed, old evidence | Receipt before audit | Completed | Settled | Preserve | Wait for audit, then resolve dirty key and certify | Allowed |
| `REC-22` | Completed, old evidence | STATUS ESCROWED contradicts delivered custody | Drift, listing unchanged | Settled | Quarantine | Report once | Quarantined |
| `REC-23` | Completed, old evidence | Retry bound | Drift, listing unchanged | Settled | Quarantine | Report once even after receipt replay | Quarantined |
| `REC-24` | Completed, current evidence | Worker tick | Completed | Preserve settlement | Delivered | No repeat handoff | Allowed |
| `REC-25` | Completed, certified | Contradictory status changes after certification | Drift, listing unchanged | Preserve settlement | Quarantine | Recheck and revoke certification | Quarantined |
| `REC-26` | Completed, behind cursor | Status producer commits between pages | Drift, listing unchanged | Preserve settlement | Quarantine | Committed dirty key forces recheck behind cursor | Quarantined |
| `BZ-10` | Completed, obsolete marker | BZ-10 same-fence version or world-set change and rebuild failure | Rebuilding or fenced | Preserve settlement | Preserve | Obsolete marker cannot serve with fresh watermark | Allowed rebuild failure |
| `BZ-09` | Cancelled, refunded | BZ-09 ESCROWED status contradicts Return | Drift, listing unchanged | Preserve refund | Quarantine | Report once and revoke certification | Quarantined |
| `REC-29` | Cancelled, refunded | Reversal of rejected Deliver | Drift, listing unchanged | Preserve refund | Quarantine | Report the conflicting commitment once | Quarantined |
| `REC-30` | Rebuild, old epoch | Rebuild start | Current after scan | Preserve | Preserve | Old coordinator cannot certify. The correct epoch can finish | Allowed |
| `REC-31` | Rebuild, world absent | Configured world missing | Fenced until corrected | Preserve | Preserve | No sale retry charged for configuration gap | Allowed to defer |
| `BZ-12` | Rebuild, early missing STATUS | BZ-12 page checkpoint before contradictory legacy sale | Later sale in Drift | Preserve | Quarantine affected sale | Advance past missing listing and report the later sale once without new intake | Allowed enumeration with later sale quarantined |
| `REC-33` | Rebuild, completed legacy item with no authority asset row | Page finds contradictory Deliver evidence | Drift, listing unchanged | Preserve settlement | Quarantine | Enumerate by the retained sale and command, then report once | Quarantined |
| `BZ-14` | Rebuild, Active escrow behind cursor | BZ-14 Conflicting STATUS commits between pages | Active, certification pending | Preserve | Projection quarantined | Commit the listing dirty key with the conflict. The final page leaves certification fenced | Quarantined |
| `BZ-15` | Rebuild, completed legacy item with missing configured world | BZ-15 Retry checks without current Deliver evidence | Completed, certification pending until the world and evidence return | Preserve settlement | Preserve | Resolve the world from retained item details. Charge no retries and report no drift while the world is missing | Allowed to defer |
| `BZ-14b` | Rebuild, Active escrow behind cursor | BZ-14b Applied STATUS names a missing operation | Active, then certified after replacement and a clean scan | Preserve | Wait for valid evidence | Keep the listing key until the replacement resolves it | Allowed to defer |
| `BZ-16` | Rebuild, Escrow behind operation cursor | BZ-16 Receipt or decision audit conflict | Fenced | Preserve | Conflicted evidence | Restart enumeration and recheck the invalidated operation | Quarantined |
| `BZ-17` | Rebuild, completed legacy item with missing configured world | BZ-17 Immediate drift with world missing | Completed, then certified after world and evidence recover | Preserve | Preserve | Charge no retries and record no drift until the world resolves | Allowed to defer |
| `BZ-18` | Rebuild, v5 checkpoint | BZ-18 Legacy writer attempts evidence and checkpoint writes | Rebuilding at the same cursor | Preserve | Preserve | Reject both writes and keep certification fenced until the v5 scan finishes | Impossible for the older writer |
| `BZ-19` | Rebuild, paged operation scan | BZ-19 Evidence epoch changes | Restarted from page one | Preserve | Recheck | Withhold the marker and pause after three consecutive resets | Allowed to defer |
| `REC-41` | Rebuild, early Drift | Later page | Both contradictions in Drift | Preserve | Quarantine affected sales | Report both once and continue past early Drift | Quarantined |
| `REC-42` | Rebuild, page in flight | Writer connection closes before commit | Last committed cursor, then replayed page | Preserve | Preserve | Observer cannot read abandoned checkpoint. Retry commits successor | Allowed |
| `REC-43` | AwaitingDelivery, Return audited | Second worker races to compensate | Cancelled | Refund once | Returned once | One compensation, one Return dispatch, and one buyer refund ledger entry | Allowed |
| `SALE-GOLD-COMPLETE` | Active, then AwaitingDelivery with a gold sale | Attempt both completion writes | Mark the sale complete only from AwaitingDelivery | Preserve | Preserve | Verify the completion marker is absent after the refused write | Impossible from Active |
| `BZ-13` | SQL guard, production macro definition | BZ-13 nested concat!/include_str! assembles a direct state write | Rejected by the guard | Preserve | Preserve | Report the assembled production SQL | Impossible to bypass the lifecycle writer guard |


## Invariants

- A sale can release value only along its proven outcome. Buyer delivery may
  settle the seller's funds. Seller return may refund the buyer. Delivery and
  refund, or return and seller settlement, cannot both be authorized.
- Each item sale has exactly one canonical Deliver identity derived from its
  immutable sale key. At most one outbox operation may use that key. Before
  issuance a sale can be safely returned with no published Deliver. Alternate
  staged records grant no authority and any independent commitment must be checked.
- Certification requires current audited evidence and matching rebuild version,
  worlds and fence. The scanned evidence epoch must still match at certification. A fresh watermark alone grants no custody authority.
- A published Deliver or Return with unresolved sale evidence shares one
  budget of eight durable waiting attempts for the sale.
  On the next unresolved check it goes to Drift and is reported once. Scanning
  other listings continues while certification stays fenced.
- Checkpoints, dirty-key changes and evidence applied by a page commit together.
  World-policy gaps leave financial data and the retry budget untouched.
