# Phase 7 Production-Like Validation Evidence

## Purpose

This document records non-secret evidence produced by the controlled Phase 7 production-like validation workflow.

It does **not** treat CI/staging as final production.

Evidence classifications used here:

- `STAGING_PROVEN` — directly demonstrated by a successful controlled production-like workflow run on an exact commit SHA;
- `PRODUCTION_PROVEN` — directly demonstrated in the final production environment;
- `UNKNOWN` — not yet proven for the stated environment.

A successful staging result may prove that a procedure, guardrail or least-privilege design works under controlled production-like conditions. It does not prove final production DNS, TLS, firewall, network isolation, provider configuration, deployed credentials, backup schedule, monitoring sink or exact deployed production SHA.

Under ADR 0007, this evidence closes the Phase 7 engineering/hardening validation boundary but does not satisfy the separate Production Go-Live Gate.

## Latest successful exact-head controlled validation

Validation date: `2026-07-20T21:18:07Z`

Workflow: `Phase 7 Production-Like Validation`

Workflow run: `29779554130` / run number `9`

Validated Oteryn Platform SHA: `7842f78ec4ac2d07d3800ffe8bde9809b055822d`

Rollback target SHA: `b6878c4775eda542738c78ea99fd5d2e19d2b35f`

Evidence artifact: `phase7-production-like-evidence-29779554130`

Evidence artifact digest: `sha256:e4b272dee3ac26ed789525b3191946b74a41685910f436db41ef8df49d64f96b`

Required repository CI on the same validation SHA: run `29779553687` / CI run number `759` — PASS.

Agent Governance on the same validation SHA: run `29779554188` / run number `679` — PASS.

The artifact contains only non-secret status values, exact commit identities, timestamp, row-count integrity measurements and restore duration. It intentionally contains no credentials, connection strings, private endpoints or environment dump.

An earlier successful controlled run #5 on SHA `b6dcd6ed95c55f400206864ffd6ff799e65aa2b3` measured a `102 ms` restore. The latest final-PR-head run #9 below measured `105 ms`. Both measurements are staging-only recovery evidence and neither is a production RTO or RPO.

## Staging evidence matrix

