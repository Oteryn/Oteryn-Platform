# Player, content and guild reports

`OTERYN_REPORTS_ENABLED=true` enables the localized `/pl/support/reports` and
`/en/support/reports` browser routes; every other value leaves them absent.
The public `Reports` service enforces the same flag. Browser mutations use the
public web session and CSRF APIs; reporter IDs never come from form data.

The authoritative-contract search in `docs/contracts` found no reporting wire
contract. Behaviour follows the Laravel report request, controller and action:
three report types, type-specific categories, target limit 160 characters,
optional evidence limit 4000, per-account UUID request-key deduplication,
owner-only detail (404 for another owner), 302 browser redirects with localized
validation feedback, and three submission attempts per minute. The database
throttle counts invalid submissions and retries too. Requests failing session or
CSRF checks do not enter the domain service. All private responses disable caching.

The plan adds a 30-day paginated history, at most 20 pending reports, and a block
after repeated unjustified reports. `OTERYN_REPORTS_PENDING_LIMIT` defaults to the
reference's stricter 5 and accepts 1–20. `OTERYN_REPORTS_UNJUSTIFIED_LIMIT` defaults
to 3 rejections in 30 days. Invalid configuration denies routing and service
access. Rejection timestamps survive closure. The threshold is a configurable
local policy because the plan supplies no exact rejection threshold or duration.

T33's branch was unavailable at implementation time. The narrow dependency seam
uses only `identity::AccountId` and the public browser-session API. There is no
dependency on ticket storage or game data. Targets are evidence references, not
claims of verified game ownership or existence. No game repository access or
fixtures are needed. Team moderation belongs to T51: storage includes private
notes, assignee, public outcome and lock version, but the reporter projection
selects only public-safe fields. This task exposes no moderation mutation.

Tests read `PORTAL_TEST_DATABASE_URL`; when absent, every PostgreSQL test prints a
skip notice. Five isolated PostgreSQL tests cover browser sessions and CSRF,
submission, privacy, limits, concurrency, retries, rejection history, bad input,
and rolling history. Query execution uses bound parameters without query macros,
so no additional offline query metadata is needed.
