# Browser mutation matrix

From `portal-rs/e2e`, regenerate and verify the source inventory.

```sh
python3 mutation/generate.py
python3 mutation/generate.py --check
```

`forms.json` records source POST forms, including Rust-rendered markup, and write-method router registrations. A form's source and ordinal identify it when several forms share a line. An empty `action` submits to the current document. `actionVariants` captures finite template expansions inferred from literals or the form's rendering code. Runtime identifiers remain template expressions. Locale-prefixed routes share a row, while unprefixed aliases retain their own rows. The inventory includes source registrations behind feature gates and compatibility routes. Native and internal protocol receivers and provider webhooks are outside the browser inventory. The generator fails when it finds an unresolved write-route binding, so indirect registrations require an explicit source mapping.

There are 108 source forms and 165 write-method and route pairs. The table
covers route aliases and named submit choices together where they use the same
form, authorization and persistence rules.

Gift redemption is a browser JSON endpoint. The test submits its real request
with the account's CSRF token and verifies the grant in the wallet or premium
screen and ledger.

Start the disposable PostgreSQL and Portal harness documented in [the E2E guide](../README.md). Give the local native test provider a synthetic coin package as part of the harness startup.

```sh
PORTAL_COIN_PACKAGES='[{"id":"019b0760-0000-7000-8000-000000000006","title":"Native local test coins","coins":121,"price":{"minor_units":250,"currency":"PLN","exponent":2},"valid_for_seconds":300}]' node local.mjs
```

Run the matrix from the same directory.

```sh
umask 077
node run.mjs /absolute/scratch/env.json test --config mutation/playwright.config.mjs --retries=0
```

For a non-default build directory, export `PORTAL_E2E_BINARY` with the same binary used by the harness, or supply its `CARGO_TARGET_DIR`. The spec uses the binary's seed CLI for disposable accounts and the existing browser helpers for login and ticket setup. The checkout tests use the harness test provider and verify durable payment and wallet effects independently of the page.

