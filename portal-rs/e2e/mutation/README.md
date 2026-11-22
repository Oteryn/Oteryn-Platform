# Browser mutation matrix

From `portal-rs/e2e`, regenerate and verify the source inventory.

```sh
python3 mutation/generate.py
python3 mutation/generate.py --check
```

`forms.json` records source POST forms, including Rust-rendered markup, and write-method router registrations. A form's source and ordinal identify it when several forms share a line. An empty `action` submits to the current document. `actionVariants` captures finite template expansions inferred from literals or the form's rendering code. Runtime identifiers remain template expressions. Locale-prefixed routes share a row, while unprefixed aliases retain their own rows. The inventory includes source registrations behind feature gates and compatibility routes. Native and internal protocol receivers and provider webhooks are outside the browser inventory. The generator fails when it finds an unresolved write-route binding, so indirect registrations require an explicit source mapping.

Start the disposable PostgreSQL and Portal harness documented in [the E2E guide](../README.md). Give the local native test provider a synthetic coin package as part of the harness startup.

```sh
PORTAL_COIN_PACKAGES='[{"id":"019b0760-0000-7000-8000-000000000006","title":"Native local test coins","coins":121,"price":{"minor_units":250,"currency":"PLN","exponent":2},"valid_for_seconds":300}]' node local.mjs
```

Run the matrix from the same directory.

```sh
node run.mjs /absolute/scratch/env.json test --config mutation/playwright.config.mjs --retries=0
```

For a non-default build directory, export `PORTAL_E2E_BINARY` with the same binary used by the harness, or supply its `CARGO_TARGET_DIR`. The spec uses the binary's seed CLI for disposable accounts and the existing browser helpers for login and ticket setup. The checkout tests use the harness test provider and verify durable payment and wallet effects independently of the page.

