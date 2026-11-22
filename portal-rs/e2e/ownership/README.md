# Player ownership regression tests

This suite covers the player ownership lane of [issue #2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002). Start the existing browser harness as described in [README.md](../README.md) and use a disposable PostgreSQL database. Set the following variables when launching `local.mjs` so the character service routes reach their ownership check.

```sh
export PORTAL_E2E_BINARY=/absolute/path/to/debug/oteryn-portal
export OTERYN_CHARACTER_SERVICES_COMMAND_AUTHORITY=ownership-e2e
export OTERYN_CHARACTER_SERVICES_PROJECTION_AUTHORITY=ownership-e2e
export OTERYN_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS=1
export OTERYN_CHARACTER_RENAME_PRODUCT_ID=character.rename
export OTERYN_CHARACTER_RENAME_PRODUCT_VERSION=1
export OTERYN_CHARACTER_RENAME_PRICE=70
node local.mjs
```

Run from `portal-rs/e2e` using the scratch path printed by the launcher.

```sh
node run.mjs /absolute/scratch/env.json test --config ownership/playwright.config.mjs \
  --project pl-1440 --project en-390
```

The seed CLI creates two verified players per project. They log in through the normal browser flow in separate contexts. Mutations use B's CSRF token and the required password or sudo proof. The tests compare scoped PostgreSQL rows around each request. Security bookkeeping, such as session touches, rate limits, and refusal audit records, can be written by design. Keep the harness database and Playwright artifacts local.

## Routes and contracts

Paths below omit the `/{lang}` prefix. The router sources also define the locale aliases.

| Source | Covered routes and expected foreign request |
| --- | --- |
| [Character creation](../../crates/characters/src/create_web/http.rs) | GET `/account/characters/requests/{operation}` → 404 |
| [Account character composition](../../crates/web/src/lib.rs), [profile](../../crates/characters/src/profile/http.rs), [private projections](../../crates/community/src/families/web.rs) | GET and POST `/account/characters/{name}/profile` and GET `/account/characters/{name}/{achievements,equipment,outfit}` → 404 |
| [Character services](../../crates/characters/src/services/browser.rs) | GET and POST `/account/characters/{character}/services` → 403 for schedule, cancel, and rename |
| [Support tickets](../../crates/support/src/tickets/web.rs) | GET `/support/tickets/{id}`, POST `/read` and `/reply`, POST and PUT `/status` → 404 |
| [Player reports](../../crates/support/src/reports/web.rs) | GET `/support/reports/{id}` → 404 |
| [Bazaar browser runtime](../../crates/bazaar/src/game/runtime/browser.rs) | GET `/bazaar/{listing}` and `/bazaar/items/{listing}` → 404 for a private draft. POST `/account/bazaar/{listing}/control` → 403 for resume and cancel. POST `/account/bazaar/{listing}/bids` → 409 when the bid request ID belongs to another account. Active listings support cross-player bidding by design. |
| [Shop](../../crates/store/src/web.rs), [checkout](../../crates/store/src/web_checkout.rs), [payment receipts](../../crates/payments/src/account.rs) | GET `/shop/test-checkout/{intent}`, `/shop/coins?checkout={intent}`, and `/panel/payments/receipts/{id}` → 404. POST test checkout pay, reject, and cancel → 404. POST `/shop/transfer` with A's operation ID → 409. GET `/shop/history?before={entry}` uses the ID as a sort boundary and returns B's ledger. |
| [Browser sessions](../../crates/identity/src/sessions_http.rs) | GET `/account/sessions` only lists the current account's sessions. POST and DELETE `/account/sessions/{id}` → 302 to the current account's list with the inactive-session notice. |
| [Passkeys](../../crates/identity/src/passkeys_http.rs) | GET `/account/passkeys` only lists the current account's keys. POST `/account/passkeys/{id}/remove` → 403. |
| [Party](../../crates/community/src/party.rs) | POST `/party/{id}/remove` → 404. GET `/party` and POST `/party/{id}/report` allow viewing and reporting public posts. |
| [Notification preferences](../../crates/identity/src/account_lifecycle_http.rs) | GET and POST `/account/notifications` use the session account. An explicit foreign `account_id` in the form → 422. |
| [Hunt sessions](../../crates/playerdata/src/hunts_web.rs) | GET `/account/hunts/{id}` and `/account/hunts/compare?ids={ids}` → 404. GET and POST `/account/hunts/{id}/share` → 503 while sharing is disabled. |
| [G-10 inventory and history](../../crates/playerdata/src/items/web.rs) | GET `/panel/items/{world}/{character}`, `/panel/history/{world}/{market,trades}/{character}`, and `/panel/history/{world}/bank` → 503 with the default local launcher's missing G-10 authority. This exercises the unavailable contract. Ownership with a live G-10 source remains unqualified. |

[Appeals](../../crates/support/src/appeals.rs) have a player domain service and PostgreSQL tests. There is no composed player appeal HTTP route. [OAuth](../../crates/gameauth/src/oauth/oauth_http.rs) takes credentials in the protocol and has no player API-token management route by object ID. [Guild details](../../crates/community/src/guilds_pages.rs) are public projections. Notification preferences are account-scoped, and the browser has no notification detail or acknowledgement route by ID.

The standalone Bazaar moderation router and private journal deaths router have no matching mounts in the local portal binary.
