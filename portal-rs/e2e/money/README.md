# Money and Game boundary concurrency

Use the disposable PostgreSQL, Chromium, Rust toolchain and test-provider setup
from [the E2E README](../README.md). Start the existing browser launcher from
`portal-rs/e2e` with its usual database environment and the Store wire contract
enabled. These settings select the test Game identity and allow a 24-request
native purchase burst. Browser quotas retain their normal limits.

```sh
PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY=store.money.e2e.oteryn.test \
PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY=oteryn-game/store-consumer \
PRODUCTS_ENTITLEMENTS_STORE_ENABLED_UNIT_KINDS='["ITEM"]' \
PRODUCTS_ENTITLEMENTS_STORE_PURCHASES_PER_ACCOUNT_PER_MINUTE=60 \
CARGO_BUILD_JOBS=2 node local.mjs
```

In a second shell, use the harness environment file. If the launcher built in
an external target directory, export the same `CARGO_TARGET_DIR` or provide
`PORTAL_E2E_BINARY` so the tests can invoke the seed and catalogue commands.

```sh
node run.mjs /absolute/scratch/env.json test --config money/playwright.config.mjs --retries=0
```

The suite uses the existing Bazaar producer and the Store wire contract
fixtures. Its Store client uses the local harness CA and sends receipts over
the real internal mTLS listener. Tests lock disposable database rows to
reproduce timeouts and control fixture deadlines and credit scheduling to
exercise the real worker. Keep the launcher environment, keys and Playwright
artifacts local.