| Browser mutation | a | b | c | d | e |
| --- | --- | --- | --- | --- | --- |
| `/{lang}/account/privacy` (POST and PUT) | Yes | Yes | Foreign player and content editor cannot name the owner | Yes | Each save advances the revision |
| `/account/badges` and `/{lang}/account/badges` | Account-age selection and clearing | Yes | Foreign player and content editor cannot name the owner | Yes | Selection unchanged after replay |
| `/{lang}/account/notifications` | Yes | Yes | Foreign player and content editor cannot name the owner | Yes | Same preferences after replay |
| `/{lang}/account/consents` | Grant and revoke both purposes | Yes | Foreign player and content editor cannot name the owner | Malformed fields are 400. Obsolete grant text yields localized conflict | Each decision is a new history row |
| `/oauth/authorize` | Approve creates a bound code. Deny redirects with no grant | Yes | Anonymous | OAuth protocol validation error | The issued code exchanges once |
| `/account/sessions/{id}` and locale alias, POST and DELETE | Session removed and security event stored | Yes | Foreign player and content editor | Malformed ID is 400 | Already-revoked target leaves the same event and session state |
| Session sign-out-others POST and DELETE, both prefixes | Only the current session remains | Yes | Anonymous refused. Authenticated actors cannot target another account | CSRF-only forms | Each successful self-action records its own event |
| Session sign-out-all POST, both prefixes | Login shown and all sessions removed | Yes | Anonymous refused. Authenticated actors cannot target another account | CSRF-only form | Replayed token refused with unchanged state |
| Login and MFA login POST, both prefixes | Login lands on the account homepage and persists a session and login event | Yes | Signed-in foreign player and content editor retain their own sessions | Localized validation | Login intentionally rotates the acting session |
| Registration and work challenge, both prefixes | The real browser solves the work challenge and registration stores one pending account | Yes | Signed-in player and content editor cannot register a second account from their active session. Work is bound to the issuing form | Registration has localized field errors. Challenge issuance has a CSRF header and no user fields | Successful registration consumes the work challenge |
| Verification and resend POST, both prefixes | The local mail sink receives verification and resend messages. Submitting the mailed form activates the account | Yes | Signed-in player and content editor cannot resend for a different account. Verification is authorized by possession of the mailed capability | Malformed capabilities show localized validation. Resend gives generic feedback to preserve account privacy | Activation consumes one token. Resend cooldown preserves the mail and token counts on replay |
| Logout POST, both prefixes | Login page shown and only the acting session revoked | Yes | Signed-in foreign player and content editor cannot revoke the target session | CSRF-only form | Replayed token refused with unchanged state |
| Sudo POST, both prefixes | Browser confirms the password and persists recent authorization for the acting session | Yes | Other player and content editor cannot authorize the target | Localized invalid password feedback | Fresh successful confirmation renews the session proof |
| Password change POST and PUT, both prefixes | Real form changes the credential, transaction records the security event and revokes old sessions | Yes | Other player and content editor refused | Localized input validation | Old request cannot change the credential again |
| Forgot password POST, both prefixes | Actual form queues a capability delivered to the local mailbox | Yes | Signed-in foreign player and content editor are refused | Localized generic confirmation preserves account privacy and malformed input queues nothing | Each newly authorized request can issue a capability |
| Reset password POST, both prefixes | Mailed form changes the password and revokes the earlier session | Yes | Signed-in foreign player and content editor are refused | Localized refusal for malformed fields and mismatched passwords | The capability is accepted once |
| Account termination request and cancellation | The request returns the owner to login and persists one scheduled termination. A fresh owner session cancels it before finalization | Yes | Foreign player and content editor are refused | Localized confirmation and password errors | The request revokes the issuing session. Replayed cancellation leaves the cancelled row unchanged |
| WebAuthn registration, authentication and removal | Chromium authenticator completes real ceremonies, and credential and session records change | Yes | Other player and content editor cannot operate on the owner challenge or credential | Structured authenticator errors and malformed references | Challenge or credential state changes once |
| `/{lang}/unsubscribe` | Signed confirmation form | Signed capability missing or forged | Possession of the signed link grants access | Malformed and overlength capabilities | No additional change on replay |
| `/{lang}/support/tickets` | Yes | Yes | Anonymous | Yes | Request-key replay |
| `/{lang}/support/tickets/{id}/reply` | Yes | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/support/tickets/{id}/status` (POST and PUT, close and reopen) | Yes | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/admin/support/tickets/{id}/reply` (public reply) | Yes | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/admin/support/tickets/{id}/status` (POST resolve and PUT reopen) | Yes | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/support/tickets/{id}/read` | Yes | Yes | Foreign player and content editor | Yes | Unread flag clears once |
| `/{lang}/support/reports` | Yes | Yes | Anonymous, and report stays private from another player | Yes | Request-key replay |
| `/{lang}/shop/coins` | Yes | Yes | Anonymous | Yes | Operation replay |
| Gift code issuance | Staff form returns a coin or premium-days batch. Exactly one code and audit record persist | Yes | Player and content editor | Localized feedback for empty, overlong, malformed and out-of-range values | Matching replay returns the same batch without issuing another code |
| Gift code redemption | Browser JSON request followed by visible wallet credit or premium end date, backed by its ledger | Yes | A valid unused code authorizes redemption. Both a different player and content editor are refused after it is consumed | Localized feedback and unchanged ledger | Replayed operation returns the first receipt and grants value once |
| `/{lang}/shop/transfer` | Yes | Yes | Foreign player and content editor with separate wallets | Yes | One debit and one credit |
| `/{lang}/shop/test-checkout/{intent}` (pay) | Yes | Yes | Foreign player and support staff | Yes | One event and one credit |
| `/{lang}/shop/test-checkout/{intent}` (reject) | Yes | Yes | Foreign player and support staff | Yes | One event and zero credits |
| `/{lang}/shop/test-checkout/{intent}` (cancel) | Yes | Yes | Foreign player and support staff | Yes | One event and zero credits |
| `/native-account/coin-checkout/{order_id}` (pay and fail) | Real test-provider form and visible terminal outcome | Yes | Foreign player and support staff | Four localized invalid decision cases | Exactly one provider event. Pay credits once and fail leaves no credit |
| News create and update (POST and PUT) | Both languages, persisted text and staff audit | Yes | Foreign player and support staff | Malformed title, time and slug refused | Stale revision refused on update |
| News deletion (POST and DELETE) | Editor index and public 404, durable row removed and audited | Yes | Foreign player and support staff | Missing, overlength, malformed and stale revisions conflict | Repeat returns 404 with one deletion |
| Event create, update and status (POST and PUT) | Draft, edit, schedule, publish, archive and return to draft. Status, version, translation and staff IDs checked | Yes | Foreign player and support staff | Localized validation of dates, titles and states | Stale revisions refused on updates and status changes |
| Wiki create, update, revision restore and review actions | Visible draft, review, published and archived states. Changes persist bilingual revisions and a review history | Yes | Foreign player and support staff | Localized malformed text, slug and revision feedback | Stale edits, restores and transitions are refused |
| Media upload and deletion (POST and DELETE) | Upload shows and serves the image. Stored bytes, dimensions, thumbnail and audit entry are verified before deletion | Yes | Foreign player and support staff | Bad metadata, image bytes and IDs are refused | Deleted bytes stay unavailable after replay |
| Staff roles (create, set permissions, grant and revoke) | Role and membership forms are visible. Permissions, members and each audit action persist | Yes | Player and content editor | Empty, overlong and malformed values refused | Each deliberate administrative change is audited |
| `/{lang}/account/characters/new` | Queued, and publisher confirms creation | Yes | Anonymous | Yes | Submission replay |

