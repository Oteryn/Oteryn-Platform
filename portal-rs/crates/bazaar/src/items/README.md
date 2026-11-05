# Item listings (T60)

The module implements fixed-price, negotiated and wanted notices, per-unit stack
pricing, reserved offers, acceptance/rejection, proxy auctions, buy-now before the
first bid, five-minute extensions, cancellation, renewal and expiry. It uses T57's
public types, proxy calculation and transaction-local account/funding ports.
Amounts sent to the reservation adapter are full stack totals. Coin commission
is frozen at 10% in immutable sale evidence. No seller credit occurs in T60.

The process mounts only `configured_router`, which returns localized 503 pages.
`ItemFlags` defaults to disabled. The composable `router` accepts a qualified
service, database and explicit CSRF trusted origins; it uses server sessions and
shared marketplace account/source rate limits. Authenticated actor IDs never
come from browser forms. Owner-only views exclude proxy maxima. Public reads
check fresh custody/privacy evidence and render only the approved projection.
Templates use the existing design tokens/components and Fluent Polish/English.

## Dependencies and deny decisions

T57's code is already present in the contract base. The T57 and T58 remote
branches did not exist at task admission. T14b was merged before implementation.
T58 must implement `ItemAuthority` using verified durable evidence in the supplied
transaction, with locking, current world policy, exact definition/quantity and
attribute conservation, escrow receipt binding, watermark/epoch/recovery and
privacy floors. `Unavailable` denies every authority check. Its publication
result is a local approved projection, never a proposed alternate receipt.
A wanted-notice seller response uses the request UUID as its stable offer ID;
T58 must prepare/bind that exact seller deposit before `offer_ready` can succeed.

G-7 revision 2 leaves capability advertisement, CHAR_CMD result-data mapping,
destination custody, production freshness and retention acceptance unresolved.
No game route, new receipt decoder, wire body or capability inference is added.
Outbox rows are local immutable saga references with the contract issuance-key
shape. They remain `blocked` in a dedicated table outside the shared worker's
claimable queue. T58 must construct accepted canonical intents and prove the
funding/disposition fences before dispatch. Timeout, disabled flags or stale
evidence never authorize an asset move, payout or release of a matched sale.
Matched sales cannot be cancelled or expired here. Auctions freeze one immutable
sale and retain the complete maximum pending T58 reconciliation.

Gold is denied rather than routed through T57's older gold adapter: that adapter
rounds its fee down, while G-7 revision 2 proposes ceiling rounding and protected
bank headroom. Item creation currently accepts only coin listings. A negotiated
notice without a price uses T57's internal positive floor, which is never rendered
as a quoted price. Renewal is restricted to active notices without buyer activity;
expired custody needs T58 return/relisting evidence and cannot be reused blindly.
The Laravel implementation has character auctions only; it supplies the browser
validation/authorization/redirect conventions, not an item or gold wire contract.

## Verification

Tests in `tests.rs` use synthetic data and isolated PostgreSQL schemas. They read
`PORTAL_TEST_DATABASE_URL` and print a skip notice if absent. Tests exercise real
SQL transactions, funding rollback, ownership denial, stale authority, idempotent
creation/bids/offers/buy-now, concurrent acceptance, deposit-before-publication,
stack totals, wanted roles, sniping, cancellation/expiry, immutable blocked outbox
and an authenticated session/CSRF form path. No SQL query macros were added.

### Candidate validation (2026-11-05)

On pinned Rust 1.97.0, with `SQLX_OFFLINE=true` and the local PostgreSQL 18.6 URL
exported as `PORTAL_TEST_DATABASE_URL`:

- `cargo fmt --all --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo test --locked`: failed in the unchanged game-auth real-listener test
  `admission::tests::db_gateway_client_calls_real_admission_listener`.
- Diagnostic `cargo test --locked --no-fail-fast`: 341 passed, 14 failed,
  1 ignored. All 14 failures are in the unchanged gateway/game-auth targets and
  report Rustls 0.23.46 being unable to select a process CryptoProvider with both
  `ring` and `aws-lc-rs` enabled. Neither crate depends on the item module; both
  source trees and the Rustls version are unchanged from the contract base.
- T60: 14 passed, including 11 PostgreSQL tests. No T60 DB tests skipped.
- Chromium at 360px: no horizontal overflow, labeled controls, 44px buttons and
  working keyboard focus; existing design-system contrast tests passed.
- `python3 tools/validation/legacy_token_ratchet.py`: ratchet ok.

Workspace acceptance remains blocked by that provider-selection defect. Fixing
it requires a gateway/game-auth change outside T60's allowed paths. No test was
skipped or weakened to bypass the failure. G-7/T58 rollout blockers above remain
open independently of this workspace defect.