| Browser mutation | a | b | c | d | e |
| --- | --- | --- | --- | --- | --- |
| `/{lang}/account/privacy` (POST and PUT) | Yes | Yes | Foreign player and content editor cannot name the owner | Yes | Each save advances the revision |
| `/account/badges` and `/{lang}/account/badges` | Account-age selection and clearing | Yes | Foreign player and content editor cannot name the owner | Yes | Selection unchanged after replay |
| `/{lang}/account/notifications` | Yes | Yes | Foreign player and content editor cannot name the owner | Yes | Same preferences after replay |
| `/{lang}/account/consents` | Grant and revoke both purposes | Yes | Foreign player and content editor cannot name the owner | Malformed fields are 400. Obsolete grant text yields localized conflict | Each decision is a new history row |
| `/oauth/authorize` | Approve creates a bound code. Deny redirects with no grant | Yes | Anonymous, another player and content editor are refused when they attempt to authorize the owner's grant | OAuth protocol validation error | The issued code exchanges once |
| `/account/sessions/{id}` and locale alias, POST and DELETE | Session removed and security event stored | Yes | Foreign player and content editor | Malformed ID is 400 | Already-revoked target leaves the same event and session state |
| Session sign-out-others POST and DELETE, both prefixes | Only the current session remains | Yes | Anonymous refused. Authenticated actors cannot target another account | CSRF-only forms | Each successful self-action records its own event |
| Session sign-out-all POST, both prefixes | Login shown and all sessions removed | Yes | Anonymous refused. Authenticated actors cannot target another account | CSRF-only form | Replayed token refused with unchanged state |
| Login and MFA login POST, both prefixes | Login lands on the account homepage and persists a session and login event | Yes | Signed-in foreign player and content editor retain their own sessions | Localized validation | Login intentionally rotates the acting session |
| MFA enrollment, confirmation, email, challenge and device removal POST, both prefixes | A real authenticator enrolls, the mail sink receives the challenge proof, and the remembered device disappears after removal | Yes | A foreign player and content editor cannot enroll with the owner's credentials, finish the owner's challenge or remove the owner's device | Localized password, proof and device validation. The email dispatch form has only CSRF | Confirmation rotates the session. The email proof and remembered-device removal each apply once |
| Registration and work challenge, both prefixes | The real browser solves the work challenge and registration stores one pending account | Yes | Signed-in player and content editor cannot register a second account from their active session. Work is bound to the issuing form | Registration has localized field errors. Challenge issuance has a CSRF header and no user fields | Successful registration consumes the work challenge |
| Verification and resend POST, both prefixes | The local mail sink receives verification and resend messages. Submitting the mailed form activates the account | Yes | Signed-in player and content editor cannot resend for a different account. Verification is authorized by possession of the mailed capability | Malformed capabilities show localized validation. Resend gives generic feedback to preserve account privacy | Activation consumes one token. Resend cooldown preserves the mail and token counts on replay |
| Logout POST, both prefixes | Login page shown and only the acting session revoked | Yes | Signed-in foreign player and content editor cannot revoke the target session | CSRF-only form | Replayed token refused with unchanged state |
| Sudo POST, both prefixes | Browser confirms the password and persists recent authorization for the acting session | Yes | Other player and content editor cannot authorize the target | Localized invalid password feedback | Fresh successful confirmation renews the session proof |
| Password change POST and PUT, both prefixes | Real form changes the credential, transaction records the security event and revokes old sessions | Yes | Other player and content editor refused | Localized input validation | Old request cannot change the credential again |
| Email change, confirmation and reversal POST, both prefixes | The browser requests the change and receives both capabilities through the local mail sink. Confirmation and reversal each update the stored account and security generation | Yes | Other player and content editor are refused with the owner's credentials or a forged capability | Localized form and token refusals | Each mailed capability changes account state once |
| Forgot password POST, both prefixes | Actual form queues a capability delivered to the local mailbox | Yes | Signed-in foreign player and content editor are refused | Localized generic confirmation preserves account privacy and malformed input queues nothing | Each newly authorized request can issue a capability |
| Reset password POST, both prefixes | Mailed form changes the password and revokes the earlier session | Yes | Signed-in foreign player and content editor are refused | Localized refusal for malformed fields and mismatched passwords | The capability is accepted once |
| Account termination request and cancellation | The request returns the owner to login and persists one scheduled termination. A fresh owner session cancels it before finalization | Yes | Foreign player and content editor are refused | Localized confirmation and password errors | The request revokes the issuing session. Replayed cancellation leaves the cancelled row unchanged |
| Recovery-key generation, download acknowledgement, revocation and recovery, both prefixes | The owner displays and activates a new key, recovers through the public form, then revokes it with the account form | Yes | Another player and content editor cannot manage the owner key or recover the account without a valid key | Localized invalid credentials and confirmation. Download acknowledgement has no fields beyond its CSRF token | Activation and revocation preserve one event on replay. Public recovery issues at most one session per usable key |
| WebAuthn registration, authentication and removal | Chromium authenticator completes real ceremonies, and credential and session records change | Yes | Other player and content editor cannot operate on the owner challenge or credential | Structured authenticator errors and malformed references | Challenge or credential state changes once |
| `/{lang}/unsubscribe` | Signed confirmation form | Signed capability missing or forged | Possession of the signed link grants access | Malformed and overlength capabilities | No additional change on replay |
| `/{lang}/support/tickets` | Yes | Yes | Anonymous. A foreign player and content editor cannot submit the owner's form or read the ticket | Yes | Request-key replay |
| `/{lang}/support/tickets/{id}/reply` | Yes | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/support/tickets/{id}/status` (POST and PUT, close and reopen) | Yes | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/admin/support/tickets/{id}/reply` | Public replies appear for both parties. Internal notes persist and remain visible to staff | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/admin/support/tickets/{id}/status` (POST and PUT) | Resolved, open, waiting for player, waiting for staff and closed | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/support/tickets/{id}/read` | Yes | Yes | Foreign player and content editor | Yes | Unread flag clears once |
| `/{lang}/support/reports` | Yes | Yes | Anonymous. A foreign player and content editor cannot submit the owner's form or read the report | Yes | Request-key replay |
| `/{lang}/shop/coins` | Yes | Yes | Anonymous, foreign player and unrelated support staff are refused for the owner's operation | Yes | Operation replay |
| Gift code issuance | Staff form returns a coin or premium-days batch. Exactly one code and audit record persist | Yes | Player and content editor | Localized feedback for empty, overlong, malformed and out-of-range values | Matching replay returns the same batch without issuing another code |
| Gift code redemption | Browser JSON request followed by visible wallet credit or premium end date, backed by its ledger | Yes | A valid unused code authorizes redemption. Both a different player and content editor are refused after it is consumed | Localized feedback and unchanged ledger | Replayed operation returns the first receipt and grants value once |
| `/{lang}/shop/transfer` | Yes | Yes | Foreign player and content editor with separate wallets | Yes | One debit and one credit |
| `/{lang}/shop/test-checkout/{intent}` (pay) | Yes | Yes | Foreign player and support staff | Yes | One event and one credit |
| `/{lang}/shop/test-checkout/{intent}` (reject) | Yes | Yes | Foreign player and support staff | Yes | One event and zero credits |
| `/{lang}/shop/test-checkout/{intent}` (cancel) | Yes | Yes | Foreign player and support staff | Yes | One event and zero credits |
| `/native-account/coin-checkout/{order_id}` (pay and fail) | Real test-provider form and visible terminal outcome | Yes | Foreign player and support staff | Four localized invalid decision cases | Exactly one provider event. Pay credits once and fail leaves no credit |
| Staff commerce grant, revoke, adjustment, reconciliation and refund | Bilingual staff forms record changes to premium and wallets, review a real test payment, and refund one uncredited payment | Yes | Another player and content editor are refused | Localized identifier, amount and reason validation | Replaying each request preserves its operation and effect |
| Staff commerce approval and cancellation | A different staff member approves or cancels the request, and the panel removes its action form | Yes | Another player, content editor and the proposing staff member are refused | Localized malformed request IDs | One grant or adjustment after approval, and no effect after cancellation |
| News create and update (POST and PUT) | Both languages, persisted text and staff audit | Yes | Foreign player and support staff | Malformed title, time and slug refused | Stale revision refused on update |
| News deletion (POST and DELETE) | Editor index and public 404, durable row removed and audited | Yes | Foreign player and support staff | Missing, overlength, malformed and stale revisions conflict | Repeat returns 404 with one deletion |
| Event create, update and status (POST and PUT) | Draft, edit, schedule, publish, archive and return to draft. Status, version, translation and staff IDs checked | Yes | Foreign player and support staff | Localized validation of dates, titles and states | Stale revisions refused on updates and status changes |
| Wiki create, update, revision restore and review actions | Visible draft, review, published and archived states. Changes persist bilingual revisions and a review history | Yes | Foreign player and support staff | Localized malformed text, slug and revision feedback | Stale edits, restores and transitions are refused |
| Media upload and deletion (POST and DELETE) | Upload shows and serves the image. Stored bytes, dimensions, thumbnail and audit entry are verified before deletion | Yes | Foreign player and support staff | Bad metadata, image bytes and IDs are refused | Deleted bytes stay unavailable after replay |
| Editorial pages save and delete | Bilingual pages become readable, translations are refreshed when the source changes, and deletion removes the page | Yes | Foreign player and support staff | Localized title, length, date and version validation | Stale revisions cannot change the publication |
| Announcements create and edit | A bilingual draft is private, publication makes the ticker visible in both languages, and changing back to draft removes it | Yes | Foreign player and support staff | Localized field and schedule validation | Repeating an old revision leaves the ticker and database unchanged |
| Staff roles (create, set permissions, grant and revoke) | Role and membership forms are visible. Permissions, members and each audit action persist | Yes | Player and content editor | Empty, overlong and malformed values refused | Each deliberate administrative change is audited |
| Legacy staff ticket and report actions | Public replies and outcomes reach owners, staff notes stay private, and every exposed status is persisted | Yes | Another player and content editor are refused | Localized body, version and status validation | Stale requests cannot produce a second mutation |
| Staff player account actions | Notes, forced logout, kick, identity verification, MFA-reset request and independent approval appear in the panel and persist as scoped state and audit actions | Yes | Another player, unrelated content editor and a support proposer without approval authority are refused | Localized reason, action and identifier validation | MFA reset completes once. Notes and standalone revocations are deliberate staff actions |
| Referral link, new and returning invitations, and acceptance | The link and localized queue notice appear. A mailed invitation binds the eligible returning account, and a new player can join through the referral form | Yes | Another player and a content editor cannot submit for the inviter or accept a bound invitation meant for someone else | Localized kind, address and capability validation | Replays preserve one link, one outgoing invitation per recipient and one binding per invitee |
| Enforcement sanctions and appeal decisions | Staff creates warning, restriction and suspension records, expires or revokes a sanction, and reviews, accepts or rejects an appeal | Yes | Another player and content editor are refused | Localized reason, state, date and version checks | Stale submissions cannot repeat a transition |
| Bazaar item and character listing | The fixture's Game service confirms custody and the listing becomes active. The item escrow or both character custody receipts persist | Yes | Another player and unrelated content editor cannot duplicate custody | Localized kind, revision and duration validation | Matching submission returns the existing listing with one decision per Game operation |
| Bazaar Coin and Gold bids | A real test-provider wallet makes a Coin hold. The fake Game bank holds Gold and settles the auction at its clearing price | Yes | Seller is refused. Coin refuses the unrelated content editor on submission. For Gold, the Game bank rejects the editor's foreign character with a receipt and leaves all balances and the leader unchanged | Localized amount, format and bounds validation | One hold per successful bid, one Gold settlement, and one retained refusal on replay of the foreign bid |
| Bazaar seller resume and cancel | The seller rebinds the listing to a fresh browser session and cancels a separate unbid listing through the real form | Yes | Another player and content editor are refused | Localized action and request-key validation | One browser operation per submission and one Game return |
| Bazaar staff fees, suspension, restriction, review and invalidation | Staff updates the fee revision, holds the listing, records a restriction, reviews both decisions, and uses a separate approver for invalidation with one compensated Game return | Yes | Another player, unrelated content editor, and the invalidation proposer cannot approve | Localized identifier, reason, fee and review validation | Reviewed reports and approved invalidations produce one effect. A deliberate repeat suspension revises the hold |
| `/{lang}/account/characters/new` | Queued, and publisher confirms creation | Yes | Anonymous. A foreign player and content editor cannot submit the owner's form or read the command | Yes | Submission replay |
| Character profile settings (POST and PUT, both prefixes) | The owner sees the published profile and its saved visibility and presentation settings | Yes | Another player and content editor cannot edit the published character | Localized visibility and version validation | Each intentional save advances the revision |
| Character services (schedule, cancel and paid rename) | The owner sees Game-confirmed changes. Rename settles one coin from the test-funded wallet | Yes | Another player and unrelated staff are refused | Localized confirmation, name and price validation | One Game command per operation and one rename debit |
| Recruitment create, remove, report and staff moderation | The publisher supplies the owned character. Browser lists show creation, owner removal, dismissal and staff removal | Yes | Another player and unrelated staff are refused from owner or moderation actions | Localized character, report and moderation validation | Requests preserve unique posts and reports |

