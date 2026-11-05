# World daily consumer

T88 implements the accepted G-12 revision 2 wire profile. It adds no Game client,
mutation, scoring or T94 outcome source. The content crate's existing Today page
is extended through appended registration; disabled public serving preserves its
response unchanged.

Activation is limited to `APP_ENV=testing` or `preproduction`. All flags default
to false. Ingest additionally requires `PORTAL_PUBLIC_PROJECTIONS_ENABLED=true`
and `PORTAL_WORLD_DAILY_INGEST_ENABLED=true`; serving requires
`PORTAL_WORLD_DAILY_PUBLIC_ENABLED=true`. Configure the existing
`PORTAL_PUBLIC_PROJECTIONS_AUTHORITY` and `PORTAL_PUBLIC_PROJECTIONS_SAN`, plus
`PORTAL_WORLD_DAILY_WORLDS` as a comma-separated list of exact registered UUIDv7
World IDs (no whitespace or duplicates). The trusted private TLS adapter must
supply the verified certificate extension; headers cannot authenticate requests.
No listener, certificate or production activation is introduced here.

Snapshots and watermarks use `/internal/v2/game-public/world-daily` and
`/internal/v2/game-public/watermark`. The watermark route accepts only the new
`world_daily` family; the existing v1 family enum stays closed. The shared JSON
decoder's depth bound is exclusive, so a value of six permits the contract's
five container levels. Snapshot and watermark limits are 131072 and 512 bytes;
all oversize failures return empty 400. The authenticated publisher has bounded
rolling-minute limits of 120 snapshots and 60 watermarks per process.

`/pl/today` and `/en/today` retain editorial cards and gain current boosts,
coverage-bounded history and published fixed-slot raids. `?world=<WorldId>`
selects an allowlisted World; omission works only with exactly one configured
World. Localized public page headers with a header element gain the same stored
boost widget. `/pl/today/widget` and `/en/today/widget` expose HTML fragments;
`/pl/today/data` and `/en/today/data` expose only presentation fields. Root
`/today` variants default to Polish and support `lang=en`. Every representation
is no-store. The data endpoint returns 503 for unavailable evidence and 400 for
invalid query input.

Serving checks the global public recovery epoch, a complete configured-World
baseline, the 30-second completeness window with one second of clock uncertainty,
and the current half-open reset interval. The whole uncertain clock interval
must fit inside it. Catalogue resolution joins the active ready content version,
`publishable`, the current independent visibility allowlist, and both-language
admin names. Boost keys resolve as `Creature`; raid keys as `Raid`. Missing,
hidden, inactive or unresolved content makes the daily view unavailable; keys
are never treated as display names. Empty raids describe only the published
fixed slots, never possible random or future raids.

Snapshot identity is authority/World/epoch/revision. Exact content is compared on
retry, ignoring a changed envelope timestamp and retaining its original value.
Conflicting content commits a quarantine before 409; subsequent publications do
not silently clear it. Reconciliation remains an authorized operator action,
with no public clearing endpoint. Epoch raises use the existing cross-family
public fence and make older generations unavailable. Withdrawals replace the
whole view and prevent current facts or history from being served.

The worker expires stored content after 30 days, retaining only minimal replay
and conflict fences and a SHA-256 content identity. Conflicting payloads and raw
envelopes are never archived. Reads independently deny expired content between
cleanup runs. No cache, prior payload or alternate source can restore visibility.

Run `cargo test -p oteryn-community --test today_contract -- --nocapture` with
`PORTAL_TEST_DATABASE_URL` set to a disposable PostgreSQL database. Each database
test uses and drops its own schema. Without that variable each prints a skip
notice. No sqlx query macros are introduced.
