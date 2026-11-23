# Security and operations staging simulation

The suite uses Chromium against `https://staging.oteryn.com` in Polish and English at 1440px and 390px. Each test checks readiness first. It uses public registration and recovery forms, with Mailpit reads limited to the scenario's own recipients. The two named staff identities are shared across projects, so staff sign-in runs serially while other tests can use both workers.

Run from `portal-rs/e2e`.

```sh
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts
npx playwright test -c staging/sim/security-ops/playwright.config.mjs
```

## Playwright results

| Scenario | pl-1440 | pl-390 | en-1440 | en-390 |
| --- | --- | --- | --- | --- |
| Health and readiness | Pass | Pass | Pass | Pass |
| Read-only download origin, stable unavailable | Pass | Pass | Pass | Pass |
| Registration validation, language and back navigation | Blocked | Pass | Pass | Pass |
| Signed-out session and invalid verification link | Pass | Pass | Pass | Pass |
| Password recovery confirmation and invalid reset | Pass | Pass | Pass | Pass |
| Signed-out staff panels, unavailable | Pass | Pass | Pass | Pass |
| Player session revocation across browsers | Blocked | Blocked | Blocked | Blocked |
| Staff roles, audit and catalog | Blocked | Blocked | Blocked | Blocked |

The full run recorded 23 passed, 9 skipped and 0 failed. A focused Polish desktop registration recheck reached the shared rate limit again.

## Staging limitations

- Staff pages on the requested hostname returned HTTP 404, including `/admin`, `/admin/players`, `/admin/console/audit`, `/admin/moderation/enforcement` and `/admin/world`. This blocks staff sign-in verification, role separation, catalog panel actions and audit observations. The runtime gates these routes by staff hostname and feature configuration. [Staff host guard](../../../../crates/admin/src/web.rs#L294) · [Player tools configuration](../../../../crates/admin/src/players/http.rs#L21) · [World catalog configuration](../../../../crates/world/src/admin.rs#L31)
- The requested Mailpit listing returned HTTP 404 initially and at the 10- and 20-minute rechecks. The 20-minute check also returned HTTP 200 for `/readyz` and HTTP 404 for `/admin`. The checked-in staging ingress sends requests to the portal and the staging stack defines Mailpit as a separate service. The checked-in deploy path includes a player seed. Staff account availability could not be established through password recovery while delivery was inaccessible. [Staging ingress](../../../../../deploy/oracle/staging.compose.yml#L67) · [Staging seed](../../../../../deploy/oracle/staging-deploy.py#L361)
- Fresh player registration returned a localized HTTP 503 unavailable response on Polish desktop. The other three projects reached a repeated shared registration rate limit during the full run. Player email verification and the authenticated session scenario could not proceed. [Registration availability response](../../../../crates/identity/src/abuse/http.rs#L41)
- Stable update metadata and policy returned HTTP 404. The browser and HTTP checks assert the unavailable and method-refusal states. Successful download publication requires the operator command. Refund outbox reconciliation and worker execution also use operator commands. [Download HTTP routes](../../../../crates/downloads/src/http.rs#L14) · [Download operator](../../../../crates/downloads/README.md#L12) · [Worker and reconciliation entry points](../../../../bin/oteryn-portal/src/main.rs#L29)
- The built-in Security Admin role grants access, role management and audit. Player account and session views and web logout require Support Tickets, kick has a dedicated permission, and enforcement requires Support Enforcement. The two requested roles cannot exercise these positive scenarios under the built-in grants. The revocation and sanctions streams require their own service identities over mutual TLS. [Built-in role grants](../../../../crates/admin/src/rbac/catalog.rs#L352) · [Web logout](../../../../crates/admin/src/players/mod.rs#L327) · [Kick](../../../../crates/admin/src/players/mod.rs#L383) · [Security stream](../../../../crates/identity/src/security_revocations.rs#L278) · [Sanctions stream](../../../../crates/support/src/sanctions/export.rs#L265)

## Test corrections

- The registration language change now follows the portal footer's language link. The initial test waited for a home link with an exact accessible name that differed from the rendered navigation.
- Staging returned HTTP 429 on the registration challenge and registration submission during a browser probe. The suite retries once after the server's indicated wait and treats a repeated shared rate limit as a staging limitation.

## Product defects

None confirmed.