Temporary Game producers supply the profile, character-service and recruitment
prerequisites through the existing internal ingress. The cases run from
`matrix.spec.mjs` under the publisher configuration.

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
checks missing and invalid CSRF, denial for an anonymous browser and owner
overrides from another player and content editor, localized input errors,
the persisted opt-in and opt-out, and replay of the same opt-in. The
unavailable report page prevents a browser-form success for either journal row.

The standard harness produces an inactive Game catalogue with no accepted
world version. Its atlas returns 503 without annotation forms. The matrix
exercises the annotation JSON endpoints with real browser sessions, including
marker creation, private reads, cross-account deletion refusal and replay,
and creation of a pending map report. Another player and an unrelated content
editor cannot name the owner's account in a create request or delete the
owner's marker. Malformed JSON fields return the endpoint's 422 response.
The places participation endpoint saves and exposes both account-local choices
while the `UnresolvedSource` adapter leaves its form unavailable. Each save
advances the stored privacy revision.

The fake Bazaar runtime mounts the item catalogue as GET-only and exposes
its own sell form. The legacy catalogue POST returns 405. The legacy
`accept`, `reject`, `publish`, `renew`, `cancel`, `bid`, `offer` and
`buy-now` actions return 404 under the configured live Bazaar router. The
tests check that no legacy listing is persisted.

