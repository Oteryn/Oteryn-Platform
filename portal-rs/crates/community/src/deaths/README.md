# Public deaths (T24D)

This module implements only the accepted G-4 deaths slice. The governing wire
source is `docs/contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md`
with the owner's accepted notes. The V1 family set and endpoints stay unchanged.
The accepted V2 enum contains deaths, guild, house and skills; this adapter accepts
only deaths publications and deaths watermarks. Other families need their own
adapters. No private family is exposed.

## Wiring

`configured_router` registers the internal V2 character-deaths and watermark POST
routes and `/[pl|en/]deaths`, `/[pl|en/]characters/{name}/deaths` GET routes.
The optional `world` query is an exact public world name, never a canonical ID.
The list shows the latest 50 visible records; character lists show at most 20
within 30 days, ordered by descending receipt time and occurrence ID.
Historical killer labels are plain text without identity or account resolution.
No cause is inferred from an empty killer list.

All activation requires `APP_ENV=testing|preproduction` and
`PORTAL_PUBLIC_PROJECTIONS_ENABLED=true`, a valid
`PORTAL_PUBLIC_PROJECTIONS_AUTHORITY`, and the existing purpose-specific
`PORTAL_PUBLIC_PROJECTIONS_SAN` certificate mapping. Independently:

- `PORTAL_DEATHS_V2_INGEST_ENABLED=true` enables ingestion.
- `PORTAL_DEATHS_ENABLED=true` enables presentation.
- `PORTAL_DEATHS_INGEST_REQUESTS_PER_MINUTE` is 1..6000 (default 6000).
  Invalid settings disable ingestion. The shared database budget is per route
  for the entire certificate group, stricter than per-certificate limits.

The binary deliberately supplies `FailClosed` until the profile privacy authority
is available. Enabling environment flags alone cannot disclose records. Consumers
of `router` must supply `Privacy`: `ready` proves the current privacy service is
available even for empty lists, and `allows_deaths` must enforce the stable
subject's current hidden/deaths switches and all relevant historical killer-label
privacy floors. Missing settings, unresolved identities and failed purge/fence
acknowledgements deny. A mock allow policy exists only in tests.

Presentation also requires eligible same-epoch T24 profile facts and fresh profile
and deaths watermarks (`now - complete_through + 1 <= 30`). Missing/stale evidence
returns 503 with no records; hidden/unknown/ambiguous character lookups return 404.
This follows the stricter G-4 refusal rule rather than the legacy world's 200
unavailable page. Disabled public routes return 404; disabled ingest returns 503.
Every non-200 ingest response has an empty body; oversize requests return 400.

## Storage and retention

V2 bodies are isolated from V1. Transactions lock T24's shared epoch fence so that
raising either feed fences all older families. Equal revision conflicts and
changes to immutable occurrence contents return 409 without replacing evidence.
Suppression replaces the body with an empty list. Only the current body is kept;
expired records are filtered immediately on ingest and read. The hourly worker
cleanup removes expired/fenced bodies while preserving minimal digest/replay
fences. No notifications or history archive are created.

Platform privacy changes must fence presentation through the privacy adapter;
the eventual privacy owner also owns its contract-required purge acknowledgement.
Production activation, Game V2 producer delivery and profile privacy integration
remain coordinator dependencies. This task does not enable them.

## Verification

The four PostgreSQL tests use isolated schemas and
`PORTAL_TEST_DATABASE_URL`; when unset they print a skip notice. They cover
replacement, replay, occurrence conflicts, suppression, rate refusal, shared epoch
fencing, privacy refusal, unknown/ambiguous names, stale evidence, public identity
redaction, retention and concurrent suppression fencing. Transport/schema tests also cover success and bad input.
No SQL query macros or offline query artifacts are added.

The presentation pool needs at least two connections: a shared T24 epoch-fence
lock prevents Game suppression from racing a multi-profile read while the public
T24 read API uses another connection. Smaller pools deny rather than deadlock.
