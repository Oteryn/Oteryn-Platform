# Account overview

`/account` resolves its principal from the existing server-side browser session.
`OTERYN_ACCOUNT_OVERVIEW_ENABLED=true` enables the page; it is off by default.
Only an explicitly active account can read it. Responses are private, not cached
and excluded from indexing. The page does not accept an account identifier.

LCFA reads use the public T18 projection API. Enable them separately with
`OTERYN_LCFA_ENABLED=true`, `OTERYN_LCFA_SOURCE_AUTHORITY` and a supported
`OTERYN_ENV` (`testing` or `preprod`). Missing, stale, invalid and disabled feeds
never render character rows. A current empty snapshot remains a current game
connection. Names are escaped and encoded as single profile URL segments.

`OTERYN_ACTIVE_CHARACTER_LIMIT` and `OTERYN_TOTAL_CHARACTER_LIMIT` must both be
positive integers, with total at least active. These are informational limits;
this page never grants character creation permission. Invalid configuration is
unavailable, never an unlimited allowance.

`OTERYN_ACCOUNT_BALANCE_ENABLED=true` separately enables the wallet's public
locked balance read. The adapter refuses every transfer and adjustment. Missing
wallet state does not become a zero balance.

## Producer boundaries still pending

The base includes T18 but no T08 branch. Browser sessions exist, but neither a
last successful website login reader nor the planned security-event reader is
published. Session activity is not a login timestamp.

LCFA publishes character identity, world identity, name and availability only.
It does not publish level, vocation, house, guild, deletion state or game login
time. Public profile rendering and an eligible profile read API are pending.
The world UUID is shown without inventing a display name. Character profile
links use the existing `/characters/{name}` route contract.

The loyalty crate exposes an issuing snapshot operation that mutates authority;
this GET deliberately does not call it. No public read-only loyalty/title API or
account-termination reader is present. Those fields display localized
unavailability until the owning crates expose their read APIs. This page never
reads their private tables or invents an authoritative zero, title or date.

Security, premium and balance shortcuts follow the account-panel routes. Their
destination features retain their independent flags and implementation status.

The PostgreSQL test uses an isolated generated schema and covers active reads,
current and stale LCFA, suspended/missing accounts and private response headers.
When `PORTAL_TEST_DATABASE_URL` is unset it prints a skip notice.