The Bazaar report and promotion pages render their real forms for an active
listing. The configured `PendingIntegration` adapter cannot authorize the
account, confirm public listing authority or debit the promotion ledger.
The matrix verifies localized refusal, CSRF, the unrelated actors, and
malformed inputs. Valid submissions create neither a report nor a promotion
or debit. The staff review case seeds the historical report prerequisite in
the disposable database and uses the real staff decision forms.

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

Staff step-up consumes a real TOTP and stores the acting account's proof for
the named request. The approval page and POST refuse a missing preview with
an unchanged approval table. Approval requires the verified successful dry-run
result from an R4 Game agent, which the harness cannot publish.

The native sandbox creates paid and failed orders through its hosted form. The
staff player panel reports no purchase hold after the paid order and exposes
no release form. A valid request with a nonexistent hold conflicts and leaves
the wallet and order unchanged. The sandbox signs its own pay and fail events
with an in-process key and supplies no native provider reversal event to create
a chargeback hold.

The world catalogue panel displays no active version or mutation forms with the
harness's pinned unsupported import candidate. Its publish, visibility, and
name submissions with a missing version return 404 for a platform admin and
403 for an unrelated player or content editor. The test verifies that the
legacy and native pointers and world audit records stay unchanged. The
publisher helper supplies character projections and receipts. The world
panel requires an accepted catalogue artifact that this harness cannot publish.

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

