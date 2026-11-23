# Bazaar production qualification (GAP-43 / G-7)

Production qualification is pending. The [G-7 proposal](../contracts/OTERYN_V2_BAZAAR_CONTRACT_PROPOSAL.md) records the wire and custody decisions awaiting ratification. The Platform simulator exercises a candidate exchange on a disposable database. Its results cover the Platform consumer and simulated effects.

## Platform views and rollout

- Players can open the localized readiness page from Bazaar or My listings. It explains the production status for character custody, item custody, gold reservations, transfers and settlement. The trading pages use their existing unavailable responses when the runtime is absent.
- Staff can open Bazaar readiness from the administration panel with staff MFA, `admin.access` and `marketplace.manage`. The runtime indicator describes the local web process. A listing-ID lookup shows persisted Platform lifecycle, moderation and settlement records. These are observations of the Platform database. Current Game custody and balances require the Game authority.
- `OTERYN_BAZAAR_ENABLED` defaults off. The candidate runtime is limited to `OTERYN_ENV=testing` or `preprod`. This applies to the web composition, private listener and worker through the shared runtime configuration. Readiness pages have no trading actions.

## Evidence for the joint qualification

| Scenario | Existing Platform evidence | Required joint result |
| --- | --- | --- |
| Signed character listing and item escrow | [Candidate browser flow](../../portal-rs/e2e/specs/bazaar.spec.mjs) and [consumer tests](../../portal-rs/crates/bazaar/src/game/authority_tests.rs) | The Game commits the lock or exact item move, Platform checks the signed facts and publishes once. Missing or refused custody leaves the listing unpublished. |
| Funding, handoff, fees and release | [Coin and gold browser flows](../../portal-rs/e2e/specs/bazaar.spec.mjs) and [outbid and return flows](../../portal-rs/e2e/specs/game-sim-bazaar.spec.mjs) | Reject invalid bids and concurrent duplicate reservations. A funded winning bid transfers once, settles once, applies the ratified fee and releases the right funds. Browser, ledger, custody and receipt views agree after exact replay. |
| Receipt binding, recovery and ambiguous effects | [Consumer tests](../../portal-rs/crates/bazaar/src/game/authority_tests.rs) and [G-7 acceptance checklist](../contracts/OTERYN_V2_BAZAAR_CONTRACT_PROPOSAL.md#11-acceptance-checklist-all-unchecked) | Reject foreign and stale receipts. Exercise conflicting and out-of-order evidence, interruption around commit and restore under a new generation. Reconcile from current evidence and preserve unresolved locks, escrow and holds. |

The coordinator and owners must record the accepted contract revision, exact Platform and Game candidate SHAs, shared fixture digests, producer capabilities, trust and restore scope, and the observed result of each scenario. Qualification starts in testing or preproduction. Production approval is a separate decision on that evidence.

## Remaining Game work

1. Ratify the character-authority version extension and receipt mappings. Advertise the exact decoder capabilities, settle the open G-7 decisions and allocate the escrow identity.
2. Prove atomic character lock and signed snapshot capture, transfer interlocks and unlock ordering. Prove item identity and attribute conservation on escrow, delivery and return, including offline or full destinations.
3. Prove Account+World gold holds against concurrent bank activity, replacement and release, seller headroom, fees and once-only payout.
4. Produce durable effect-bound receipts and policy, custody and completeness feeds. Qualify mTLS identities, signing trust and rotation, replay and restore fencing against the exact Platform candidate.

## Operator recovery

Keep new trading disabled while production qualification is pending. During a rehearsal, record outstanding listing, command, escrow and hold references before suspending ingress and workers. Preserve durable receipts, money records and replay fences. The staff view can identify the stored listing and settlement state for follow-up. An absent record or overdue response leaves the Game disposition unresolved. Reopen only after the owners reconcile each outstanding obligation under the current recovery generation and approve the selected candidate.
