# Support route coverage

Source baseline `7847cf7de91957d4317e7693d31d049b8f1c5f8e`. The portal binary composes the browser router from `oteryn_web::router` and merges the support routers. Player reports mount at [main.rs:320](../../../../bin/oteryn-portal/src/main.rs#L320), tickets and their localized staff routes at [main.rs:354](../../../../bin/oteryn-portal/src/main.rs#L354), and the staff queues at [main.rs:376](../../../../bin/oteryn-portal/src/main.rs#L376). The [account navigation](../../../../crates/support/src/navigation.rs) follows the player flags. The sanction export mounts after the public listener has been captured. Game enforcement mounts on the internal mTLS listener. [main.rs:517](../../../../bin/oteryn-portal/src/main.rs#L517) · [game_routes.rs:367](../../../../bin/oteryn-portal/src/game_routes.rs#L367)

## Actors and flags

- **Player paths** use `pl` or `en`, reject unsigned browser mutations through the session CSRF guard, and scope records to their owner. Tickets require an active session and `OTERYN_SUPPORT_TICKETS=true`. Reports accept active or pending-verification sessions and require `OTERYN_REPORTS_ENABLED=true`, pending limit 1–20 (default 5), and unjustified limit above 0 (default 3). The outer Chrome layer redirects anonymous GETs to the localized login even when the underlying route is unavailable. [tickets/web.rs:66](../../../../crates/support/src/tickets/web.rs#L66) · [reports/web.rs:21](../../../../crates/support/src/reports/web.rs#L21) · [reports.rs:154](../../../../crates/support/src/reports.rs#L154) · [chrome.rs:32](../../../../crates/web/src/chrome.rs#L32)
- **Staff paths** require `OTERYN_ADMIN_PANEL=true`, an exact `OTERYN_ADMIN_HOST`, a verified staff MFA session, `admin.access`, and the area's permission. Support and platform-admin have `support.tickets.manage` and `support.reports.manage`. Moderator and platform-admin have `support.enforcement.manage`. Support and moderator are each denied in the other's area. Security-admin, content-editor, operations and game roles lack all three support permissions. Staff ticket and report queues also depend on the corresponding feature flags. [catalog.rs:313](../../../../crates/admin/src/rbac/catalog.rs#L313) · [web.rs:154](../../../../crates/admin/src/web.rs#L154) · [web.rs:294](../../../../crates/admin/src/web.rs#L294) · [support_web.rs:50](../../../../crates/admin/src/support_web.rs#L50)
- **Internal paths** require the appropriate service identity in addition to the flag. The sanction stream uses `PORTAL_SANCTION_EXPORT_ENABLED` and `support-sanction-export`. Game enforcement uses `PORTAL_GAME_ENFORCEMENT_ENABLED`, valid profile/budget configuration and distinct reader/result identities. Restore-notice mode disables enforcement results. These listeners have no portal player or staff role. [export.rs:22](../../../../crates/support/src/sanctions/export.rs#L22) · [enforcement.rs:29](../../../../crates/support/src/sanctions/enforcement.rs#L29) · [enforcement.rs:174](../../../../crates/support/src/sanctions/enforcement.rs#L174) · [game_routes.rs:367](../../../../bin/oteryn-portal/src/game_routes.rs#L367)
- **Player appeals and sanction history** are domain services. Sanction history, acknowledgment, and appeal submission have no player HTTP route. Staff decisions on an appeal require an existing requested or reviewing workflow. Creating a sanction initializes the workflow in state `none`. [appeals.rs:280](../../../../crates/support/src/appeals.rs#L280) · [sanctions/store.rs:600](../../../../crates/support/src/sanctions/store.rs#L600)

## Form and query contracts

`{id}`, `{after}`, `reply`, and `request_key` are UUIDs when present. Every browser mutation also requires `_csrf`. The localized player and staff ticket status routes share the POST and PUT handlers. The two staff ticket surfaces use different route and form shapes.

| Contract | Fields and validation |
| --- | --- |
| Ticket list and detail | Optional `page` is a zero-based unsigned integer. Optional detail `after` is a message UUID. |
| Create ticket | `request_key`, `category` (account, character, technical, payment, other), `subject` (1–160), `body` (1–8000). Same key and input return the original ticket. Open capacity is 10. |
| Acknowledge ticket reply | `reply`, optional `after`. Clears the unread marker only for the latest displayed public staff reply. Repeat succeeds without changing the state again. |
| Localized ticket reply | `lock_version`, `body` (1–8000), optional `internal` (staff checkbox). Player reply requires open or waiting-user state. Staff reply requires an active ticket. |
| Localized ticket status | `lock_version`, `status`. Player can close or reopen a closed ticket. Staff can also use waiting-user, waiting-staff and resolved. |
| Report list and detail | Optional `page` is a one-based unsigned integer. Detail is owner-scoped by UUID. |
| Submit report | `request_key`, `report_type` (player, content, guild), a matching `category`, `target_reference` (1–160), optional `evidence_summary` (up to 4000). Three attempts per account per minute. Identical replay returns the same record. Invalid fields redirect to the form with a localized message. |
| Staff queues | Optional list `page` 1–10000, optional detail `after`. Mutations include `lock_version` and `action`. |
| Staff ticket queue mutation | `action=reply` with `body` and optional `internal`, or `action=ticket_status` with `status`. |
| Staff report mutation | `action=moderate`, `status`, `public_outcome`, `moderator_notes`. Public outcome required for actioned, rejected or closed. |
| Staff sanction mutation | `action=sanction`, `create`, `target` (account UUID), `category` (warning, restriction, suspension), `status` (active, expired, revoked), `public_reason`, `moderator_notes`, `effective_at`, `expires_at` (UTC epoch seconds). The create page generates its own action UUID. |
| Staff appeal mutation | `action=appeal`, `appeal_status` (reviewing, accepted, rejected), `appeal_outcome` (required for a decision). |
| Internal sanction / command stream and reconciliation | JSON `schema`, `after` (null or versioned cursor), `limit` (1–500). Each path has its own schema. |
| Internal command read | JSON `schema`, `operation_id`. |
| Internal enforcement result | Canonical JSON `schema`, `operation_id`, `sanction_stream_id`, `command_digest`, `receipt_revision`, `state`, `result_code`, `accepted_decision_revision`, `stream_state`, `effective_until_ms`, `runtime_effect`, `capability_revision`, `recovery_generation`, `decided_at_ms`. |

Form and handler references. [tickets/page.html](../../../../crates/support/src/tickets/page.html) · [tickets/web.rs:473](../../../../crates/support/src/tickets/web.rs#L473) · [tickets/mod.rs:70](../../../../crates/support/src/tickets/mod.rs#L70) · [reports/web.rs:91](../../../../crates/support/src/reports/web.rs#L91) · [reports.rs:55](../../../../crates/support/src/reports.rs#L55) · [support_web.rs:325](../../../../crates/admin/src/support_web.rs#L325) · [support.rs:28](../../../../crates/admin/src/support.rs#L28) · [enforcement.rs:455](../../../../crates/support/src/sanctions/enforcement.rs#L455) · [export.rs:100](../../../../crates/support/src/sanctions/export.rs#L100)

## Staging observations

On the public staging origin, both player route probes and all six anonymous support mutations returned 404. Anonymous GETs redirected to sign-in via Chrome. Every staff route returned 404 on the supplied origin. That status is consistent with a disabled admin panel or a different configured admin host. The public tests cannot distinguish those configurations. The internal routes returned 404 on the public origin as expected. Mailpit returned 404 and registration independently failed with 503. No staff session or player record was available for an authorized queue action, ownership check, or cross-account persistence assertion.

## Route × actor matrix

**A** = anonymous player support navigation and form refusal. **B** = public origin refuses staff and internal support routes. **T** = ticket creation, follow-up, closing and reopening. **R** = report draft, submission and owner history. The two authenticated player scenarios are present but skipped by the deployment prerequisite checks. A staff actor on the configured host would first require a staff MFA session. `Other` means another fresh player for an existing owner record.

| Method and path | Actor | Expected from source | Observed on supplied staging origin | Test |
| --- | --- | --- | --- | --- |
| GET `/{lang}/support/tickets` | Anonymous / active player | 303 localized login / 200 own list when mounted | 303 / player blocked | A, T |
| GET `/{lang}/support/tickets/create` | Anonymous / active player | 303 / 200 create form when mounted | 303 / player blocked | A, T |
| GET `/{lang}/support/tickets/{id}` | Anonymous / owner / other | 303 / 200 / 404 | 303 / owner and other blocked | A, T |
| POST `/{lang}/support/tickets` | Anonymous / active player | 403 without token / 302 on create or replay, 422 invalid / 404 if unmounted | 404 / player blocked | A, T |
| POST `/{lang}/support/tickets/{id}/read` | Anonymous / owner / other | 403 without token / 302 on valid repeat, invalid field 422 / 404 / 404 if unmounted | 404 / owner and other blocked | A, T |
| POST `/{lang}/support/tickets/{id}/reply` | Anonymous / owner / other | 403 without token / 302 or invalid 422, stale 409 / 404 / 404 if unmounted | 404 / owner and other blocked | A, T |
| POST `/{lang}/support/tickets/{id}/status` | Anonymous / owner / other | 403 without token / 302 or invalid 422, stale 409 / 404 / 404 if unmounted | 404 / owner and other blocked | A, T |
| PUT `/{lang}/support/tickets/{id}/status` | Anonymous / owner / other | 403 without token / 302 or invalid 422, stale 409 / 404 / 404 if unmounted | 404 / owner and other blocked | A, T |
| GET `/{lang}/support/reports` | Anonymous / player | 303 localized login / 200 own recent list when mounted | 303 / player blocked | A, R |
| GET `/{lang}/support/reports/create` | Anonymous / player | 303 / 200 create form when mounted | 303 / player blocked | A, R |
| GET `/{lang}/support/reports/{id}` | Anonymous / owner / other | 303 / 200 / 404 | 303 / owner and other blocked | A, R |
| POST `/{lang}/support/reports` | Anonymous / player | 403 without token / 302 on submit, replay or validation redirect / 404 if unmounted | 404 / player blocked | A, R |
| GET `/{lang}/admin/support/tickets` | Anonymous / player or moderator / support or platform-admin | 404 at host gate, otherwise 303 / 403 / 200 | 404 / authenticated roles blocked | B |
| GET `/{lang}/admin/support/tickets/{id}` | Anonymous / player or moderator / support or platform-admin | 404 at host gate, otherwise 303 / 403 / 200 | 404 / authenticated roles blocked | B |
| POST `/{lang}/admin/support/tickets/{id}/reply` | Anonymous / player or moderator / support or platform-admin | 404 at host gate, otherwise 403 without CSRF / 403 / 302 valid or 409 stale, 422 invalid | 404 / authenticated roles blocked | B |
| POST `/{lang}/admin/support/tickets/{id}/status` | Anonymous / player or moderator / support or platform-admin | 404 at host gate, otherwise 403 without CSRF / 403 / 302 valid or 409 stale, 422 invalid | 404 / authenticated roles blocked | B |
| PUT `/{lang}/admin/support/tickets/{id}/status` | Anonymous / player or moderator / support or platform-admin | 404 at host gate, otherwise 403 without CSRF / 403 / 302 valid or 409 stale, 422 invalid | 404 / authenticated roles blocked | B |
| GET `/admin/support/tickets` | Anonymous / player or moderator / support or platform-admin | 404 at host gate, otherwise 303 / 403 / 200 | 404 / authenticated roles blocked | B |
| GET `/admin/support/tickets/{id}` | Anonymous / player or moderator / support or platform-admin | 404 at host gate, otherwise 303 / 403 / 200 | 404 / authenticated roles blocked | B |
| POST `/admin/support/tickets/{id}` | Anonymous / player or moderator / support or platform-admin | 404 at host gate, otherwise 403 without CSRF / 403 / 303 valid or 409 stale, 422 invalid | 404 / authenticated roles blocked | B |
| GET `/admin/moderation/reports` | Anonymous / player or moderator / support or platform-admin | 404 at host gate, otherwise 303 / 403 / 200 | 404 / authenticated roles blocked | B |
| GET `/admin/moderation/reports/{id}` | Anonymous / player or moderator / support or platform-admin | 404 at host gate, otherwise 303 / 403 / 200 | 404 / authenticated roles blocked | B |
| POST `/admin/moderation/reports/{id}` | Anonymous / player or moderator / support or platform-admin | 404 at host gate, otherwise 403 without CSRF / 403 / 303 valid or 409 stale, 422 invalid | 404 / authenticated roles blocked | B |
| GET `/admin/moderation/enforcement` | Anonymous / player or support / moderator or platform-admin | 404 at host gate, otherwise 303 / 403 / 200 | 404 / authenticated roles blocked | B |
| GET `/admin/moderation/enforcement/create` | Anonymous / player or support / moderator or platform-admin | 404 at host gate, otherwise 303 / 403 / 200 with generated UUID | 404 / authenticated roles blocked | B |
| GET `/admin/moderation/enforcement/{id}` | Anonymous / player or support / moderator or platform-admin | 404 at host gate, otherwise 303 / 403 / 200 | 404 / authenticated roles blocked | B |
| POST `/admin/moderation/enforcement/{id}` | Anonymous / player or support / moderator or platform-admin | 404 at host gate, otherwise 403 without CSRF / 403 / 303 valid or 409 stale or invalid state, 422 invalid | 404 / authenticated roles blocked | B |
| POST `/internal/v1/support/sanctions/stream/read` | Public browser / export service | 404 on public listener / 200 on authorized internal listener | 404 / service outside staging browser access | B |
| POST `/internal/v1/support/enforcement-commands/stream/read` | Public browser / reader service | 404 on public listener / 200 on authorized internal listener | 404 / service outside staging browser access | B |
| POST `/internal/v1/support/enforcement-commands/commands/read` | Public browser / reader service | 404 on public listener / 200 on authorized internal listener for existing command | 404 / service outside staging browser access | B |
| POST `/internal/v1/support/enforcement-commands/reconcile/read` | Public browser / reader service | 404 on public listener / 200 on authorized internal listener | 404 / service outside staging browser access | B |
| POST `/internal/v1/support/enforcement-commands/results` | Public browser / result service | 404 on public listener / 200 for accepted result on authorized internal listener | 404 / service outside staging browser access | B |
| GET `/{lang}/support/appeals` and `/{lang}/support/sanctions` | Public browser | 404, no player HTTP mount | 404 | B |
