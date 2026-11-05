# Staff player tools (T49)

Routes: `GET /admin/players?q=...`, `GET /admin/players/{id}` and
`POST /admin/players/{id}`. Search accepts an exact account UUID or email and
never returns the email, credentials, network addresses or session tokens.
Forms support `note`, `logout`, `request-reset` and `approve-reset` actions.

`OTERYN_ADMIN_PLAYERS=true` enables account features; the default is off.
`OTERYN_ADMIN_PLAYER_CHARACTERS=true` separately enables character reads.
Both use the T10b staff host/session/CSRF/MFA guard. The existing
`support.tickets.manage` permission governs player assistance. Reset approval
also requires `admin.roles.manage`; requester and approver must be distinct,
currently authorized staff, and neither may reset their own account.
Role writes and player mutations share the T10 authorization advisory lock.
Notes and security mutations commit with their audit record or roll back.
MFA requests expire after 30 minutes and cannot be replayed.

Character reads use the public `characters::lcfa::Projection` API, preserving
its identity, watermark, epoch and freshness checks. The configured adapter
requires `OTERYN_ENV=testing|preprod` and
`OTERYN_ACCOUNT_CHARACTERS_AUTHORITY` matching the publishing authority.
Missing/stale/invalid projections are unavailable, never an empty success.

## Dependency seams

T10b is integrated in the base. No T08 branch existed on origin at task start.
The base identity API provides account lookup but no session listing, complete
revocation or MFA reset API. `Accounts` is the narrow seam for those operations:
implementations receive the transaction connection and must revoke every
session/token class, update revocation fences and record security events in
that transaction. Support evidence must come from authoritative verification,
remain valid until commit and be consumed by reset. Client input alone cannot
establish verification. The configured identity adapter refuses these missing
operations, and session views explicitly show unavailable information.

The configured T10b authentication adapter also remains deny-by-default.
`http::router` accepts the trusted staff/session and account adapters for
integration. Production activation needs those authoritative adapters; this
module does not guess session fields or write another crate's tables.

## References and validation

The governing design is the Rust plan sections 4, 5.4, 7, 8 and row T49.
The native account identity and remembered-device-session contracts establish
canonical account identity and complete revocation requirements. The Laravel
`ResetIdentityMfa` and `RevokeIdentityWebSessions` actions establish transactional
reset and revocation behavior. There is no Laravel staff-player HTTP route to
copy; these SSR mutations use existing staff-panel 303/400/403/404/503 conventions.
No Game wire format is changed and no external fixture is copied.

PostgreSQL tests use isolated schemas, skip with a notice when
`PORTAL_TEST_DATABASE_URL` is unset, and cover actor-to-result HTTP/CSRF,
authorization, bad input, account lookup, flags, notes, transactional rollback,
support verification, request expiry, permission revocation and concurrent
single-use reset approval. Templates use escaped Askama output, local Fluent
Polish/English messages and the T39 responsive, labelled keyboard-accessible
components without adding CSS or JavaScript.
