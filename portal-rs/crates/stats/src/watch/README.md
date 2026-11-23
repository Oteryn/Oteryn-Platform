# Private watches

`PORTAL_STATS_WATCHES_ENABLED=true` opts into routing. Default: disabled (404).
Verified players can issue a session-bound bearer at `/{lang}/account/stats-watches`.
It is scoped to the watch API, expires within one hour, and is replaced by issue
or invalidated by revoke or session rotation. The qualified T66 source adapter
is still required for list, subscribe and fanout. See
[readiness](../../../../../docs/product/GAP-36_STATISTICS_QUALIFICATION.md).

Bearer-only `/api/v1/stats/watches` supports GET, POST and DELETE. Cookies do
not authorize requests. POST and DELETE accept exactly `target_kind`
(`character` or `guild`), `target_id` (non-nil UUID) and `event_kind` (`online`,
`level_up`, `death`, `guild_change`). IDs stay inside the authenticated private
surface; this API does not expose public lookups or URLs by canonical identity.
An adapter must resolve accepted canonical identities, never mutable names.
No public watch counters, follower lists or target existence errors are exposed.

Successful mutations return 204 and are idempotent. Missing/invalid bearer:
401; malformed JSON or invalid ID: 422; denied/unknown target or unavailable
policy/storage: 503 with no details; body ceiling: 1024 bytes. GET returns only
subscriptions passing current disclosure policy. All responses use private,
no-store caching. Requests have a database-backed 60/minute/account budget
(429), and subscription creation has a serialized 100/account ceiling.
Removal remains possible after the subject becomes hidden. The credential page
uses the standard browser session and CSRF middleware.

`Policy` is the narrow integration seam for absent T66: it validates the bearer,
active account, scope, current source and privacy generations, event-time and
current per-field disclosure, and accepted guild identity/membership. Unknown
state denies. The policy must hold the applicable privacy fence in the supplied
transaction through notification commit. Guild subscription permission alone
never authorizes a member's event. The configured API supports owner-scoped removal while source qualification is unavailable.

An accepted projection consumer may call `Watches::dispatch` with an immutable
source event ID. This method is intentionally not an HTTP ingest endpoint and
is not connected to unaccepted Game producers. It selects matching subscriptions,
rechecks policy, deduplicates per recipient/event across character and guild
subscriptions, and atomically commits its receipt with T36 `Notifications::enqueue`.
Retries after rollback are safe. Existing T36 delivery handles email outbox jobs.

T36 currently exposes news, security and support categories. Watches use the
optional news preferences and never security's mandatory delivery. Messages
use Fluent Polish/English resources and contain only a generic watch-list
reminder, without target identifiers, names, event type, event details or deep
links. This durable content remains safe after a later privacy restriction;
current subject information is read only through the policy-filtered API.
A dedicated watch preference/category requires an extension of T36's public API.

No matching Laravel character/guild-watch route or authoritative watch HTTP
contract exists. Marketplace watches are a separate mechanic. Privacy is governed
by `OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md` (independent privacy
revision/cutoff, no stale allow resurrection) and plan §5.7. G-8 online-source
acceptance and canonical guild authority remain gated by their owning contracts;
no Game fixture or producer has been invented here.

Tests: `PORTAL_TEST_DATABASE_URL` selects isolated PostgreSQL schemas. With it
unset, the database tests print a skip notice. No query macros were added.
