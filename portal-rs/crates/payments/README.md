# Payments core

`Payments` is a Rust API with no HTTP routes or UI. Construct it with a database
pool, a `PaymentProvider`, an authoritative `AccountReader`, and `PaymentFlags`.
The default flag disables the API. Account lookup errors, unknown, ambiguous,
and restricted states deny access. Account IDs have no foreign key until the
accounts contract exists. Callers own authentication and authorization of this
internal API; account/auth crates must not depend on payments.

`create_intent` accepts positive integer minor units and a three-letter uppercase
currency. It returns a pending intent. The provider receives the UUIDv7 intent ID
as its stable creation key. A real provider adapter will need durable request
recovery/reconciliation before production use; only the deterministic local
provider is implemented here (decision D3).

For `TestProvider`, webhook signatures are raw 32-byte HMAC-SHA256 over the exact
JSON bytes. Supply a secret of at least 32 bytes from outside the repository.
`PaymentEvent` binds an event ID, intent ID, provider reference, amount, currency,
and target state. Payloads are limited to 64 KiB. Provider event IDs are unique
within a provider and have no repeated effects. Reusing an ID for a different
event is a provider contract violation. Signatures are verified before parsing.

Transitions: created → pending/succeeded/failed; pending → succeeded/failed;
succeeded → refunded. Terminal failed/refunded states cannot be reopened.
A successful event writes its receipt, minimal invoice, new state and
`wallet.top_up` job in one transaction. The intent row lock serializes concurrent
events. The outbox key is the intent ID. `TopUpJob` documents the payload: intent
ID, account ID, amount and currency. Amount is the purchased minor-unit quantity;
the future wallet handler must define conversion and deduplicate by intent ID.
No wallet call or handler registration is included.

The provider refund operation has a deterministic refund key. A signed refund
event records the refunded state; this task does not implement staff refund
authorization, financial reconciliation, or a wallet reversal handler. These
must be wired before enabling real payments, along with the selected provider.
Invoices are minimal success records, not tax-document generation.

Tests use isolated PostgreSQL schemas and the embedded workspace migrations.
Set `PORTAL_TEST_DATABASE_URL` to a disposable PostgreSQL 18.6 database whose user
can create schemas. Nine database tests run when set; otherwise each prints a
skip notice. Three independent tests cover transitions, provider behavior and an
RFC 4231 HMAC vector. No SQL query macros are used; builds support SQLX_OFFLINE.

## Native coin purchases (sandbox, phase A)

`coin`, `coin_store` and `coin_http` implement Game contract
OTERYN_COIN_PURCHASE_V1 under `/api/v1/native-account/coin-purchases`
(`catalogue`, `balance`, `orders`, `requests/{request_id}`) and the hosted
checkout page `/native-account/coin-checkout/{order_id}`. Off by default: every
route answers 503 unless `PORTAL_COIN_PURCHASES_ENABLED=true`, `OTERYN_ENV` is
`testing` or `preprod` (never production) and `OTERYN_PUBLIC_ORIGIN` is valid.
`PORTAL_COIN_PACKAGES` is a JSON package list; unset means no packages and a
malformed value keeps the feature off. Only `game:account-services` access
tokens name the payer; `game:ticket` tokens are refused, and the reverse holds.

An order consumes its single-use quote in the transaction that rechecks the
quote's expiry, owner, character, environment and the payer's eligibility.
Provider I/O happens after commit. An identical replay returns the current
order; a changed body for an existing `request_id` is 409. Only a verified
`TestProvider` webhook (signed in-process by the checkout page) marks an order
paid; in one transaction it writes the invoice receipt, credits the
payer's coin balance through the host's wallet ledger keyed by the intent ID (once), and bumps
the order revision. States never move backwards and `sandbox` is always true.
A payer no longer eligible when the verified payment arrives gets no coins:
the order is held in `processing` with `review_required` and code
`COIN_ORDER_REVIEW_REQUIRED` for manual handling. Every delivery, replays
included, must match the bound intent's amount, currency, provider reference
and payer; a replayed event ID with another payload is refused and changes
nothing. A wrong-scope token is 401, as on the existing game OAuth routes. A
signed-out checkout visit stores the exact checkout path as the post-login
return target (`oauth_return`), which the login hook accepts only in that form.
The generic `Payments::webhook` refuses coin intents.

## Native coin purchases: reversals and restrictions (phase B)

Under the same flag, a verified `refunded` or `chargeback` event (`CoinEvent`:
the `PaymentEvent` shape whose state may also be `chargeback`) reverses a
credited order once. In the event's transaction, with the intent, order and
wallet rows locked, the whole credit is debited from the unreserved coin
balance (one coin class: coins of any origin, top-up, reward, staff grant or
referral, count) by a wallet adjustment keyed by the reversal ID (reason
`coin-purchase-reversal:<id>`), whatever the payer's restrictions. One
`coin_purchase_reversals` row per order (unique) and the unique provider event
ID make redeliveries, restarts and concurrent deliveries debit at most once; a
chargeback after a refund changes only the state. When the unreserved
coin balance no longer covers the credit (spent or reserved), nothing is
debited (never partially, never below zero): the order is flagged
`review_required` with code `COIN_ORDER_REVERSAL_REVIEW_REQUIRED` and the payer
gets a `coin_purchase_holds` row that freezes new purchasing until staff
resolve it (no release path exists yet). A reversal before any credit holds the
order in `processing` for review and fences a later payment, which is retained
and credits nothing; a failed order cannot be reversed.

A payer is refused (409, audited as `coin_purchase.payment_refused`) at order
creation, checkout and sandbox payment, before any provider call or credit,
when the account is not active, unverified, under the email-change lock, has a
pending termination, an effective restriction or suspension sanction, or a coin
purchase hold. A verified payment for such a payer is held without coins
(`coin_purchase.payment_held`). A changed body for an existing `request_id` is
409 and audited (`coin_purchase.request_replay_refused`); a refused provider
event (changed payload for a known event ID, or a binding mismatch) is audited
after rollback (`coin_purchase.event_replay_refused` / `coin_purchase.event_refused`).
Partial refunds need a new contract. No HTTP route delivers provider events.
