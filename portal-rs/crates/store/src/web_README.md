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
provided for deployment. The configured composition keeps purchase and transfer
flags false until those authoritative adapters are supplied. Missing adapters
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