The documented unavailable views cover collection deck creation/deletion, battle start/turn/forfeit, season-pass purchase/claim, daily puzzle answers, and collection PvP and set purchases. The harness has no accepted collection ownership/progress adapter, season-pass authority adapters, daily puzzle catalogue, or the collection PvP gameplay and wallet adapters. The standard matrix proves that the character creation form persists one pending command. The publisher matrix proves the terminal receipt, durable portfolio, and visible creation result.

Additional refusal boundaries were verified in all four projects. Hunt sharing
returns 503 because the browser composition closes the share route pending its
accepted party privacy scope. The shop purchase form is absent because the
configured browser shop has no authoritative Game ownership adapter and
sets purchases off. A signed-in browser receives 503 on the attempted purchase,
and no store order is persisted. Stats watch POST and DELETE refuse browser
cookies with 401 and no subscription. These routes are bearer-only and the
configured policy denies all bearer principals until the T66 adapter is wired.

The publisher matrix also asserts a–d for both profile methods and both route
prefixes. It uses a different player and an account with the content editor
role as non-owners, requires localized validation for invalid inputs, and
reads the stored profile after each form submission. Profile saves advance
the privacy revision, so the replay criterion is inapplicable.

The journal report page returns 503 because its production `DenySource`
adapter supplies no accepted Game journal authority. The matrix checks the
absent weekly form and the valid-session note submission's 404 with no row.
The email preference route still exposes its account-local write. The matrix
checks CSRF and anonymous refusal, localized input errors, the persisted opt-in
and opt-out, and replay of the same opt-in. The unavailable report page prevents
a browser-form success for either journal row.

The standard harness produces an inactive Game catalogue with no accepted
world version. Its atlas returns 503 without annotation forms. The matrix
exercises the actual annotation endpoints with the browser session, including
marker creation, private reads, cross-account deletion refusal and replay,
and creation of a pending map report. The places participation API also saves
and exposes its account-local choice while the `UnresolvedSource` adapter
leaves the parent page and its form unavailable.

The fake Bazaar runtime mounts the item catalogue as GET-only and exposes
its own sell form. The legacy catalogue POST returns 405 and each of its eight
item actions returns 404 because the configured live Bazaar router replaces
those legacy routes. The tests check that no legacy listing is persisted.

Recruitment creation requires an accepted character profile snapshot and a
fresh `character_profile` watermark in addition to LCFA ownership. The standard
harness leaves the profile family empty. The optional publisher can send that
family with `publicCharacter`. The standard matrix checks the valid-session 403 from creation and the 404 from reporting
or removing an absent post, with no persisted post.

The R0 console dashboard, world, node, and database actions require a
connected Game control agent with a negotiated operation for the selected
target. Neither the standard harness nor the Game publisher fixture
exchanges control-agent hello or result envelopes. An authenticated Game
viewer sees the localized unavailable state and no write form. All seven
console operations reject valid-CSRF submissions with 503 and persist no
control request. Game player searches also require an agent with the player
query capability. A Game support user reaches the unavailable results page
from a valid POST and sees no form or persisted request. The matrix verifies
CSRF, an unrelated player and content editor, and malformed searches.