| Boundary | Classification | Evidence |
|---|---|---|
| Exact staging validation SHA | `STAGING_PROVEN` | Workflow run 9 validated final PR #63 head `7842f78ec4ac2d07d3800ffe8bde9809b055822d`. |
| Clean deployment | `STAGING_PROVEN` | Clean release directory was built from the exact validation SHA and activated through the controlled release pointer. |
| Migrations | `STAGING_PROVEN` | `php artisan migrate --force --no-interaction` passed against the production-like Platform MariaDB database. |
| Configuration guardrails | `STAGING_PROVEN` | `php artisan production:verify-configuration` passed; a deliberate `APP_DEBUG=true` mutation failed closed. |
| Health/readiness | `STAGING_PROVEN` | The deployed release booted and `/health` returned successfully. |
| Rollback | `STAGING_PROVEN` | Release pointer switched to `b6878c4775eda542738c78ea99fd5d2e19d2b35f`; configuration and migration-state smoke passed. |
| Interrupted deployment isolation | `STAGING_PROVEN` | An incomplete release directory was created without changing the active release pointer. |
| Redeploy current SHA | `STAGING_PROVEN` | Release pointer returned to `7842f78ec4ac2d07d3800ffe8bde9809b055822d`; migrations/configuration passed again. |
| Generic Canary read-only effective grants | `STAGING_PROVEN` | Real MariaDB principal provisioned from the reviewed SQL template; `canary:verify-db-privileges` passed. |
| Generic Canary write denial | `STAGING_PROVEN` | The read-only principal was denied an `UPDATE` attempt. |
| Generic Canary excess privilege fail-closed | `STAGING_PROVEN` | Temporary `UPDATE` privilege caused the application verifier to fail; privilege was revoked and the verifier passed again. |
| Provisioning effective grants | `STAGING_PROVEN` | Real MariaDB column-level principal provisioned from the reviewed SQL template; `canary:verify-provisioning-db-privileges` passed. |
| Provisioning insufficient privilege fail-closed | `STAGING_PROVEN` | A required column-level `SELECT` privilege was removed; the verifier failed, then passed after restoration. |
| Provisioning cross-surface write denial | `STAGING_PROVEN` | Provisioning principal was denied an attempted `players` insert. |
| Character-create effective grants | `STAGING_PROVEN` | Real MariaDB column-level principal provisioned from the reviewed SQL template; `canary:verify-character-create-db-privileges` passed. |
| Character-create cross-surface write denial | `STAGING_PROVEN` | Character-create principal was denied an attempted `accounts` update. |
| Canary DB unavailable behavior | `STAGING_PROVEN` | Privilege verification against an unavailable Canary endpoint failed closed. Application-level dependency degradation remains covered by the exact-SHA regression suite. |
| Runtime Redis ACL/key/command boundary | `STAGING_PROVEN` | Real Redis ACL user restricted to `cluster:channel:*:runtime` plus the required read commands successfully read valid runtime data; `SET` was denied with `NOPERM`. |
| Runtime Redis missing/expired semantics | `STAGING_PROVEN` | Missing runtime key returned no runtime state rather than fabricated availability. TTL validity is exercised by the real `PTTL` path and regression suite. |
| Runtime Redis malformed data | `STAGING_PROVEN` | Invalid status payload raised the expected fail-closed malformed-data behavior. |
| Runtime Redis unavailable behavior | `STAGING_PROVEN` | Reader against an unavailable Redis endpoint failed rather than returning fabricated healthy runtime data. |
| SMTP delivery-capable staging path | `STAGING_PROVEN` | Application delivered through a real SMTP protocol connection to a safe test SMTP service; Laravel `array`/`log` transport was not used as delivery evidence. |
| Mail-unavailable behavior | `STAGING_PROVEN` | Delivery against an unavailable SMTP endpoint failed as expected. |
| Session runtime path | `STAGING_PROVEN` | Controlled environment used the repository's file-session default; live web middleware produced a Secure and HttpOnly session cookie. This does not prove production multi-instance suitability. |
| Cache mode | `STAGING_PROVEN` | The controlled environment had no external cache dependency and used the repository's file-cache default; no production shared-cache requirement is inferred from this result. |
| Queue mode | `STAGING_PROVEN` | The controlled environment used the repository's synchronous queue mode and completed critical flows without an asynchronous worker dependency. Production queue topology remains a separate decision if scope changes. |
| Full exact-SHA regression suite | `STAGING_PROVEN` | The production-like workflow's isolated test environment completed `composer test`; required CI #759 independently passed Composer audit, Pint, PHPStan and full tests on the same SHA. |
| Registration and Identity↔Canary binding | `STAGING_PROVEN` | Exact-SHA feature coverage validates registration, durable binding intent, ready binding and failure-to-pending behavior; MariaDB integration coverage separately exercises the real provisioning adapter and least-privilege boundary. |
| Login/logout and session revocation | `STAGING_PROVEN` | Exact-SHA Identity web-session feature coverage passed in the full suite. |
| Password recovery/change | `STAGING_PROVEN` | Exact-SHA password recovery/change feature coverage passed; the controlled environment separately proved a delivery-capable SMTP path. Final production provider/domain delivery remains `UNKNOWN`. |
| MFA | `STAGING_PROVEN` | Exact-SHA MFA state and web-flow feature coverage passed. |
| Administrator bootstrap and RBAC deny/allow | `STAGING_PROVEN` | Exact-SHA administrator authorization/role-management coverage passed, including deny-by-default policy and bootstrap invariants. |
| CMS and administrator audit | `STAGING_PROVEN` | Exact-SHA CMS management and administrator audit feature coverage passed. |
| Platform-originated account provisioning | `STAGING_PROVEN` | Exact-SHA provisioning action coverage plus real MariaDB integration tests and effective-grant verification passed. |
| Character creation | `STAGING_PROVEN` | Exact-SHA character-creation feature coverage plus real MariaDB integration tests and effective-grant verification passed. |
| Public news/character/search/online/highscores/servers flows | `STAGING_PROVEN` | Full exact-SHA feature/regression suite passed; live `/` and `/health` smoke passed; runtime server availability additionally used the real ACL-restricted Redis path. |
| CSP/browser security headers | `STAGING_PROVEN` | Live response contained CSP, `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy` and `Permissions-Policy`. |
| Secure/HttpOnly cookies | `STAGING_PROVEN` | Live login response set Secure and HttpOnly cookie attributes. |
| Request correlation | `STAGING_PROVEN` | Live response contained server-generated `X-Request-ID`. |
| Structured request logging | `STAGING_PROVEN` | Live request produced `http.request.completed` through the JSON-to-stderr logging channel. |
| Representative sensitive error/log behavior | `STAGING_PROVEN` | Representative 404 output did not expose a stack trace or ephemeral application key; request logs did not contain the key. This is a bounded representative check, not a proof that arbitrary future log messages are secret-free. |
| Rate limiting | `STAGING_PROVEN` | Exact-SHA registration/login/password-recovery security regression coverage passed. |
| Security/audit events | `STAGING_PROVEN` | Exact-SHA Identity and administrator audit regression coverage passed. |
| Platform DB backup | `STAGING_PROVEN` | Real MariaDB logical backup was created from the production-like Platform database using a consistent single-transaction dump. |
| Clean restore | `STAGING_PROVEN` | Backup restored into a newly created clean database. |
| Restore integrity | `STAGING_PROVEN` | Source/restored table counts matched `13/13`; migration counts matched `11/11`; a SHA-tagged restore probe matched the validated SHA. |
| Restored-environment smoke | `STAGING_PROVEN` | `migrate:status` and production configuration verification passed against the restored database. |
| Measured staging restore time | `STAGING_PROVEN` | Latest final-head restore completed in `105 ms` for this controlled dataset on `2026-07-20`; this is **not** a production RTO or RPO. |
| Emulated DNS/TLS/WAF/Access/authenticated-origin composition | `STAGING_PROVEN` | Edge Security Emulation run `30270571670` validated SHA `791c350b93406eabe50702f8860b0515678a80bb`; artifact `edge-security-emulation-evidence-30270571670` digest `sha256:17ba33a26793a7f8d536acbcb78097e961a27bff65a690874fb72b884634e0b7` records DNS, TLS 1.2/1.3, legacy TLS denial, mTLS origin authentication, direct-origin denial, forwarded-header normalization, WAF/protocol/body denial, HTTP 429, Access denial and independent Platform auth as PASS/denied outcomes. Reserved `.test`, loopback and ephemeral credentials only. |
| Actual production HTTPS/TLS/DNS/WAF/Access and origin ingress | `UNKNOWN` | The emulation proves a reviewed procedure and controlled composition only. Real provider settings, public certificates, hostname policy, HSTS scope, Cloudflare rules and effective production origin exposure must be verified in the final environment. |
| Centralized logging/metrics/alerts/on-call | `UNKNOWN` | Application JSON logging primitive is proven; final production sink, retention, access, alerting and on-call routing require final environment evidence. |

