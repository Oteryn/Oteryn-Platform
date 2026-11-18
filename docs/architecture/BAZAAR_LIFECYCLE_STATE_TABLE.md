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

## Executable transition rows

The PostgreSQL acceptance test uses five products and named recovery scenarios.
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

| Current | Requested state | Next | Disposition |
| --- | --- | --- | --- |
| Draft | Draft | Draft | Impossible because an enum write requires a state transition |
| Draft | Active | Active | Allowed |
| Draft | Matched | Draft | Impossible because the lifecycle has no direct edge |
| Draft | AwaitingDelivery | Draft | Impossible because the lifecycle has no direct edge |
| Draft | Completed | Draft | Impossible because the lifecycle has no direct edge |
| Draft | Cancelled | Cancelled | Allowed |
| Draft | Expired | Draft | Impossible because the lifecycle has no direct edge |
| Active | Draft | Active | Impossible because the lifecycle has no direct edge |
| Active | Active | Active | Impossible because an enum write requires a state transition |
| Active | Matched | Matched | Allowed |
| Active | AwaitingDelivery | AwaitingDelivery | Allowed |
| Active | Completed | Active | Impossible because the lifecycle has no direct edge |
| Active | Cancelled | Cancelled | Allowed |
| Active | Expired | Expired | Allowed |
| Matched | Draft | Matched | Impossible because the lifecycle has no direct edge |
| Matched | Active | Matched | Impossible because the lifecycle has no direct edge |
| Matched | Matched | Matched | Impossible because an enum write requires a state transition |
| Matched | AwaitingDelivery | AwaitingDelivery | Allowed |
| Matched | Completed | Matched | Impossible because the lifecycle has no direct edge |
| Matched | Cancelled | Matched | Impossible because the lifecycle has no direct edge |
| Matched | Expired | Matched | Impossible because the lifecycle has no direct edge |
| AwaitingDelivery | Draft | AwaitingDelivery | Impossible because the lifecycle has no direct edge |
| AwaitingDelivery | Active | AwaitingDelivery | Impossible because the lifecycle has no direct edge |
| AwaitingDelivery | Matched | AwaitingDelivery | Impossible because the lifecycle has no direct edge |
| AwaitingDelivery | AwaitingDelivery | AwaitingDelivery | Impossible because an enum write requires a state transition |
| AwaitingDelivery | Completed | Completed | Allowed |
| AwaitingDelivery | Cancelled | AwaitingDelivery | Impossible without the compensation writer and verified return |
| AwaitingDelivery | Expired | AwaitingDelivery | Impossible because the lifecycle has no direct edge |
| Completed | Draft | Completed | Impossible because the listing is terminal |
| Completed | Active | Completed | Impossible because the listing is terminal |
| Completed | Matched | Completed | Impossible because the listing is terminal |
| Completed | AwaitingDelivery | Completed | Impossible because the listing is terminal |
| Completed | Completed | Completed | Impossible because an enum write requires a state transition |
| Completed | Cancelled | Completed | Impossible because the listing is terminal |
| Completed | Expired | Completed | Impossible because the listing is terminal |
| Cancelled | Draft | Cancelled | Impossible because the listing is terminal |
| Cancelled | Active | Cancelled | Impossible because the listing is terminal |
| Cancelled | Matched | Cancelled | Impossible because the listing is terminal |
| Cancelled | AwaitingDelivery | Cancelled | Impossible because the listing is terminal |
| Cancelled | Completed | Cancelled | Impossible because the listing is terminal |
| Cancelled | Cancelled | Cancelled | Impossible because an enum write requires a state transition |
| Cancelled | Expired | Cancelled | Impossible because the listing is terminal |
| Expired | Draft | Expired | Impossible because the listing is terminal |
| Expired | Active | Expired | Impossible because the listing is terminal |
| Expired | Matched | Expired | Impossible because the listing is terminal |
| Expired | AwaitingDelivery | Expired | Impossible because the listing is terminal |
| Expired | Completed | Expired | Impossible because the listing is terminal |
| Expired | Cancelled | Expired | Impossible because the listing is terminal |
| Expired | Expired | Expired | Impossible because an enum write requires a state transition |

### Listing controls

