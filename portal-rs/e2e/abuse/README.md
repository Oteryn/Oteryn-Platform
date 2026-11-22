# Non-authentication abuse budgets

Issue [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002). The spec runs in Polish and English at 1440 × 900 and 390 × 900.

From `portal-rs`, build the portal and install the pinned browser test dependencies. Supply `PORTAL_E2E_ADMIN_DATABASE_URL` for a disposable local PostgreSQL 18.6 instance using the procedure in [the harness README](../README.md). Node must support `process.execve` (22.15 or later). The runner uses the installed Chromium and needs the standard harness ports plus loopback HTTPS on 18443 and STARTTLS on 587.

```sh
CARGO_BUILD_JOBS=2 cargo build --locked -p oteryn-portal
cd e2e
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts
node abuse/run.mjs
```

The runner uses `harness.mjs` to migrate and start the portal and selects the abuse spec with the shared Playwright project configuration. The seed CLI creates verified browser actors with generated credentials. Reset checks age only the saturated actor's rows in PostgreSQL and require the next real request to pass the limit. The party fixture seeds persisted ownership and public profile inputs, following `crates/community/src/party/party_tests.rs:672`. The production ownership and profile readers still run. Fixed-window checks use real proxy-to-portal connections from two loopback addresses and verify that the same account receives separate source buckets. SMTP terminates at the local sink. The referral assertions inspect the committed queue. Traces and any retained private harness logs belong on the local machine.

| Surface | Configured policy and browser contract |
| --- | --- |
| Create support ticket | Six per fixed minute per account and source. The seventh returns localized 429 with the remaining window in `Retry-After`. The open-ticket cap is ten. The form preserves a rejected draft, and closing a ticket frees a slot. `crates/web/src/ratelimit.rs:82`, `crates/support/src/tickets/web.rs:588`. |
| Reply to support ticket | Twelve per fixed minute per account and source. The thirteenth returns the same localized refusal and retry metadata. `crates/web/src/ratelimit.rs:83`, `crates/support/src/tickets/web.rs:706`. |
| Change support ticket status | Twelve per fixed minute per account and source. The thirteenth returns localized 429 with `Retry-After`. `crates/web/src/ratelimit.rs:84`, `crates/support/src/tickets/web.rs:706`. |
| Submit player report | Three attempts per account per rolling minute. The fourth returns the report-specific localized 429 and `Retry-After: 60`. The default pending cap is five and processing a report frees a slot. `crates/support/src/reports.rs:172`, `crates/support/src/reports/web.rs:273`. |
| Create party post | Five live posts and ten creations per rolling hour per account. The party response provides localized 429 with no `Retry-After`. Removing posts opens space under the active cap. `crates/community/src/party/party_store.rs:82`, `crates/community/src/party.rs:383`. |
| Report a party post | Twenty pending reports and twenty new reports per rolling hour per account. The refusal is localized and supplies no `Retry-After`. `crates/community/src/party/party_store.rs:145`, `crates/community/src/party.rs:469`. |
| Place Bazaar bid | Twenty bid requests per fixed minute per account and source. The test posts a valid payload against a missing listing, expects the handler's 404 twenty times, and requires the next response to be a localized 429 with retry metadata. `crates/web/src/ratelimit.rs:80`, `crates/bazaar/src/game/runtime/browser.rs:557`, `crates/bazaar/src/game/runtime/mod.rs:382`. |
| Update a character profile comment | Thirty saved profile preference updates per account in the preceding minute. A comment-only update uses the shared profile budget and returns 429 when it is exhausted. `crates/characters/src/profile/mod.rs:328`. |
| Send referral invitation | Twenty-five email attempts per rolling 24 hours, including resends. Returning invitations also have a cap of twenty attempts per rolling 30 days. Resending to the same recipient is allowed after 14 days. The route redirects to the same localized notice for allowed and suppressed requests and sends no `Retry-After`. `crates/community/src/referrals/campaign.rs:225`, `crates/community/src/referrals/web.rs:232`. |

The separate default restriction after three unjustified player reports in 30 days is enforced by `crates/support/src/reports.rs:202`.

## Findings

- **Personal data export has no request-rate limit.** `GET /{lang}/account/privacy/export` streams an export on each request with verified ownership and recent sudo. The read-only transaction and 30-second deadline bound an individual request. See `crates/identity/src/export.rs:21`, `crates/identity/src/export.rs:50`, and `crates/identity/src/export.rs:107`.
- **Forum post/reply, standalone comment, and standalone contact form routes are absent.** The closest mounted paths are support tickets, private reports, and referral email. See the composition in `bin/oteryn-portal/src/main.rs:133`, `bin/oteryn-portal/src/main.rs:148`, and `bin/oteryn-portal/src/main.rs:201`.