## Critical-flow evidence composition

The production-like evidence intentionally combines complementary layers instead of pretending one synthetic test proves everything:

1. the exact-SHA full feature/regression suite exercises registration, login/logout, password recovery/change, MFA, RBAC, administrator/CMS/audit, account/binding, character and public read flows;
2. MariaDB integration tests exercise real database transaction/privilege behavior for provisioning and character creation;
3. the environment workflow runs the three effective-grant verifiers against real dedicated MariaDB principals and actively proves prohibited cross-surface writes are denied;
4. the runtime Redis path uses a real ACL user and real TTL/hash reads;
5. mail delivery uses an actual SMTP protocol path to a safe test service;
6. the deployed release itself is live-smoked for health, browser headers, cookies, correlation and structured logging;
7. the edge-emulation workflow composes reserved authoritative DNS, public-edge TLS, authenticated origin pulls, WAF/rate limiting and optional Access admission around the current-SHA Laravel application.

This composition is `STAGING_PROVEN`. It is not a substitute for final production smoke tests against the exact deployed production SHA.

## Recovery evidence interpretation

The measured `105 ms` latest final-head restore time and earlier `102 ms` staging restore apply only to the small controlled datasets and runners used by the corresponding workflows. Neither may be converted into a production RTO or RPO.

The deployment/rollback result proves the controlled release-directory and atomic pointer-switch model works. It does not prove that the final hosting provider uses that model or that provider-specific rollback access is configured correctly.