These events leave the listing enum and custody unchanged. The bidder hold,
cancel request and moderation hold are the effects shown below. No custody
drift report is emitted by these controls.

| Current | Event | Next | Effects | Disposition |
| --- | --- | --- | --- | --- |
| Draft | Bid | Draft | Preserve all | Impossible because bids require Active |
| Draft | Cancel | Draft | Record cancel request | Allowed |
| Draft | Suspend | Draft | Record moderation hold | Allowed |
| Active | Bid | Active | Hold funds and set leader | Allowed |
| Active | Cancel | Active | Record cancel request | Allowed |
| Active | Suspend | Active | Record moderation hold | Allowed |
| Matched | Bid | Matched | Preserve all | Impossible because bids require Active |
| Matched | Cancel | Matched | Preserve all | Impossible because the request requires Draft or Active with no leader |
| Matched | Suspend | Matched | Preserve all | Impossible because this fixture lacks a suspendible pre-handoff state |
| AwaitingDelivery | Bid | AwaitingDelivery | Preserve all | Impossible because bids require Active |
| AwaitingDelivery | Cancel | AwaitingDelivery | Preserve all | Impossible because the request requires Draft or Active with no leader |
| AwaitingDelivery | Suspend | AwaitingDelivery | Preserve all | Impossible because this fixture lacks a suspendible pre-handoff state |
| Completed | Bid | Completed | Preserve all | Impossible because delivery evidence awaits terminal recheck |
| Completed | Cancel | Completed | Preserve all | Impossible because the request requires Draft or Active with no leader |
| Completed | Suspend | Completed | Preserve all | Impossible because this fixture lacks a suspendible pre-handoff state |
| Cancelled | Bid | Cancelled | Preserve all | Impossible because bids require Active |
| Cancelled | Cancel | Cancelled | Preserve all | Impossible because the request requires Draft or Active with no leader |
| Cancelled | Suspend | Cancelled | Preserve all | Impossible because this fixture lacks a suspendible pre-handoff state |
| Expired | Bid | Expired | Preserve all | Impossible because bids require Active |
| Expired | Cancel | Expired | Preserve all | Impossible because the request requires Draft or Active with no leader |
| Expired | Suspend | Expired | Preserve all | Impossible because this fixture lacks a suspendible pre-handoff state |

### Game custody projection intake

Game intake changes the durable projection independently of financial and
listing effects. The conflict outcome creates a projection quarantine. The
sale recheck determines whether the conflicting evidence creates a sale
drift report.

| Current status | Incoming revision | Next status | Effects | Disposition |
| --- | --- | --- | --- | --- |
| locked | Newer | locked | Replace projection | Allowed |
| locked | Older | locked | Preserve and supersede incoming | Impossible to apply because revision is older |
| locked | Replay | locked | Preserve and acknowledge duplicate | Allowed |
| locked | Conflict | locked | Preserve prior and quarantine subject | Quarantined |
| ESCROWED | Newer | ESCROWED | Replace projection | Allowed |
| ESCROWED | Older | ESCROWED | Preserve and supersede incoming | Impossible to apply because revision is older |
| ESCROWED | Replay | ESCROWED | Preserve and acknowledge duplicate | Allowed |
| ESCROWED | Conflict | ESCROWED | Preserve prior and quarantine subject | Quarantined |
| DELIVERED | Newer | DELIVERED | Replace projection | Allowed |
| DELIVERED | Older | DELIVERED | Preserve and supersede incoming | Impossible to apply because revision is older |
| DELIVERED | Replay | DELIVERED | Preserve and acknowledge duplicate | Allowed |
| DELIVERED | Conflict | DELIVERED | Preserve prior and quarantine subject | Quarantined |
| RETURNED | Newer | RETURNED | Replace projection | Allowed |
| RETURNED | Older | RETURNED | Preserve and supersede incoming | Impossible to apply because revision is older |
| RETURNED | Replay | RETURNED | Preserve and acknowledge duplicate | Allowed |
| RETURNED | Conflict | RETURNED | Preserve prior and quarantine subject | Quarantined |
| UNLOCKED | Newer | UNLOCKED | Replace projection | Allowed |
| UNLOCKED | Older | UNLOCKED | Preserve and supersede incoming | Impossible to apply because revision is older |
| UNLOCKED | Replay | UNLOCKED | Preserve and acknowledge duplicate | Allowed |
| UNLOCKED | Conflict | UNLOCKED | Preserve prior and quarantine subject | Quarantined |
| INVALID | Newer | INVALID | Replace projection | Allowed |
| INVALID | Older | INVALID | Preserve and supersede incoming | Impossible to apply because revision is older |
| INVALID | Replay | INVALID | Preserve and acknowledge duplicate | Allowed |
| INVALID | Conflict | INVALID | Preserve prior and quarantine subject | Quarantined |

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

