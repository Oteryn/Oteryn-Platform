# Credential changes and sudo

`OTERYN_ACCOUNT_CREDENTIALS=true` opts into `/password/change` (GET, POST,
PUT) and `/account/sudo` (GET, POST), including `/pl` and `/en` variants.
The default is disabled. Composition also requires the notification service signing
key in `OTERYN_NOTIFICATION_SIGNING_KEY`; browser origins use
`OTERYN_PUBLIC_ORIGIN`. Forms use the existing session and CSRF middleware,
T04's password policy and the named password-change/security-mutation rate limits.
Browser validation failures redirect with a generic localized flash message. A
successful password change redirects to login and clears the current cookie.
Sudo lasts 15 minutes and rotates both the cookie identifier and CSRF token.

The base did not contain T16 or T21, and their remote branches were absent at
implementation time. Consequently the configured router uses
`UnavailableRevoker`: password changes fail closed with 503. Do not replace it
with a no-op when enabling the feature. Compose `credentials_http::router` with
an `AuthorizationRevoker` that uses the caller's transaction to revoke outstanding
OAuth authorization codes, access/refresh families, device families and trusted
devices, and enforce an activated native non-rollback witness. Any failure must
return an error. It must not commit, start an independent transaction or advance
the account's native generation: the credential service owns that increment.
The positive account generation must remain below the PostgreSQL bigint limit.
An overflow aborts the entire change.

`Credentials::change` validates input, locks the active verified account, rechecks
the live browser session and verifies the current password. It then invokes the
revoker, replaces the password, advances the native generation, deletes every web
session (including the current one), queues the existing `PasswordChanged` security
notification and writes an audit event in one transaction. Other credential
mutation flows must use the same account-first locking order and revocation
contract. An already admitted game connection is outside this mechanism.

`require_sudo` checks current database state; callers must not cache its result
across sensitive mutations. A sensitive operation must recheck sudo and account
state within its own account-locked transaction. Public credential APIs take a
SHA-256 cookie digest, not a form-supplied owner ID. HTTP adapters must apply CSRF,
rate limits and body limits before calling them.

Tests use isolated PostgreSQL schemas and read `PORTAL_TEST_DATABASE_URL`. They
print a skip notice if unset. The service test exercises a transactional test
revoker for all five authorization categories, rollback after partial revocation,
missing-dependency denial, input/refusal cases, notification enqueueing, complete
web-session invalidation and generation advancement. The browser test covers real
session middleware, CSRF, localized forms, sudo cookie/CSRF rotation, expiry and
throttling. The test revoker is not a substitute for T16/T21 integration tests.

Validation: `cargo fmt --all --check`, `cargo clippy --locked --all-targets --
-D warnings` and `cargo test --locked` passed with `SQLX_OFFLINE=true` and the
PostgreSQL test URL set: 337 passed, 0 failed, 1 existing ignored; both T08C
PostgreSQL tests ran. The isolated credential router was checked in Chromium
at 360 and 1280 px in both languages: no horizontal overflow, expected keyboard
order, and zero axe WCAG 2.2 AA violations. Full-application browser startup is
blocked by the inherited duplicate `GET /register` registration in web composition;
that unrelated code was left unchanged. The isolated harness used the production
credential router, production CSS and a synthetic local PostgreSQL account/session.