## Final Production Go-Live verification pass

The following items remain `UNKNOWN` until directly proven in the final production environment. This is the minimal production-only pass after Phase 7 engineering/hardening completion:

- [ ] `PRODUCTION_PROVEN` — exact deployed Oteryn Platform SHA and relevant Canary/login-server versions; confirm required CI passed for the exact production candidate.
- [ ] `PRODUCTION_PROVEN` — production DNS/proxy/Cloudflare/WAF/Access state and actual TLS termination/certificate behavior; make the HSTS decision from the real hostname/TLS policy.
- [ ] `PRODUCTION_PROVEN` — direct-origin exposure decision and effective ingress firewall/reverse-proxy restrictions.
- [ ] `PRODUCTION_PROVEN` — production Platform DB endpoint/topology/network isolation/HA and effective credential ownership/rotation.
- [ ] `PRODUCTION_PROVEN` — production generic Canary DB effective grants using `canary:verify-db-privileges` plus actual network-path separation.
- [ ] `PRODUCTION_PROVEN` — production provisioning DB effective grants using `canary:verify-provisioning-db-privileges`, if provisioning is enabled, plus actual dedicated credential/network-path separation.
- [ ] `PRODUCTION_PROVEN` — production character-create DB effective grants using `canary:verify-character-create-db-privileges`, if character creation is enabled, plus actual dedicated credential/network-path separation.
- [ ] `PRODUCTION_PROVEN` — production runtime Redis endpoint, network/TLS state and effective ACL for the dedicated read principal; prove dependency/freshness monitoring.
- [ ] `PRODUCTION_PROVEN` — effective production session/cache/queue choices, multi-instance suitability and worker supervision/retry/failed-job handling if asynchronous queues are introduced.
- [ ] `PRODUCTION_PROVEN` — production mail provider, sender-domain readiness, password-recovery delivery and delivery/bounce monitoring.
- [ ] `PRODUCTION_PROVEN` — production structured logging/metrics/alerting sink, retention/access policy, request-ID preservation and on-call routing.
- [ ] `PRODUCTION_PROVEN` — actual production deployment/migration/rollback mechanism, migration compatibility boundary and authorized emergency rollback operator path.
- [ ] `PRODUCTION_PROVEN` — production backup scope/schedule/retention/encryption/access policy and a dated production restore test with measured production recovery time and data-loss observation; staging restore time is not production RTO/RPO.
- [ ] `PRODUCTION_PROVEN` — final production health/readiness and critical smoke/E2E checks against the exact deployed SHA, including registration/login/logout/password recovery/MFA/admin/RBAC/CMS/provisioning/binding/character/public reads as applicable to launch scope.
- [ ] Resolve the separately authorized authoritative game-login bridge if Platform-originated game login is part of launch scope.

Under ADR 0007:

- **Phase 7 — COMPLETE**;
- **Production Readiness — STAGING_PROVEN**;
- **Production Go-Live Gate — PENDING PRODUCTION VERIFICATION**;
- **Production Verification — REQUIRED BEFORE GO-LIVE**.

The go-live gate must remain fail closed until its mandatory production evidence is directly proven. No staging evidence is promoted to `PRODUCTION_PROVEN` by Phase 7 completion.
