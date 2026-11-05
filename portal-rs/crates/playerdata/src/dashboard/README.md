# Dashboard and reminders

T79 consumes G-9 closed-session summaries and G-11 timer observations. G-10 is
not a timer authority. Bazaar deadlines come from Bazaar, premium reminders also
require independently current commercial authority. No game command is introduced.

The registered `/pl/account/dashboard` and `/en/account/dashboard` routes return
empty 503 responses by default. The injected router requires a session-derived
`Actor`; no browser account selector or header can supply it. Staff access is
refused. The HTML uses the existing surface stylesheet, semantic headings/lists,
escaped text, Polish/English Fluent messages, and the responsive viewport.

## Dependency seams and activation blockers

The T72 branch was absent at task admission. `Source` accepts only already
validated, complete, fenced snapshots and independently current receipt cuts.
`Unavailable` is the production adapter until that implementation is integrated.
Source adapters must enforce stream allowlists, source/catalog identities, clock
confidence, complete baselines, removal, erasure and transfer suppression. Data
are historical observations; online observations never claim live presence.
There is no duplicate private ingest protocol in T79.

T36 is merged. Its public notification categories are security/news/support and
its worker delivers SMTP and portal notifications. It has no reminder category,
reminder consent or web-push API. Reusing another category would bypass consent.
`Authority` and `Delivery` therefore remain explicit, unavailable adapters. A
future delivery adapter must enqueue under the stable intent UUID using the
supplied transaction, and revalidate source evidence, flags, current account,
ownership, deletion and channel opt-in immediately before external delivery.
The external worker must deduplicate delivery retries by that UUID. `delivered`
means durably handed to this adapter, not confirmed by a remote mail/push server.
No endpoint, subscription token, recipient or credential is persisted here.
Production enablement requires those integrations and their actor-to-delivery tests.

Fail-closed interpretations of revision 2:

- House rent has no accepted closed payload and is always unavailable.
- Full stamina requires an observed 2520 minutes. No refill forecast is invented.
- Premium observations never grant entitlement; disagreements suppress delivery.
  Microseconds convert with ceiling to milliseconds; unknown deadlines stay unknown.
- Only evidence no older than ten seconds is usable; future observations are denied.
  Trusted-clock uncertainty above five seconds, incomplete baselines or any changed
  ownership cut suppress delivery and cancel pending work at dispatch.
- Pre-transfer histories cannot authorize reminders for either owner. The exact
  receipt cut includes character/projection revisions, epoch and recovery generation.
- Until Game accepts producer mappings, no ingest or game command is enabled.

`Reminders` persists only bounded intents. Schedule is idempotent for owner/kind/
entity/channel/deadline. Dispatch locks the row, checks its entire binding and
revalidates evidence before atomically handing off to delivery. Cancellation is
owner-only and remains possible with feature flags disabled. There is no browser
mutation route until session/CSRF and preference integration is available.

Tests include PostgreSQL scheduling, retry, delivery, cancellation, invalid input
and cross-account denial. They read `PORTAL_TEST_DATABASE_URL`, print a skip notice
when absent, and use an isolated schema. No sqlx query macros are used.
