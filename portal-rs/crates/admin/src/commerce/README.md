# Staff commerce

`router` composes the T10b staff host/session/CSRF/MFA guard with the commerce
service. `LedgerEffects` calls the public T26 `adjust_in` and T27 `mutate_in`
APIs in the same transaction as the staff operation and audit. The caller supplies
transactional account authority and wallet audit adapters. Unknown authority
must deny. Account status and permission checks are repeated for approvals;
role writers and commerce share T10's advisory lock.

All flags default off. The environment-mounted route additionally requires
`OTERYN_ADMIN_COMMERCE=true`; the domain switches are
`OTERYN_PREMIUM_ENABLED`, `OTERYN_WALLET_ENABLED`, `OTERYN_PAYMENTS_ENABLED`,
and `OTERYN_REFUNDS_ENABLED`. Exact `true` is the only enabled value.
The configured route retains T10b's `PendingAuthentication` and a denying
`PendingEffects` until the application supplies verified staff/domain authority.
Use the injected `router` for that composition; enabling flags alone grants no
identity or monetary authority.

The staff page is `/admin/commerce`. Grants accept 1–366 days, adjustments a
nonzero integer within ±1,000,000,000, and reasons 10–500 trimmed characters.
Targets and request keys are RFC 4122 UUIDv7 identifiers. Reasons are persisted
only as SHA-256 hashes. Idempotency binds actor, target, command and reason hash.
Reusing a key with another payload refuses the request. No approved correction
threshold exists, so **every correction** requires a different staff member.
Refunds always require a different staff member. The approval queue presents
the immutable target and requested amount. Approval never trusts submitted
amounts or actor identifiers.

`PaymentActions` is the narrow T31 integration seam. The included
`TestPaymentAdapter` is exclusively for deterministic non-production use with
T31's `TestProvider`. Reconciliation records
`reviewed_no_payment_state_change`, matching Laravel's resolution contract;
it does not manufacture payment settlement. Only full refunds are supported.
Approved refunds atomically persist an intent-scoped unique refund and enqueue
`commerce.refund`. `RefundDelivery` validates the outbox payload and rechecks the
intent's account, currency, amount and eligible state before calling the
provider with its stable per-intent refund identity. Its row lock serializes
workers; a crash can retry the provider's idempotent request. The binary exposes
`Registry::commerce_refunds` for explicit registration with the selected
adapter. No provider is installed by default. The dispatch receipt is separate
from settlement: only T31's verified webhook may mark a payment refunded. This
module never converts money to coins or fabricates a wallet reversal policy.

Database tests use `PORTAL_TEST_DATABASE_URL`, create isolated schemas and print
a skip notice when the variable is absent. They cover real ledger effects,
idempotency, four eyes, concurrency, changed balance, revoked permissions,
unknown targets, refunds without fabricated settlement, localized SSR and CSRF.
