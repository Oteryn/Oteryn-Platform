# GAP-08 account data qualification

## Platform delivery

- Registered clients use the existing authorization-code flow with PKCE S256,
  exact registered callbacks, browser approval and account MFA where required.
  Request the single `account:read` scope. It authorizes the Platform account
  summary and portal notification feed. It is independent of both native
  Game-ticket and coin-purchase authorization.
- `GET /api/v1/account/summary` returns the bearer owner's account UUID and
  locale. The balance field is `coins: null` in this composition.
- `GET /api/v1/feed` returns the owner's portal notifications, newest first,
  with `limit` in 1–100 (default 20) and optional `unread_only=true|false`.
  An empty feed means the owned Platform read succeeded and found no matches.
  There is no client-side acknowledgement or cursor.
- Send `Authorization: Bearer <access token>`. An active, verified account and
  a live grant for a configured client are required. Foreign account selectors,
  out-of-range limits and malformed filters return 400. Missing or invalid
  credentials return 401 when OAuth is configured. Disabled OAuth or an
  unavailable backend returns 503. Private responses are never cached.
- The Account menu opens the localized Journal Dashboard. Its HTTP 503 page
  retains the portal shell and explains that journal data is unavailable until
  a qualified Game source is composed.
- The existing optional browser consent page appears in Account settings when
  `OTERYN_ACCOUNT_CONSENTS_ENABLED=true`. The default is off. Each decision
  remains separate from OAuth approval and from legal acceptance. The
  [optional-consent client contract](../contracts/OTERYN_V2_ACCOUNT_CONSENTS_CLIENT_CONTRACT.md)
  remains proposed.

## Operator verification

On the staff host, `/admin` shows Account data integrations after the usual
staff sign-in and MFA. The panel reports whether the deployment has a usable
OAuth client registration, whether optional browser consent collection is
enabled, and the availability of the journal and account-summary coin balance.
A configured registration is configuration status. Complete a real PKCE
approval and read both endpoints to verify runtime readiness. Check that the feed
contains only the approving account's notifications and that a ticket grant
cannot read it.

## Remaining Game and client decisions

- Launcher and Game-client owners must qualify support for `account:read`,
  exact redirect handling, platform-only feed semantics, and 401/503 states
  before advertising the actions in a client release.
- Journal activation needs qualified G-9 closed-session and G-11 observation
  producers, independently verified ownership and complete baselines, and a
  reviewed Platform source adapter. Keep the dashboard in the unavailable state
  until those inputs are proven.
- Ratify the browser consent entry point separately. A Game-side consumer would
  require a separate reviewed account binding, revocation and failure contract
  before it may use the recorded decision.

The [local actor-to-result spec](../../portal-rs/e2e/specs/gap-08.spec.mjs)
uses the portal runner's OAuth registration and both desktop and 390 px
projects. It exercises real PKCE approval, private reads, the dashboard
unavailable state, and the existing optional browser consent forms.
