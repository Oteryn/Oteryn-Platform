# Browser mutation matrix

From `portal-rs/e2e`, regenerate and verify the source inventory.

```sh
python3 mutation/generate.py
python3 mutation/generate.py --check
python3 ledger/forms.py
python3 ledger/forms.py --check
python3 -B -m unittest discover -s mutation -p 'test_*.py'
```

`forms.json` records source POST forms, including Rust-rendered markup, and write-method router registrations. A form's source and ordinal identify it when several forms share a line. An empty `action` submits to the current document. `actionVariants` captures finite template expansions inferred from literals or the form's rendering code. Runtime identifiers remain template expressions. Locale-prefixed routes share a row, while unprefixed aliases retain their own rows. The inventory includes source registrations behind feature gates and compatibility routes. Native and internal protocol receivers and provider webhooks are outside the browser inventory.

The inventory has 123 source forms and 179 write routes. The ledger recorder and report use all 179 routes in addition to the general route inventory. `ledger/forms.py` maps browser form submissions by method and pathname, including submitters. It checks that every mutation form has a ledger mapping or an explicit route-only classification. The CI workflow contract test checks both inventories and runs the matcher tests.

| Refreshed inventory group | Evidence or classification |
| --- | --- |
| Login and MFA, including replacement confirmation, managed credentials, remembered and native device removal | Form rows use the localized pathname. The staff return query shares the login row. `identity-authentication.spec.mjs`, `ledger-unhit-account.spec.mjs`, and `panels/identity.spec.mjs` exercise the flows. Unprefixed compatibility routes retain their own route rows. |
| Account profile and email-change reversion | Profile form evidence comes from `panels/identity.spec.mjs`. The Rust-rendered email-change flow is classified for route evidence and exercised by `identity-account-security.spec.mjs`. |
| Editorial page approval, event create/edit/status and media upload/delete | Form rows include approval, staff reason and upload-key fields. The content crate covers legal approval and the browser specs cover editorial writes, legal display, and uploads. API PUT requests qualify for route evidence. |
| Staff invitation approval, creation, revocation, acceptance and logout | Form and route evidence comes from `staff-invite.spec.mjs`. Return-language query parameters do not create additional ledger routes. |
| GM enforcement step-up and apply, player retry, and console nodes | Form and route rows are attributed separately by method. `panels/gm-enforce.spec.mjs` exercises the enforcement forms. Player retry remains a tracked gap when the fixture supplies no retryable command. |
| Staff account status, email, review flag and note, and player/staff ticket reply and status | Form rows include the named submitters and required reasons. `panels/accounts.spec.mjs`, `panels/support.spec.mjs`, and the mutation matrix exercise these flows. |
| Bazaar review and staff support actions | Classified for route evidence in `ledger/forms.py`. The mutation inventory reads the staff support template together with the renderer's action names and form fields. The ticket queue's GET filter has its own ledger form. `commerce-bazaar.spec.mjs` and `panels/support.spec.mjs` exercise these routes. |

The checked-in aggregate ledger and delta retain their captured revision. Refresh those reports with a matching full capture when publishing a new baseline.

Start the disposable PostgreSQL and Portal harness documented in [the E2E guide](../README.md). Run the matrix from the same directory.

```sh
node run.mjs /absolute/scratch/env.json test --config mutation/playwright.config.mjs --retries=0
```

For a non-default build directory, export `PORTAL_E2E_BINARY` with the same binary used by the harness, or supply its `CARGO_TARGET_DIR`. The spec uses the binary's seed CLI for disposable accounts and the existing browser helpers for login and ticket setup. The checkout tests use the harness test provider and verify durable payment and wallet effects independently of the page.

| POST mutation | a | b | c | d | e |
| --- | --- | --- | --- | --- | --- |
| `/{lang}/account/privacy` | Yes | Yes | Anonymous | Yes | Each save advances the revision |
| `/{lang}/support/tickets` | Yes | Yes | Anonymous | Yes | Request-key replay |
| `/{lang}/support/tickets/{id}/reply` | Yes | Yes | Anonymous, foreign player, content editor on another account | Yes | Stale lock version |
| `/{lang}/support/tickets/{id}/status` (close) | Yes | Yes | Anonymous, foreign player, content editor on another account | Yes | Stale lock version |
| `/admin/support/tickets/{id}` (reply and resolve) | Yes, built-in support role | Yes | Anonymous, other player, content editor lacking `support.tickets.manage` | Yes | Stale lock version |
| `/{lang}/shop/coins` | Yes | Yes | Anonymous | Yes | Operation replay |
| `/{lang}/shop/test-checkout/{intent}` (pay) | Yes | Yes | Foreign player | Yes | One event and one credit |
| `/{lang}/shop/test-checkout/{intent}` (reject) | Yes | Yes | Foreign player | Yes | One event and zero credits |
| `/{lang}/shop/test-checkout/{intent}` (cancel) | Yes | Yes | Foreign player | Yes | One event and zero credits |
| `/{lang}/account/characters/new` | Queued, Game outcome pending | Yes | Anonymous | Yes | Submission replay |

The unavailable Game-backed case checks the views and writes. It verifies missing and invalid CSRF, valid-CSRF malformed inputs, anonymous and authenticated refusal, and unchanged durable state. Portal-owned collection fixtures also prove owner forfeit and deletion with Game offline, exact redirects, and replay refusal. The harness has no accepted collection ownership/progress adapter or daily puzzle catalogue. The season-pass matrix uses an empty season catalog with the testing Wallet enabled. The character creation matrix verifies the pending command and submission replay. [The Game-data spec](../specs/game-data.spec.mjs) uses `harness.mjs --game-data` to publish a CreateCharacter receipt and verify the confirmed outcome.

The remaining inventory rows need scenario fixtures and assertions. The table describes the cases implemented in this spec.

## Runtime dependencies for Game-backed write rows

Here a is an authorized success with a durable effect, b is CSRF rejection, c is
authorization rejection, d is field validation, and e is replay or concurrency.
An unavailable GET establishes why the browser has no valid form context. Columns
b, c and d require their own write requests where middleware can be exercised.

| Inventory write row | Positive and replay dependency |
| --- | --- |
| `POST /{lang}/collection/{character}/decks` | The composed collection router in `crates/minigames/src/collection/http.rs` uses `Ownership(Unavailable)` and a progress store with no configured source. LCFA character projection does not supply collection ownership. |
| `POST /{lang}/collection/decks/{id}/battle` | Requires a saved deck and current private collection progress. Both depend on the unavailable collection adapters. |
| `POST /{lang}/collection/battles/{id}` | Requires an active battle and a turn version issued by its start operation. The prerequisites for starting a battle are unavailable. |
| `POST /{lang}/pass/{season}/buy` | The enabled testing Wallet rejects purchases targeting a missing season with HTTP 403. [The season-pass spec](../specs/season-pass.spec.mjs) seeds a purchasable season and verifies purchase and replay. |
| `POST /{lang}/pass/{season}/claim` | The composed router keeps reward claims disabled and returns HTTP 404. |
| `POST /{lang}/puzzles/{id}/answer` | The basic mutation fixture has no eligible daily definition. `puzzles.spec.mjs` starts the dedicated puzzle fixture, publishes through staff ingress and exercises the answer. |
| `POST /{lang}/shop` | `crates/store/src/web_backend.rs::configured` constructs the browser backend with `purchases: None` and an access adapter whose purchase target check always returns false. The matrix checks CSRF and the composed 503 response with no purchases, debits, or deliveries. The Store crate's enabled-backend purchase test checks malformed fields and durable state. |