Enable `PORTAL_PARTY_ENABLED=true` in the publisher instance for recruitment.
The publisher supplies the owned character and a fresh public profile
watermark. Browser forms create and remove recruitment posts, submit reports,
and resolve reports as staff by dismissing them or removing the post. The
matrix checks visible listings, owner and staff permissions, localized field
refusals, and the persisted posts and reports.

## Refund fixture

Run the uncredited payment case in a separate instance of `local.mjs`. The
worker's existing `OTERYN_WALLET_ENABLED` switch defers credit delivery while
the server and refund worker handle actual test-provider payments. Use a private
key file for the test provider's settlement webhook.

```sh
umask 077
export PORTAL_E2E_REFUND_BINARY=/absolute/path/to/oteryn-portal
export PORTAL_E2E_REFUND_KEY_FILE="$(mktemp)"
openssl rand -hex 32 | tr -d '\n' > "$PORTAL_E2E_REFUND_KEY_FILE"
export PORTAL_E2E_BINARY="$PWD/mutation/refund-worker.sh"
node local.mjs
```

With the same environment, run the tagged case in all four projects.

```sh
node run.mjs /absolute/scratch/env.json test --config mutation/refund.config.mjs --retries=0
```

The purchaser sees the uncredited payment. Two distinct staff identities
submit and approve the refund, and the refund worker records one dispatch.
The signed test-provider settlement appears in the browser as refunded, and
replaying approval and settlement leaves one refund and zero wallet credits.
