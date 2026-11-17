# Shop web composition

T32 mounts `/{pl,en}/shop`, `/shop/coins`, `/shop/history` and
`/shop/transfer`. Every response is private (`no-store`), and every mutation
uses the existing browser session and synchronizer CSRF middleware. Acting
account IDs come exclusively from the session. Amounts are positive integer
smallest units; form bodies reject unknown and duplicate fields. Wallet history
uses a descending UUID cursor and 25 entries per page.

All flags default to false. `OTERYN_SHOP_ENABLED=true` enables authenticated
catalog, balance and history reads. `OTERYN_PUBLIC_ORIGIN` supplies the exact
trusted origin for browser mutations. Missing/unknown/inactive/unverified
accounts deny access.

`router(pool, flags, backend, origins)` is the explicit composition API for
mutations. `Domain` delegates transfers and reads to `wallet::Ledger`, catalog
reads to `CatalogStore`, and purchases to T29 `Purchases` through `Buy`. The
`Access` adapter must authorize current account state and authoritative character
and world ownership; a character projection alone does not prove ownership.
Wallet adapters must check both parties' restrictions and D14 cooldowns in the
ledger transaction and persist purchase audit evidence. No permissive adapter is
provided for deployment. The configured composition keeps purchases disabled
until authoritative ownership is supplied. Transfers use the identity adapter
described below. Missing adapters
never produce a success response.

Payments already depends on store. To avoid a dependency cycle, the binary
registers `web_payments.rs` as its composition module; the store library itself
knows only `TopUps`. Test payment creation additionally requires:

- `OTERYN_ENV=testing`;
- `OTERYN_SHOP_TEST_PAYMENTS_ENABLED=true`;
- `OTERYN_SHOP_TOP_UPS_ENABLED=true`;
- `OTERYN_SHOP_TOP_UP_PACKAGES`, a nonempty JSON array of objects with `key`,
  `coins`, `minor_units` and uppercase three-letter `currency`.

Package rates are supplied by configuration, never inferred from browser input.
The adapter uses T31's local test provider only. It does not register a real
provider, process real payments or credit a wallet. The stored package binding
preserves the chosen conversion policy for a future verified-settlement consumer.
T31's settlement outbox remains the authority for eventual coin credit; a browser
response, redirect or pending intent never grants coins.

`store_web_topups` claims the UUIDv7 operation before invoking T31. Exact replay
returns the original payment intent, and changed bindings return a conflict. An
interrupted creation leaves a claim with no intent ID and returns unavailable on
retry; reconciliation must resolve it before any further attempt. Do not clear
such claims automatically, because provider state may be unknown. The migration
adds only this table and index and preserves existing financial records. Rollback
means disabling the feature and retaining its history.

Run PostgreSQL checks with `PORTAL_TEST_DATABASE_URL` set. Unset database URLs
print a skip notice. The optional ignored browser test also needs Chromium,
`PORTAL_BROWSER_NODE` (a native Node executable), and `PORTAL_BROWSER_MODULES`
(a package directory with playwright-core and @axe-core/playwright). It checks
eight authenticated Polish/English pages at 360 px, keyboard navigation, a real
ledger transfer, a refusal, and WCAG 2.2 AA rules with axe. No SQL query macros
were added, so no offline query metadata is needed.

## Dummy bank checkout (SHOP-TEST)

For a local testing environment, set these values in the process environment:

```sh
OTERYN_ENV=testing
OTERYN_PAYMENTS_ENABLED=true
OTERYN_WALLET_ENABLED=true
OTERYN_SHOP_ENABLED=true
OTERYN_SHOP_TOP_UPS_ENABLED=true
OTERYN_SHOP_TEST_PAYMENTS_ENABLED=true
# Supply OTERYN_SHOP_TEST_PAYMENT_KEY externally (at least 32 bytes).
OTERYN_SHOP_TOP_UP_PACKAGES='[{"key":"coins-100","coins":100,"minor_units":250,"currency":"PLN"}]'
OTERYN_SHOP_PURCHASES_ENABLED=false
OTERYN_SHOP_TRANSFERS_ENABLED=true
```

