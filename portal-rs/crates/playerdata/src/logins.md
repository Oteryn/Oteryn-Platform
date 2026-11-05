# Game login history (T80)

Governing proposal: `docs/contracts/OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md`,
revision 2, sections 3–4, 5.12 and 6–9. The proposal remains pending acceptance.
The older plan's country/device fields and 180-day retention are superseded by the
closed four-field payload and 30-day maximum. No Game fixtures are copied.

## Dependency seam

T08 supplies the browser session middleware. T72 was absent from the base and no
remote dependency branch existed at task admission. This module therefore does
not implement a competing progress transport, peer guard or replay ledger.

T72 must authenticate and bind the private producer/source/partition, validate the
complete canonical G-11 envelope, enforce exact retry digests and sequence identity,
stage a contiguous family prefix, prove a complete baseline, process removals,
retain minimal replay fences and suspend/rebuild on restore before invoking this
module. Its accepted ingest transaction calls `Logins::apply` for a promoted
replacement, or `Logins::remove` for a validated tombstone. Any error rolls back
the whole transaction. Exact retries are answered by T72's replay ledger before
calling `apply`; equal or older entity revisions here are conflicts. No raw wire
bytes are stored here; encrypted raw replay storage belongs to T72.

An `Authority` adapter must independently revalidate Identity, accepted current
account/character ownership, visibility floor, world, transfer receipt cut,
projection epoch, recovery generation, trusted time and game_login baseline on
every operation. It must hold the corresponding authority locks in the supplied
transaction through commit. Never construct a `Cut` from browser identifiers,
login data, admission/reconnect state or a stale cached ownership projection.
Character revisions must meet the independently accepted transfer cut. Changed
ownership cuts deny history to both former and new owners by default.

The production composition uses `UnavailableAuthority`. Even setting both read
flags cannot activate it until the accepted T72 adapter is wired in. No T80 wire
endpoint is mounted; the owning T72 ingest surface must remain default-off.

## Routes and flags

Private GET `/account/game-logins`, `/pl/account/game-logins`, and
`/en/account/game-logins` use the public browser session layer. Account identity
comes from the authenticated session, with no requested account selector.
Responses are private/no-store. Anonymous actors are refused. Unknown authority
or disabled reads return unavailable, never an empty successful history.

All flags default false: `progress_ingest_enabled`, `progress_read_enabled`,
`game_login_history_enabled`. Binary read configuration uses
`OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED` and
`OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED`, requiring exactly `true`.
Ingest cannot be enabled by these settings. An enabled-family allowlist, accepted
purpose, source and epoch configuration remain T72 activation prerequisites.

An open login has unknown continuity. Stale history is explicitly labeled after
10 seconds; evidence older than 24 hours or uncertain by more than 5 seconds is
unavailable. Future observations are conservatively denied, including within the
proposal's possible five-second clock allowance. Expired login payloads are not
reinserted. Reads enforce 30 days, and the registered hourly retention job deletes
expired rows even when presentation is disabled. Account deletion cascades rows.

## Security action boundary

The native details disclosure “This was not me” / “To nie ja” explicitly reports
that no sessions or password were changed. T08 supports web-session revocation;
it does not prove game-wide revocation. No accepted public password-change API is
present in the dependency. Both actions are unavailable here. No invented game
command, direct password write, misleading success or fallback reset is used.
Later orchestration must call the owning accepted security APIs, with separate
default-off capability flags, CSRF, reauthentication and rate limits. Enabling
history must never enable those mutations.

The PostgreSQL tests use isolated schemas and `PORTAL_TEST_DATABASE_URL`, with a
printed skip notice when it is unset. They cover ownership and transfer denial,
canonical bad input, closure updates, retention, unknown authority and the real
browser middleware-to-SSR path. The page has no custom CSS or script: native
flow, contrast, focus, keyboard disclosure and wrapping work at 360 px.
