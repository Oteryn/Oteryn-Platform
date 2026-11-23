# Staging qualification findings

Run against `https://staging.oteryn.com` with local Chromium, Polish and English, and 1440 px and 390 px viewports. [Route inventory and observed coverage](COVERAGE.md)

## Product defects

### Login heading differs from the portal locale files

- Routes `/pl/login` and `/en/login`. Open the localized account character creation URL while signed out. The portal redirects to the matching login page and displays the password form.
- Expected `Logowanie` and `Log in`. The template uses `login-title` for the heading. [Template](../../../../templates/account/login/page.html#L4) · [Polish locale](../../../../templates/account/login/pl.ftl#L1) · [English locale](../../../../templates/account/login/en.ftl#L1)
- Observed `Zaloguj się` and `Sign in` on both viewports. The deployment SHA could not be established, so the source-to-staging difference is confirmed and its cause is unknown.
- Local evidence `test-results/game-Player-navigation-and-signed-out-account-views-{pl-1440,pl-390,en-1440,en-390}/login-heading.png`.

### A valid registration can return HTTP 503

- Route `POST /pl/register`. Follow the registration link from sign-in, enter a fresh staging address and matching passwords, accept both legal terms and submit. The observed request used the 390 px project.
- Expected HTTP 303 to `/pl/register/check-inbox` after accepting valid input. The handler has a 503 branch for unavailable registration dependencies. [Form](../../../../templates/account/register.html#L5) · [Handler](../../../../crates/web/src/routes/register.rs#L117) · [Success redirect](../../../../crates/web/src/routes/register.rs#L144)
- Observed HTTP 503 instead of the check-inbox page. The specific unavailable dependency was unverified. Shared-egress HTTP 429 responses were retried and are excluded from the defect.
- The run captured the response status. No registration screenshot was generated on that attempt.

## Staging limitations

- The Mailpit UI and API returned 404 while `/readyz` returned 200. The API still returned 404 ten minutes later. The checked-in Oracle ingress forwards requests to the portal and Mailpit is on the private service network. The live host's ingress configuration was unavailable. This blocked the verification and password-reset ceremonies required for fresh player and seeded staff access. [Mail and ingress configuration](../../../../../deploy/oracle/staging.compose.yml#L42)
- The checked-in staging profile disables character creation and multiple Game-backed capabilities. Public routing and disabled/unavailable responses were checked. Owned characters, Game-backed writes, staff approvals and cross-actor persistence could not be exercised. [Staging profile](../../../../../deploy/oracle/staging-deploy.py#L38)
- The staff routes returned 404 on the provided public host. The source requires the configured administration host and enabled staff panel. No connected Game agent was available for the environment. [Staff guard](../../../../crates/admin/src/web.rs#L174) · [Console handler](../../../../crates/admin/src/console/mod.rs#L388)

## Test and environment corrections

- The npm default cache was unavailable in this workspace. Installing with a writable temporary cache preserved the locked dependencies.
- Chromium initially rejected the execution environment's HTTPS proxy certificate. The run used `STAGING_CHROMIUM_PROXY_SPKI` with the proxy leaf key for `staging.oteryn.com`, after OpenSSL validated its chain and hostname against the environment's proxy CA. The Playwright API context continued to use its normal TLS validation.