| Current | Event | Next | Intake effect | Work and report | Disposition |
| --- | --- | --- | --- | --- | --- |
| Pending | Commit receipt | Committed | Store receipt | Keep dirty and charge one attempt | Allowed |
| Pending | Reject receipt | Rejected | Store receipt | Keep dirty and charge one attempt | Allowed |
| Pending | Decision audit | Pending | No write because receipt is absent | Keep dirty and charge one attempt | Impossible until receipt is admitted |
| Committed | Commit receipt | Committed | Acknowledge duplicate | Keep dirty and charge one attempt | Allowed |
| Committed | Reject receipt | Drift | Keep original and record conflict | Report once and fence certification | Quarantined |
| Committed | Decision audit | Audited commitment | Store audit | Wait for matching DELIVERED and charge one attempt | Allowed |
| Rejected | Commit receipt | Drift | Keep original and record conflict | Report once and fence certification | Quarantined |
| Rejected | Reject receipt | Rejected | Acknowledge duplicate | Keep dirty and charge one attempt | Allowed |
| Rejected | Decision audit | Audited rejection | Store audit | Resolve delivery work and enable certification | Allowed |
| Audited commitment | Commit receipt | Audited commitment | Acknowledge duplicate | Wait for matching DELIVERED and charge one attempt | Allowed |
| Audited commitment | Reject receipt | Drift | Keep original and record conflict | Report once and fence certification | Quarantined |
| Audited commitment | Decision audit | Audited commitment | Store matching attestation | Wait for matching DELIVERED and charge one attempt | Allowed |
| Audited rejection | Commit receipt | Drift | Keep original and record conflict | Report once and fence certification | Quarantined |
| Audited rejection | Reject receipt | Audited rejection | Acknowledge duplicate | Resolve delivery work and enable certification | Allowed |
| Audited rejection | Decision audit | Audited rejection | Store matching attestation | Resolve delivery work and enable certification | Allowed |
| Drift | Commit receipt | Drift | Acknowledge the retained commitment | Preserve one report and fence certification | Quarantined |
| Drift | Reject receipt | Drift | Record conflict against retained commitment | Preserve one report and fence certification | Quarantined |
| Drift | Decision audit | Drift | Store matching attestation | Preserve one report and fence certification | Quarantined |

### Sale evidence and STATUS during a rebuild

Here **Audited** means a completed sale with a current audited commitment.
**Rejected** means an awaiting sale with a current audited rejection and no
audited return yet. **Refunded** means a cancelled sale with an audited Return.
STATUS for the matching terminal outcome is bound to its actual operation. A
nonmatching claim is provided without an audited operation for that claim.
These rows preserve the current funds, listing enum and canonical Deliver
identity. Verified evidence can clear the key and complete certification if
the remaining scans are complete.

