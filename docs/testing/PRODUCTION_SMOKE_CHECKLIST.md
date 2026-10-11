# Final Production Smoke Checklist

## Purpose

This is the minimal final-production verification pass after staging-verifiable functional work is closed. It is intentionally smaller than the Functional Acceptance Matrix and must be executed only against the final deployed environment.

Nothing in this checklist is `PRODUCTION_PROVEN` until directly executed and evidenced on the final production deployment. Current status for every item is `UNKNOWN`.

Do not store production credentials, tokens, reset links, TOTP secrets, recovery codes, private endpoints, database dumps or other secrets in repository evidence.

## Preconditions

- [ ] `UNKNOWN` — Record the exact deployed Oteryn Platform commit SHA and relevant legacy engine/login-server versions without secrets.
- [ ] `UNKNOWN` — Confirm required CI/security/governance checks passed for the exact deployed candidate.
- [ ] `UNKNOWN` — Confirm the authorized production operator and rollback path are available before smoke begins.
- [ ] `UNKNOWN` — Confirm production backup scope and latest usable restore evidence are available before exercising mutations.

## Minimal application smoke

- [ ] `UNKNOWN` — Home is reachable through the real production hostname.
- [ ] `UNKNOWN` — Registration/login surfaces are reachable through the real production hostname and expected edge/proxy path.
- [ ] `UNKNOWN` — One valid Platform Identity can log in and log out; stale-session behavior remains fail-closed.
- [ ] `UNKNOWN` — One confirmed-MFA Identity completes a real MFA login challenge; do not persist the TOTP secret or recovery code in evidence.
- [ ] `UNKNOWN` — One password-recovery message is delivered through the real production mail provider/domain and the reset completes successfully.

## Legacy engine/public game-data smoke

- [ ] `UNKNOWN` — One read-only public game-data flow returns expected data through the generic least-privilege legacy engine read principal.
- [ ] `UNKNOWN` — Effective generic legacy engine DB grants pass `legacy:verify-db-privileges` on the production credential/network path.
- [ ] `UNKNOWN` — Runtime server state, when enabled, is read through the dedicated production Redis principal and effective ACL/freshness behavior is evidenced.

## Account and character smoke

Run only if these launch-scope mutation surfaces are enabled in final production.

- [ ] `UNKNOWN` — One new Platform Identity completes Platform-originated legacy engine account provisioning and reaches a ready immutable 1:1 binding.
- [ ] `UNKNOWN` — Effective provisioning grants pass `legacy:verify-provisioning-db-privileges` on the production credential/network path.
- [ ] `UNKNOWN` — The bound Identity creates one character through the dedicated character-create principal without client-controlled account ownership.
- [ ] `UNKNOWN` — Effective character-create grants pass `legacy:verify-character-create-db-privileges` on the production credential/network path.
- [ ] `UNKNOWN` — The created character becomes visible through the expected public exact-name search/profile flow.

## Administrator/RBAC/CMS smoke

Use a controlled production administrator Identity with confirmed MFA.

- [ ] `UNKNOWN` — One allowed administrator route succeeds with the exact required permission.
- [ ] `UNKNOWN` — One safe deny check confirms a missing permission does not gain privileged access.
- [ ] `UNKNOWN` — One controlled CMS item is created or edited, published, observed publicly, then restored to the intended final publication state.
- [ ] `UNKNOWN` — The corresponding privileged operation appears in the permission-protected bounded audit view.

Do not rerun first-admin bootstrap when production bootstrap is already closed.

## Mail, logging and monitoring

- [ ] `UNKNOWN` — Real production password-recovery mail delivery succeeds and provider/bounce monitoring is operational.
- [ ] `UNKNOWN` — A production request carries a server-generated request ID through the effective proxy path.
- [ ] `UNKNOWN` — The same request appears in the production structured logging sink with bounded non-sensitive context.
- [ ] `UNKNOWN` — Production metrics/alerts/on-call routing for the deployed service are active and an authorized non-destructive signal is observable.

## Backup and recovery evidence

- [ ] `UNKNOWN` — Record current production backup schedule, scope, retention, encryption and access-control evidence.
- [ ] `UNKNOWN` — Record the latest dated production restore test and its measured recovery/data-loss observations.
- [ ] `UNKNOWN` — Do not reuse the controlled staging restore duration as production RTO or RPO.

## Edge and transport security

- [ ] `UNKNOWN` — Confirm the exact public hostname resolves through the intended DNS/edge/reverse-proxy path.
- [ ] `UNKNOWN` — Confirm the real TLS certificate/termination behavior and HTTP-to-HTTPS policy.
- [ ] `UNKNOWN` — Confirm effective origin exposure/firewall restrictions.
- [ ] `UNKNOWN` — Confirm production responses contain the expected CSP, `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy` and `Permissions-Policy` headers.
- [ ] `UNKNOWN` — Confirm production session cookies are `Secure` and `HttpOnly` with the intended SameSite policy.
- [ ] `UNKNOWN` — Make and record the HSTS decision from the actual production hostname/TLS/proxy topology rather than from staging assumptions.

## Completion rule

Production smoke is complete only when every launch-applicable item above has direct non-secret evidence tied to the exact deployed SHA. Only then may those individual facts be classified `PRODUCTION_PROVEN`.

A failed smoke item must remain non-passing, trigger the applicable rollback/incident procedure when required, and must not be hidden by prior `STAGING_PROVEN` evidence.
