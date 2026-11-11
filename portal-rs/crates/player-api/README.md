# Native bootstrap API

T17 owns `POST /api/v1/game-auth/tickets` and
`GET /api/v1/game-auth/native-characters`. Governing wire contracts are
`docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` sections 4 and 5.3
and design section 5.2. Golden shapes are checked against the Game client at
`857f15f376907c592bb28d854fccff18a15cdf87`.

`configured_router(pool, oauth, accounts)` takes the OAuth authority and the
account-state lookup from the caller. The binary (`bin/oteryn-portal/src/game_login.rs`)
passes the T16 `PlatformAuthority` (native client allowlist, generations and
atomic access-token consumption on the caller's connection) wrapped in the
sanctions `GuardedTickets`, so a suspended, closed or unknown account gets 401
and its token is not consumed. The character list reads the T18 projection
with `oteryn_characters::overview::configured_projection()`.

Flags, all off by default:

- `PORTAL_GAME_TICKETS_ENABLED=true` mounts both routes enabled. Ticket
  issuance is still refused unless `OTERYN_ENV` is `testing` or `preprod`
  (N4P-3); production issuance needs an owner decision.
- `OTERYN_LCFA_ENABLED` and `OTERYN_LCFA_SOURCE_AUTHORITY` configure the
  character projection. Without them the character list answers unavailable.
- `PORTAL_LCFA_INTERNAL_IDENTITY` is the DNS SAN of the Game LCFA producer
  certificate. When set, LCFA snapshot and watermark ingestion is also served on
  the internal mTLS listener. The closure listener keeps its own ingestion; its
  `PORTAL_ACCOUNT_CLOSURE_SOURCE_AUTHORITY` must equal
  `OTERYN_LCFA_SOURCE_AUTHORITY`.

The gateway's `native-admissions` listener (`PORTAL_NATIVE_ADMISSIONS_ENABLED`)
uses `oteryn_gameauth::admission::NativeTickets` behind the sanctions
`GuardedAdmission`: it redeems tickets and checks the selected character
against the live projection. Grant signing (contract sections 7.3, 8 and 9) is
not implemented, so a request that passes every check gets
`NATIVE_LOGIN_UNAVAILABLE` and the ticket stays unused.

`OAuthAuthority::verify` runs on the caller's transaction connection. It must
lock and verify the token, first-party client and authoritative account, check
scope, expiry, revocation and security generation, and return explicit active
account state. Consumption must revoke the access token and its refresh tokens
on that same connection. Ticket persistence and consumption commit together.
Character reads use `consume=false`, bind the projection to that account and
revalidate account and generation after the projection read. Unknown state denies.

Tickets contain 256 random bits, encoded without padding. Only SHA-256 digests
persist, with the native audience, account, both generations, expiry and future
redemption fields. A subsequent admission adapter must lock the ticket, verify
its audience, expiry, unused state and current account generations, then mark it
used on the admission transaction connection. T17 does not authorize admission.

PostgreSQL tests create isolated schemas and read `PORTAL_TEST_DATABASE_URL`.
They print a skip notice if it is absent. No compile-time SQL query macros or
new offline query metadata are required.
# Player reads (T36b)

These routes are new (§5.2 of the Rust plan). Neither `legacy/routes/api.php` nor
`docs/contracts` defines an existing summary/feed response. These shapes are
Platform-local and are not a new Game wire contract. Directory is excluded
until G-1.

- `GET /api/v1/account/summary`: `{account_id, locale, coins}`. Only a verified,
  active owner is admitted. Credentials and email are never returned.
  `coins: null` means the separate money flag is off. When enabled, the
  `CoinBalance` adapter reads the public wallet `Ledger::balance` API; missing
  wiring or a failed read returns 503. Zero is never inferred from missing data.
- `GET /api/v1/feed?limit=20&unread_only=false`:
  `{items:[{id,subject,body,read}]}`. The private notification feed uses T36's
  public `Notifications::list` API and its deterministic newest-first ordering.
  Limits are 1–100. This is a bounded latest-items view without cursor or
  acknowledgement mutations. Public news remains served by T11's `/news`;
  its crate currently exposes no typed read API to compose into this feed.
- Invalid query fields/values return 400. Missing, malformed, duplicate or
  invalid bearer credentials return 401. Inactive/unverified/unknown account
  state returns 403. Disabled features and backend failures return 503.
- All responses are private and no-store, including errors. Cookies and
  caller-selected account IDs are never used for authentication.

`Api` is the composition seam. Its `enabled` field must be explicitly enabled
and `BearerAuthority` must verify access-token expiry, revocation and scope.
The binary mounts `configured_router()` with the feature off and a deny-all
bearer authority. This base has no public player OAuth verifier, so production
wiring cannot safely enable these routes yet. `PrivateReads` rechecks Identity
state on every read and calls Notify through its public API. No migration is
added.

Tests cover HTTP success/refusal/input validation and PostgreSQL owner
isolation, account state and unavailable balance behavior. Set
`PORTAL_TEST_DATABASE_URL` to run the PostgreSQL test; it creates and removes
an isolated schema. Without it the test prints a skip notice.