| Sale evidence | STATUS event | Next evidence | Work and report | Disposition |
| --- | --- | --- | --- | --- |
| Pending | ESCROWED | Pending | Keep dirty and charge one attempt | Allowed |
| Pending | DELIVERED | Pending | Keep dirty and charge one attempt | Allowed |
| Pending | RETURNED | Pending | Keep dirty and charge one attempt | Allowed |
| Pending | Missing | Pending | Keep dirty and charge one attempt | Allowed |
| Committed | ESCROWED | Committed | Keep dirty and charge one attempt | Allowed |
| Committed | DELIVERED | Committed | Keep dirty and charge one attempt | Allowed |
| Committed | RETURNED | Committed | Keep dirty and charge one attempt | Allowed |
| Committed | Missing | Committed | Keep dirty and charge one attempt | Allowed |
| Audited | ESCROWED | Drift | Keep report count at one and fence certification | Quarantined |
| Audited | DELIVERED | Audited | Clear dirty key and enable certification | Allowed |
| Audited | RETURNED | Drift | Keep report count at one and fence certification | Quarantined |
| Audited | Missing | Audited | Keep dirty and charge one attempt | Allowed |
| Rejected | ESCROWED | Rejected | Clear dirty key and enable certification | Allowed |
| Rejected | DELIVERED | Drift | Keep report count at one and fence certification | Quarantined |
| Rejected | RETURNED | Rejected | Keep dirty and charge one attempt | Allowed |
| Rejected | Missing | Rejected | Keep dirty and charge one attempt | Allowed |
| Refunded | ESCROWED | Drift | Keep report count at one and fence certification | Quarantined |
| Refunded | DELIVERED | Drift | Keep report count at one and fence certification | Quarantined |
| Refunded | RETURNED | Refunded | Clear dirty key and enable certification | Allowed |
| Refunded | Missing | Refunded | Keep dirty and charge one attempt | Allowed |
| Drift | ESCROWED | Drift | Keep report count at one and fence certification | Quarantined |
| Drift | DELIVERED | Drift | Keep report count at one and fence certification | Quarantined |
| Drift | RETURNED | Drift | Keep report count at one and fence certification | Quarantined |
| Drift | Missing | Drift | Keep report count at one and fence certification | Quarantined |

### Recovery and cross-record transitions

These rows seed the linked records needed to exercise a complete lifecycle
path. A row can include a second input to prove that a later replay or retry
preserves the first outcome. The PostgreSQL acceptance test runs all 171
database rows in this document. The SQL guard test runs the final row.