Also configure `OTERYN_PUBLIC_ORIGIN` to the browser's exact origin. The signing
key is required: there is no generated fallback. Start the existing worker with
the same payment and wallet flags to deliver `wallet.top_up` jobs.

Top-up creation redirects to `/{lang}/shop/test-checkout/{intent_id}`. The bank
shows the stored package, localized currency amounts and an explicit test-mode banner.
Pay sends a signed success event through `Payments::webhook`; reject and cancel
send failure. Session authorization and CSRF apply to all browser mutations.
Foreign intents return 404. Finished intents show their persisted status and
never submit another event. Repeated decisions have stable event IDs; competing
terminal transitions are serialized by the payment intent lock. A success message
promises eventual credit, never immediate balance delivery. Receipts, invoice and the outbox use the normal payment path.
The checkout route is absent outside the testing gate. Transfers also require
that gate and `OTERYN_SHOP_TRANSFERS_ENABLED=true`; all defaults remain off.

The configured transfer policy now reads both accounts' active/verified state
and D14 `email_change_blocked_until` through identity's `trade_allowed` using the
ledger transaction connection and shared identity locks. Unknown state denies.

**Purchase activation remains blocked.** `OTERYN_SHOP_PURCHASES_ENABLED` is
reserved and cannot activate purchases yet. The portal's `account_characters`,
`account_character_snapshots` and `account_character_feed` tables are projections;
there is no authoritative current character/world ownership table. Per SHOP-TEST,
no ownership source is invented and no projection is treated as proof. The
configured purchase adapter and its purchase audit integration must wait for
that authoritative contract. Existing injected-domain purchase tests cover
success, refusal and bad input, but do not qualify configured purchase activation.

The ignored binary test `shop_web::tests::postgres_dummy_checkout_browser_360px`
uses the real local provider, signed settlement, outbox worker and configured
transfer policy. With the database URL and browser variables above set, run:

```sh
cargo test --locked -p oteryn-portal postgres_dummy_checkout_browser_360px -- --ignored --nocapture
```

It checks both checkout languages at 360 px with keyboard payment submission and
axe WCAG 2.2 AA rules, then waits for worker credit and transfers coins. The buy
step remains blocked by the ownership dependency described above.

Gift codes use the default-off `OTERYN_STORE_CODES_ENABLED` flag. The portal
mounts `POST /{lang}/admin/store/codes` on the configured admin host with the
existing session, CSRF, staff MFA and `admin.access` guards. Issuance also needs
`store.codes.generate`, rechecked under the RBAC writer fence in the issuance
transaction. The additive migration grants this permission to built-in
`platform_admin` roles. The guarded GET form generates a fresh UUIDv7 `request_id`. Form and JSON submissions require it.
The other issuance fields are `kind` (`coins` or `premium_days`),
`amount`, `count`, `max_per_code`, `max_per_account`, `valid_from`, `valid_until`
(Unix seconds), and `note`. The response returns the batch ID and the plaintext
codes once. A matching retry returns the same batch ID and an empty code list. Reuse by
another actor or with changed terms returns a conflict. Issuance is audited once without plaintext.

`POST /{lang}/shop/codes/redeem` accepts JSON `code` and UUIDv7 `operation`, with
the browser session and CSRF header. The acting account comes from the session
and must currently be active and verified. Its durable account budget is five
attempts per minute, including malformed authenticated requests. Coin redemption
also requires `OTERYN_WALLET_ENABLED`; credits and debt settlement use the wallet
transaction. Retries retain the operation ID and never grant twice.

Item codes are refused by both production adapters. The existing batch schema
has no item/product binding and no qualified game consumer; exposing queued
item grants would promise delivery that the portal cannot perform. Existing
item batches cannot be redeemed through this surface either.
