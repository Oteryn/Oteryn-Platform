# Game staff staging simulation

The suite targets `https://staging.oteryn.com` with Chromium and two workers. It covers Polish and English at 1440px and 390px. It uses the public browser flows and authenticated HTTP reads. Local test artifacts are written under `/tmp/oteryn-sim-game-staff/`.

## Expected role and page matrix

This matrix comes from the permission migrations and page handlers. It assumes the panel and feature routers are enabled on the staging host, with Game disconnected. **A** is an allowed page, **R** is HTTP 403, and **U** is a rendered unavailable state. **U/R** is an unavailable landing page with the operation link withheld and an agent selection refused with HTTP 403. Staff authentication was blocked by the password-reset defect below, so the live cells remain unverified.

| Role | Staff home | Dashboard | Worlds | Nodes | Database | Player lookup | Audit | Analytics | Platform controls |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| game-admin | A | U | U | U | U | R | U | U | R |
| game-analyst | A | U/R | U/R | U/R | U/R | R | R | U | R |
| game-viewer | A | U | U | U | U | R | U | U | R |
| game-support | A | U | U | U | U | U | U | U | R |
| game-operator | A | U | U | U | U | R | U | U | R |
| game-master-senior | A | U | U | U | U | U | U | U | R |
| game-release | A | U/R | U/R | U/R | U/R | R | R | R | R |
| game-security | A | U/R | U/R | U/R | U/R | R | U | U | R |
| game-dba | A | U/R | U/R | U/R | U | R | R | R | R |

- All nine roles get `admin.access`. Each operation uses its own grant. The Ops console checks the operation grant for filtered requests, and the page handler renders an unavailable state when the Game agent is absent. [Game role grants](../../../../migrations/20261105231403_adm-p2a_game_roles.sql#L69) · [Common staff access](../../../../migrations/20261122110000_game_staff_admin_access.sql#L2) · [Ops handler](../../../../crates/admin/src/console/mod.rs#L550)
- Player lookup is `/admin/console/players`. It uses `game.player.read` for these roles and renders the dossier, inventory and sanctions as unavailable. The prefix lookup form requires an advertised Game operation. Audit is `/admin/console/audit` and uses `game.audit.read` for these roles. [Player lookup](../../../../crates/admin/src/console/players.rs#L142) · [Audit](../../../../crates/admin/src/console/audit.rs#L67)
- Analytics covers overview, creatures, activity, levels, feed health and economy, plus their corresponding `/data` routes. Game staff with `game.analytics.read` get Game availability states and no Platform wallet totals. [Analytics routes and authorization](../../../../crates/admin/src/analytics/web.rs#L119) · [Economy output](../../../../crates/admin/src/analytics/economy.rs#L140)
- Platform controls cover roles, access, approvals, Platform players, tickets, moderation, CMS and commerce. The world catalog and Bazaar tools also have feature flags that can unmount the route on staging. [Access and approvals](../../../../crates/admin/src/console/access.rs#L430) · [Platform scope tests](../../../../bin/oteryn-portal/src/flag_boundary_tests.rs#L732) · [World catalog](../../../../crates/world/src/admin.rs#L30) · [Bazaar](../../../../crates/admin/src/bazaar.rs#L137)

## Staging observations

- Mailpit returned 404 on the initial check at 11:34 UTC and at 10-minute intervals through 12:04 UTC. Staging then returned Cloudflare 502s on `/readyz` and registration. The interrupted run also sampled registration GETs returning 503 during recovery. At 12:09 UTC both `/readyz` and Mailpit returned HTTP 200. Registration GETs subsequently returned 200 again in both languages. The suite was rerun after recovery.
- The guest requests to `/admin` and `/admin/analytics` returned HTTP 404 before and after recovery. The outer staff host gate can return 404 before examining the session. Authenticated role access and Game availability require a completed staff login. [Host guard](../../../../crates/admin/src/web.rs#L294)
- The first complete 48-case run passed the four guest checks. The initial Mailpit outage blocked 40 cases and shared rate limits blocked four registration cases after waiting.
- After recovery, the 48-case run had four passes, 42 failures and two skips. Focused reruns of both skipped `pl-1440` cases reproduced the same two defects. Across the recovered run and follow-ups, all four guest cases passed, all eight player cases failed at valid registration, and all 36 staff cases failed at valid password reset. This blocked player email verification, staff login and MFA, and the live Game page and permissions comparison.

## Product defects

1. **Valid player registration returns HTTP 503.** Open `/pl/register` or `/en/register` as a guest, enter a new address and matching valid passwords, accept both legal editions and submit. The expected outcome is a 303 to the localized check-inbox page. Staging returned 503 with the localized registration-unavailable message while `/readyz` was 200. The form had previously returned 422 and displayed the expected message for mismatched passwords. This reproduced in both languages at both widths. [Registration success handler](../../../../crates/web/src/routes/register.rs#L142) · [Registration availability response](../../../../crates/identity/src/abuse/http.rs#L41)

   Local screenshot `/tmp/oteryn-sim-game-staff/3ce517d8-4279-49a2-87af-2769225c5e85/player-player-registration-input-validation-and-check-inbox-pl-1440/registration-check-inbox.png`

2. **Valid staff password reset returns HTTP 503.** Request a reset for a named seeded staff account. Open that account's newest reset email, submit mismatched passwords, then correct the confirmation and resubmit the form, observed on `POST /en/reset-password`. The expected outcome is a 302 to the localized login page. The first submission produced the localized refusal as expected. Staging returned a generic 503 for the valid submission and the flow could not reach login. This reproduced for all nine roles on all four projects. [Password-reset completion handler](../../../../crates/identity/src/reset_http.rs#L330) · [Reset form](../../../../templates/account/reset/page.html#L7)

   Local screenshot `/tmp/oteryn-sim-game-staff/82d285eb-00a2-4cec-9df1-0eb1340d7f5b/staff-game-admin-staff-navigation-pages-and-filters-pl-1440/staff-reset-unavailable.png`

## Test bugs fixed during qualification

- The validation response renders both consent checkboxes unchecked. The corrected registration flow accepts both again before submitting the corrected password and keeps the original validation and success assertions. [Registration template](../../../../templates/account/register.html#L14)
- A screenshot raced a navigation and obscured the original assertion. Capture now waits for the document and keeps the assertion result even if capture fails.
- One qualification run loaded helpers while they were being edited. A fresh worker run used the complete updated suite.
- A late retry after shared rate limiting left under two seconds for the browser to complete a registration submission. The retry now reserves at least 20 seconds for a response.
- The reset mailer uses the account's stored language. The suite now checks both localized reset subjects and performs the reset in the link's language, then signs in using the project language. The mail poll also handles transient transport errors within its 60-second budget. [Reset mail delivery](../../../../crates/identity/src/reset_delivery.rs#L103)
