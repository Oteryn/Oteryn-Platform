# Player reads (T36b)

These routes are new (§5.2 of the Rust plan). Neither `routes/api.php` nor
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
