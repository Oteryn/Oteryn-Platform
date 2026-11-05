# Character statistics

T66 uses the public T65 `store::history` API. Public routes:

- `/{pl|en}/stats/characters/{id}/{history|experience|rankings|deaths|progress|online}`
- `/{pl|en}/stats/compare?ids=<2–5 comma-separated UUIDs>&metric=level`

Queries accept canonical UTC `start` and `end` dates, at most 366 inclusive
observations, defaulting to the last 30 days. Comparison supports experience and
the eight G-4 skill keys. A single-character request may include a positive
`target` level. Duplicate/unknown fields, malformed dates, oversize queries,
duplicate comparison subjects and invalid metrics return 400. Unknown/hidden
subjects return 404; failed or ambiguous dependencies return 503. Responses are
private and never stored by caches. Collections use stable date and ID ordering.
Charts plot observations only, with exact values in adjacent accessible cards;
gaps are never interpolated. Daily/weekly/monthly gains require exact endpoints.
The displayed record is limited to the requested interval, and losses stay signed.

`PORTAL_CHARACTER_STATS_ENABLED` is off by default. The configured router retains
a denying adapter even if the flag is set: T24b is absent on origin and main has
no public current T45 profile/privacy API. Integrators must inject `Current`
through `router(Characters)` with eligible, fresh, same-epoch G-4 profile and
privacy evidence. The contract is intentionally narrow; browser IDs confer no
authority. Permissions are checked again after storage reads. Comparison refuses
all output when any subject is hidden or unavailable.

Rank history requires retained public rank observations from the adapter, not
inferred historical ranks from a current population. No producer is available
on main; `None` renders unavailable. XP prediction requires the adapter's accepted
world-specific level threshold and eight consecutive daily observations ending
on the selected date. Positive seven-day net growth is extrapolated with integer
ceiling division; estimates beyond ten years are unavailable. No game formula is
invented here.

Progress and online time remain explicitly unavailable. Public achievement
points are stripped because G-4 facts may be account-scoped. No earned list,
equipment, outfit, private quest facts, raw participant JSON, source identities
or account associations are rendered. Online is excluded from navigation and no
G-8 intervals or rejected activity family are consumed. Death observations are
restricted to the G-4 rolling 30-day window; retained observations do not claim a
complete ledger. Event rendering uses a closed field allowlist. Guild/world/sale
observations carry labels without exposing opaque source identifiers.

References: plan sections 4, 5.7, 6, 7, 8 and T66; G-4 public families contract;
Laravel PublicGameDataController::character for 404/503 behavior. No external
Game fixtures were copied. PostgreSQL tests use an isolated schema and skip with
a notice only when PORTAL_TEST_DATABASE_URL is unset.
