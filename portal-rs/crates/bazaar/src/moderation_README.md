# Bazaar moderation (T64)

Authority: Rust portal plan §5.6, D13, §7 and T64; proposed G-7 revision 2
§§1, 5, 7–9. The existing Laravel marketplace supplies the disabled-route,
server authorization, bounded reasons and transactional audit reference. It has
no seller-profile, item-policy or two-person invalidation implementation to copy.

## Integration boundaries

T59 and T60 had no remote branches at task admission. T10b and T33c were merged.
No Game repository files or fixtures were copied. This module performs no Game
HTTP calls, constructs no Game wire commands and consumes no substitute receipt
schema. Existing CHAR_CMD receipts remain authoritative for future compensation.

`PublicViews` must return only current, approved public seller facts, including
the seller's selected Character and join date. Its adapter must prove current
ownership, publication consent, account privacy, projection epoch, recovery
fences and freshness. Hidden, missing or conflicting facts deny. The service
filters active listings against local ownership, time and moderation holds,
computes completed-sale counts from committed Platform settlement records, and
reads featuring from committed promotion records. No private proxy maximum is
part of the public view.

`Accounts` must supply current account restrictions, verification and MFA/sudo
state. `PromotionLedger` must debit transferable coins and append the Wallet
ledger using only the supplied transaction and promotion ID. Never commit the
transaction or perform external I/O inside an adapter. Featuring binds the
seller's displayed quote to an immutable fee revision; replay cannot debit twice.

Configured public adapters use `PendingIntegration`, and the configured staff
surface uses the panel's `PendingAuthentication`. Both deny authority. Tests
inject deterministic adapters to exercise the complete service and staff HTTP
paths. Replacing adapters requires integrating the owning dependencies, not
trusting request headers or form-supplied actor identities.

## Flags and compensation

`OTERYN_BAZAAR_MODERATION`, `OTERYN_BAZAAR_PROFILES` and
`OTERYN_BAZAAR_PROMOTIONS` require the exact value `true`; all default off.
The first gates local moderation/reporting; the others independently gate
public profiles and paid featuring. Game dispatch has no implementation here.
The immutable moderation outbox can be populated while Game command flags are
off, but is never registered with a dispatcher.

Suspension retains custody, reservations and proceeds. Database triggers fence
existing trading writes, including offer acceptance and settlement, without
changing the T57 API. Invalidation approval requires a different currently
permitted, MFA-verified staff member and an unchanged hold revision. It records
only `reconcile_invalidation`; it never cancels the core state, refunds money,
unlocks custody or reverses a committed transfer. Pending/started settlement
states refuse invalidation. A future T58 worker must prove safe compensation
using current-generation authoritative receipts before doing anything further.

## Ambiguities resolved by denial

- G-7 supplies no post-sale reversal contract. Such invalidation is refused;
  approval is a durable reconciliation request, never a successful reversal.
- There is no admitted catalog/instance binding in this base. A new restriction
  conservatively holds every active item listing and fences future item
  activation/trading. The panel explicitly identifies its list as local
  restrictions, not an authoritative transferable-item catalog. `item_allowed`
  can only narrow a caller's fresh admitted Game decision; unknown denies.
- D13 fixes free listing, 1000-bps coin commission and 250-bps gold commission.
  Staff cannot override these economics. The configurable positive featuring
  price does not enable featuring. Gold rounding and payout stay with T58 and
  the accepted Game contract; this module does not change settlement arithmetic.
- There is no accepted hold-release/resumption proof here. No endpoint removes
  a hold, and a changed hold revision cannot reuse an older approval.