The world catalogue panel displays no active version or mutation forms with the
harness's pinned unsupported import candidate. Its publish, visibility, and
name submissions with a missing version return 404 for a platform admin and
403 for an unrelated player or content editor. The test verifies that the
legacy and native pointers and world audit records stay unchanged. The
publisher helper supplies character projections and receipts. The world
panel requires an accepted catalogue artifact that this harness cannot publish.

The remaining inventory rows need scenario fixtures and assertions. The table describes the cases implemented in this spec.

For the owner and staff ticket status PUT aliases, Playwright forwards the real form request using PUT and the server handles the request. The report scenario observes the real per-account submission budget before submitting further invalid inputs.

## Game publisher evidence

The existing `specs/game-data.spec.mjs` from publisher commit
`b169fa9e8102e16fb6a3d09782b0749ca5a046e9` passed all four projects with zero
retries. It was run from an isolated local copy with `harness.mjs --game-data`
and the local Portal binary. Run the publisher spec with one worker because
its four projects publish the online snapshot for the same seeded world.
The publisher submits a real CreateCharacter receipt and an LCFA portfolio.
The publisher files were temporary and are outside the pushed branch.

To run the profile matrix, place the publisher commit’s `harness.mjs` and
`game-publishers.mjs` in an isolated local copy containing this matrix.
Run `harness.mjs --game-data` in that copy, then pass its report path to
the following command from `portal-rs/e2e`.

```sh
PORTAL_E2E_GAME_DATA_REPORT=/absolute/path/report.json \
  node node_modules/@playwright/test/cli.js test \
  --config mutation/publisher.config.mjs --workers=1 --retries=0
```

The character-services publisher case also requires the browser service flags
and the accepted V2 projection and command exchange. Add these values to
`test.env` in the isolated publisher copy before starting the harness.

```dotenv
OTERYN_ACCOUNT_CREDENTIALS=true
OTERYN_CHARACTER_SERVICES_ENABLED=true
OTERYN_CHARACTER_SERVICES_SCHEDULE_ENABLED=true
OTERYN_CHARACTER_SERVICES_CANCEL_ENABLED=true
OTERYN_CHARACTER_SERVICES_PROJECTION_ENABLED=true
OTERYN_CHARACTER_SERVICES_EXCHANGE_ENABLED=true
OTERYN_CHARACTER_SERVICES_PROJECTION_AUTHORITY=oteryn-game-e2e
OTERYN_CHARACTER_SERVICES_COMMAND_AUTHORITY=oteryn-game-e2e
OTERYN_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS=2
PORTAL_CHARACTER_SERVICES_EXCHANGE_ENABLED=true
PORTAL_CHARACTER_SERVICES_PROJECTION_ENABLED=true
PORTAL_CHARACTER_SERVICES_PROJECTION_AUTHORITY=oteryn-game-e2e
PORTAL_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS=2
PORTAL_CHARACTER_SERVICES_FRESHNESS_SECONDS=30
```

This case publishes fresh LCFA and character-service watermarks, checks that
foreign players and staff with a different role cannot mutate the character,
and submits the real schedule and cancellation forms. Replaying each
submission preserves its single command. The service history renders both
accepted Game receipts and removes the cancellation form after the active
state is published again.

Paid rename uses the same projection and command exchange. Add these settings
to the isolated publisher copy and provide an ephemeral local
`OTERYN_SHOP_TEST_PAYMENT_KEY` in the harness environment.

```dotenv
OTERYN_CHARACTER_SERVICES_RENAME_ENABLED=true
OTERYN_CHARACTER_RENAME_PRICE=1
OTERYN_SHOP_ENABLED=true
OTERYN_SHOP_TOP_UPS_ENABLED=true
OTERYN_SHOP_TEST_PAYMENTS_ENABLED=true
OTERYN_PAYMENTS_ENABLED=true
OTERYN_SHOP_TOP_UP_PACKAGES=[{"key":"e2e-coins","coins":1234,"minor_units":250,"currency":"PLN"}]
```

The browser buys coins through the test provider. An accepted Game receipt
settles one reserved coin, and the published portfolio and service history show
the new name. Missing and invalid CSRF, foreign players and staff, invalid
names and prices, and replay preserve the checked wallet and order state.