| Current | Event | Next | Funds | Custody | Work and report | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| Absent | Create | Draft | None | Stage deposit | Publish one escrow intent | Allowed |
| Draft, no Game effect | Lock pending | Draft | None | Await Lock | Preserve operation | Allowed |
| Draft, Lock pending | Audited Lock rejection | Cancelled or awaiting release of surviving Lock | None | Close only with verified no effect or unlock | Record refusal | Allowed |
| Draft, Escrow pending | Audited Escrow rejection | Cancelled | None | No deposit effect | Record refusal | Allowed |
| Draft, audited Escrow | Advance | Active | None | Escrow | Preserve proof | Allowed |
| Draft, audited Escrow | Cancel requested | Cancelled after audited Return | None | Returned | Dispatch one return | Allowed |
| Active, no bid | Expiry | Cancelled after audited Return | None | Returned | Dispatch one return | Allowed |
| Active, moderation hold | Expiry | Active | Preserve | Escrow | Preserve hold with no Return dispatch | Allowed to defer |
| Active, held funds | Expiry | AwaitingDelivery | Held | Escrow | Bind one sale | Allowed |
| Active, moderation hold | Approved invalidation | Cancelled | Release if reserved | Returned | Resolve hold after return | Allowed |
| Matched, legacy hold | Retry tick | AwaitingDelivery | Preserve reservation | Escrow | Retire hold once and resume settlement | Allowed |
| AwaitingDelivery, dispatched Deliver | Audited commitment and Delivered status | Completed | Settle once | Delivered | One completed sale | Allowed |
| AwaitingDelivery, dispatched Deliver | Audited rejection and audited Returned status | Cancelled | Refund once | Returned | Record compensation | Allowed |
| AwaitingDelivery, canonical and alternate staged | Second connection publishes alternate, then canonical publishes | AwaitingDelivery with canonical dispatched | Held | Escrow | Exactly one dispatch | Alternate impossible because its key cannot authorize publication |
| AwaitingDelivery, receipt without audit | BZ-08 retry bound | Drift, listing unchanged | Held | Quarantine | Queue survives eight retries. Report once and keep custody fenced after late audit | Quarantined |
| AwaitingDelivery, Return dispatched after audited Deliver rejection | Return receipt remains missing through retry bound | Drift, listing unchanged | Held | Quarantine | Report once. Deliver rejection does not clear unresolved Return work | Quarantined |
| AwaitingDelivery, Return dispatched before Deliver | Return receipt remains missing through retry bound | Drift, listing unchanged | Held | Quarantine | Report once even without a Deliver operation | Quarantined |
| AwaitingDelivery, Return dispatched after audited Deliver rejection | Audited Return rejection | Drift, listing unchanged | Held | Quarantine | Report once. Preserve return fence and require operator reconciliation | Quarantined |
| AwaitingDelivery, unpublished alternate | BZ-11 buyer becomes ineligible | Cancelled after audited Return | Refund once | Returned | Ignore staged alternate. Independently evidenced handoff still fences return | Allowed |
| Completed, old evidence | Receipt before audit | Completed | Settled | Preserve | Wait for audit, then resolve dirty key and certify | Allowed |
| Completed, old evidence | STATUS ESCROWED contradicts delivered custody | Drift, listing unchanged | Settled | Quarantine | Report once | Quarantined |
| Completed, old evidence | Retry bound | Drift, listing unchanged | Settled | Quarantine | Report once even after receipt replay | Quarantined |
| Completed, current evidence | Worker tick | Completed | Preserve settlement | Delivered | No repeat handoff | Allowed |
| Completed, certified | Contradictory status changes after certification | Drift, listing unchanged | Preserve settlement | Quarantine | Recheck and revoke certification | Quarantined |
| Completed, behind cursor | Status producer commits between pages | Drift, listing unchanged | Preserve settlement | Quarantine | Committed dirty key forces recheck behind cursor | Quarantined |
| Completed, obsolete marker | BZ-10 same-fence version or world-set change and rebuild failure | Rebuilding or fenced | Preserve settlement | Preserve | Obsolete marker cannot serve with fresh watermark | Allowed rebuild failure |
| Cancelled, refunded | BZ-09 ESCROWED status contradicts Return | Drift, listing unchanged | Preserve refund | Quarantine | Report once and revoke certification | Quarantined |
| Cancelled, refunded | Reversal of rejected Deliver | Drift, listing unchanged | Preserve refund | Quarantine | Report the conflicting commitment once | Quarantined |
| Rebuild, old epoch | Rebuild start | Current after scan | Preserve | Preserve | Old coordinator cannot certify. The correct epoch can finish | Allowed |
| Rebuild, world absent | Configured world missing | Fenced until corrected | Preserve | Preserve | No sale retry charged for configuration gap | Allowed to defer |
| Rebuild, early missing STATUS | BZ-12 page checkpoint before contradictory legacy sale | Later sale in Drift | Preserve | Quarantine affected sale | Advance past missing listing and report the later sale once without new intake | Allowed enumeration with later sale quarantined |
| Rebuild, completed legacy item with no authority asset row | Page finds contradictory Deliver evidence | Drift, listing unchanged | Preserve settlement | Quarantine | Enumerate by the retained sale and command, then report once | Quarantined |
| Rebuild, early Drift | Later page | Both contradictions in Drift | Preserve | Quarantine affected sales | Report both once and continue past early Drift | Quarantined |
| Rebuild, page in flight | Writer connection closes before commit | Last committed cursor, then replayed page | Preserve | Preserve | Observer cannot read abandoned checkpoint. Retry commits successor | Allowed |
| AwaitingDelivery, Return audited | Second worker races to compensate | Cancelled | Refund once | Returned once | One compensation and one Return dispatch | Allowed |
| SQL guard, production macro definition | BZ-13 nested concat!/include_str! assembles a direct state write | Rejected by the guard | Preserve | Preserve | Report the assembled production SQL | Impossible to bypass the lifecycle writer guard |

## Invariants

- A sale can release value only along its proven outcome. Buyer delivery may
  settle the seller's funds. Seller return may refund the buyer. Delivery and
  refund, or return and seller settlement, cannot both be authorized.
- Each item sale has exactly one canonical Deliver identity derived from its
  immutable sale key. At most one outbox operation may use that key. Before
  issuance a sale can be safely returned with no published Deliver. Alternate
  staged records grant no authority and any independent commitment must be checked.
- Certification requires current audited evidence and matching rebuild version,
  worlds and fence. A fresh watermark alone grants no custody authority.
- A published Deliver or Return with unresolved sale evidence shares one
  budget of eight durable waiting attempts for the sale.
  On the next unresolved check it goes to Drift and is reported once. Scanning
  other listings continues while certification stays fenced.
- Checkpoints, dirty-key changes and evidence applied by a page commit together.
  World-policy gaps leave financial data and the retry budget untouched.
