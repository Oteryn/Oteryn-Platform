# Functional Acceptance Matrix

## Scope and evidence rules

This matrix inventories the functional surface delivered on `main` and records the strongest directly available acceptance evidence after the Functional Acceptance follow-up program from PR #66.

Repository checkpoint evidence uses `PROVEN`, `DERIVED`, `UNKNOWN` and `CONFLICT`. Environment evidence uses `STAGING_PROVEN`, `PRODUCTION_PROVEN` and `UNKNOWN`.

For the coverage columns below:

- `PROVEN` means deterministic repository tests or inspected executable validation cover the behavior;
- `STAGING_PROVEN` means the behavior was exercised through controlled production-like dependencies and/or the exact-SHA running HTTP application at the documented boundary;
- `DERIVED` means the conclusion follows from proven adjacent evidence but was not directly exercised at that exact layer;
- `UNKNOWN` means sufficient direct evidence is not available;
- `—` means the layer is not applicable.

No staging evidence in this document is promoted to `PRODUCTION_PROVEN`.

## Current acceptance result

**Functional inventory: PROVEN for the currently delivered `main` surface.**

**Full functional acceptance for the staging-verifiable delivered surface: STAGING_PROVEN.**

The blocking gaps identified by PR #66 are closed by composed merged evidence:

- PR #67, merge commit `517968539bdfd7d189677b669bf0899c35fccec1`, supplies the production-like browser acceptance harness and live HTTP evidence for FAV-01 through FAV-03;
- final PR #67 head `2a8b341d197e94346b01da9a0ee2181034e39322` passed `Acceptance E2E and Visual UX` run `29810312159`, job `88569620450`, with artifact `acceptance-e2e-29810312159-1` (`8487281635`), digest `sha256:189381cdc76a0cb2c16442b73bff55af1e60a9c2bcc0eef4368c864fb0ef0978`;
- PR #73, merge commit `06d8d94aafd73de996eb4ea93705e8a45fbadafb`, supplies controlled Platform DB outage evidence for FAV-04;
- PR #74, merge commit `24eaa4ca5e38bb255db95a989c0ff02e954360f3`, supplies focused CMS publication-state and privileged-audit sensitive-data regressions for FAV-05;
- existing Phase 7 production-like validation remains the composed infrastructure, privilege, Redis, SMTP, release and recovery evidence layer.

Issues #68, #69, #70, #71 and #72 are satisfied by this merged evidence set.

Final production state remains `UNKNOWN`; production smoke and the Production Go-Live Gate remain pending direct execution against the exact deployed production SHA.

Visual / UI / UX Acceptance is a separate gate and remains `FAIL` until the dedicated UI/UX launch-readiness task resolves its evidenced blockers.

## Identity and authentication

| Feature | Repository evidence | Live staging E2E | Negative/failure | Authorization | Production smoke |
|---|---|---|---|---|---|
| Registration | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | guest-only / server-owned binding | yes |
| Client-controlled legacy engine/account identifiers rejected or ignored | `PROVEN` | `STAGING_PROVEN` in cross-surface ownership checks | `PROVEN` | `PROVEN` | no |
| Login and logout | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | guest/authenticated boundaries proven | yes |
| Password recovery through SMTP reset link | `PROVEN` | `STAGING_PROVEN` with MailHog SMTP | `PROVEN` generic request behavior and token replay rejection | token-authorized flow | yes |
| Password reset | `PROVEN` | `STAGING_PROVEN` | `PROVEN` expired/replayed token rejection | token-authorized flow | yes |
| Authenticated password change | `PROVEN` | `STAGING_PROVEN` | `PROVEN` old password/session rejection | auth required | yes |
| Web-session generation invalidation | `PROVEN` | `STAGING_PROVEN` | `PROVEN` stale-session denial | `PROVEN` | yes |
| Unauthenticated/stale-session protected-route denial | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | `PROVEN` | yes |

## MFA

| Feature | Repository evidence | Live staging E2E | Negative/failure | Authorization | Production smoke |
|---|---|---|---|---|---|
| Enrollment and confirmation | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | authenticated identity required | yes |
| Login challenge | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | pending-login state | yes |
| Invalid TOTP rejection | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | `PROVEN` | no |
| Replayed TOTP rejection | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | `PROVEN` | no |
| Recovery-code login | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | `PROVEN` | yes |
| Recovery-code single use / replay rejection | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | `PROVEN` | no |
| MFA disable and session revocation | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | authenticated identity + valid factor | yes |
| `mfa.confirmed` privileged gate | `PROVEN` | `STAGING_PROVEN` | `PROVEN` no-MFA denial | `PROVEN` | yes |

## Account provisioning and binding

| Feature | Repository/integration evidence | Live staging E2E | Negative/failure | Authorization | Production smoke |
|---|---|---|---|---|---|
| Platform-originated legacy engine provisioning | `PROVEN` with real MariaDB integration and privilege validation | `STAGING_PROVEN` | `PROVEN` | no client-selected account authority | yes |
| Identity -> legacy engine ready 1:1 binding | `PROVEN` | `STAGING_PROVEN` | `PROVEN` uniqueness constraints | server-owned | yes |
| Provisioning retry/idempotency | `PROVEN` | `STAGING_PROVEN` where deterministic | `PROVEN` | server-owned | no |
| Duplicate provisioning recovery | `PROVEN` real MariaDB | `STAGING_PROVEN` acceptance composition | `PROVEN` | dedicated principal | no |
| Partial-failure forward recovery | `PROVEN` real MariaDB | `DERIVED` from integration plus accepted live chain | `PROVEN` | dedicated principal | no |
| Provisioning dependency failure/pending state | `PROVEN` | `STAGING_PROVEN` fail-closed acceptance composition | `PROVEN` | `PROVEN` | no |
| Dedicated `canary_provisioning` least privilege | `STAGING_PROVEN` real MariaDB grants | boundary validation `STAGING_PROVEN` | `PROVEN` prohibited writes/reads | DB enforced | yes |

## Character management

Character deletion and rename are not delivered and are intentionally excluded from accepted scope.

| Feature | Repository/integration evidence | Live staging E2E | Negative/failure | Authorization | Production smoke |
|---|---|---|---|---|---|
| Character creation | `PROVEN` real MariaDB integration | `STAGING_PROVEN` | `PROVEN` | ready binding required | yes |
| Canonical name validation / reserved-name rejection | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | `PROVEN` | no |
| Duplicate/global-name handling | `PROVEN` including real MariaDB concurrency | `STAGING_PROVEN` deterministic browser boundary | `PROVEN` | `PROVEN` | no |
| Character quota | `PROVEN` real MariaDB | `STAGING_PROVEN` browser quota boundary | `PROVEN` | `PROVEN` | no |
| Binding/account ownership; foreign `account_id` ignored | `PROVEN` | `STAGING_PROVEN` | `PROVEN` pending/foreign authority denial | `PROVEN` | yes |
| Same-account and cross-account concurrency | `PROVEN` real MariaDB + `pcntl` | — | `PROVEN` | DB/application enforced | no |
| Idempotent retry/ambiguous-commit recovery | `PROVEN` | `STAGING_PROVEN` acceptance composition | `PROVEN` | dedicated principal | no |
| Dedicated `canary_character_create` privilege shape | `STAGING_PROVEN` | boundary validation `STAGING_PROVEN` | `PROVEN` prohibited cross-surface writes | DB enforced | yes |

## Public game data and public content

| Feature | Repository evidence | Live staging E2E | Negative/failure | Authorization | Production smoke |
|---|---|---|---|---|---|
| Home/public shell | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | public | yes |
| Character exact-name search/profile | `PROVEN` | `STAGING_PROVEN` including newly created character visibility | `PROVEN` | public | yes |
| Highscores | `PROVEN` | `STAGING_PROVEN` including empty pagination state | `PROVEN` | public | yes |
| Guild detail | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | public | no |
| Online list | `PROVEN` | `STAGING_PROVEN` | `PROVEN` explicit dependency failure | public | yes |
| Servers static/runtime state | `PROVEN` | `STAGING_PROVEN` including fresh, missing and malformed runtime states | `PROVEN` | public read-only | yes |
| Public news list/detail | `PROVEN` | `STAGING_PROVEN` through CMS lifecycle/public visibility acceptance | `PROVEN` | public | yes |
| Managed public pages | `PROVEN` | `STAGING_PROVEN` through CMS lifecycle/public visibility acceptance | `PROVEN` | public | yes |
| Generic legacy engine read-only DB boundary | `STAGING_PROVEN` real MariaDB grants | boundary validation `STAGING_PROVEN` | `PROVEN` writes/excess privilege denied | DB enforced | yes |

## Administrator and RBAC

| Feature | Repository evidence | Live staging E2E | Negative/failure | Authorization | Production smoke |
|---|---|---|---|---|---|
| First-admin bootstrap requires existing Identity | `PROVEN` by implementation/acceptance composition | `STAGING_PROVEN` | `PROVEN` unknown target failure | privileged operator command | yes |
| Bootstrap requires confirmed MFA and closes after first admin | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | bootstrap invariant | yes |
| Deny-by-default / explicit permission checks | `PROVEN` | `STAGING_PROVEN` | `PROVEN` missing/unknown permission denial | `PROVEN` | yes |
| No wildcard unrestricted bypass | `PROVEN` by explicit mapping/deny cases | `STAGING_PROVEN` acceptance composition | `PROVEN` | `PROVEN` | no |
| Role assignment/removal | `PROVEN` | `STAGING_PROVEN` | `PROVEN` missing permission denial | exact permission + MFA | yes |
| Last-platform-admin protection | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | `PROVEN` | yes |
| Unauthorized admin access/mutation | `PROVEN` | `STAGING_PROVEN` | `PROVEN` guest/no-MFA/missing-permission denial | `PROVEN` | yes |

## CMS and managed content

| Feature | Repository evidence | Live staging E2E | Negative/failure | Authorization | Production smoke |
|---|---|---|---|---|---|
| News create/edit/publish | `PROVEN` | `STAGING_PROVEN` | `PROVEN` unauthorized mutation/publication filtering | exact CMS permission + MFA | yes |
| News unpublish | `PROVEN` by PR #74 focused regression | `STAGING_PROVEN` | `PROVEN` published content becomes hidden | exact CMS permission + MFA | yes |
| Managed page create/publish | `PROVEN` | `STAGING_PROVEN` | `PROVEN` draft/future hiding | exact CMS permission + MFA | yes |
| Managed page edit/unpublish | `PROVEN` by PR #74 focused regression | `STAGING_PROVEN` | `PROVEN` public route hides unpublished page | exact CMS permission + MFA | yes |
| Escaped/plain-text rendering | `PROVEN` | accepted staging composition | `PROVEN` XSS regression | public | no |
| Unauthorized CMS mutation denial | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | `PROVEN` | yes |

## Audit

| Feature | Repository evidence | Live staging E2E | Negative/failure | Authorization | Production smoke |
|---|---|---|---|---|---|
| Privileged mutations produce audit events | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | privileged mutation permissions | yes |
| Role-change and CMS audit metadata | `PROVEN` | `STAGING_PROVEN` acceptance composition | `PROVEN` bounded metadata | server-controlled | no |
| Bounded audit visibility | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | `audit.view` + MFA | yes |
| Permission-protected audit access | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | `PROVEN` | yes |
| No passwords/tokens/MFA secrets/hashes/application key in admin audit records | `PROVEN` by PR #74 dedicated regression | accepted staging composition | `PROVEN` | server-controlled call sites | no |

## Security behavior

| Feature | Repository/infrastructure evidence | Live staging E2E | Negative/failure | Production smoke |
|---|---|---|---|---|
| CSP and browser security headers | `PROVEN` | Phase 7 `STAGING_PROVEN` | `PROVEN` | yes |
| Secure/HttpOnly production-like session-cookie behavior | `PROVEN` | Phase 7 `STAGING_PROVEN` | `PROVEN` configuration guardrail | yes |
| HTTPS application configuration guardrail | `PROVEN` | `STAGING_PROVEN` | `PROVEN` insecure config rejection | yes |
| Final TLS/reverse-proxy trust/HSTS | — | `UNKNOWN` final production boundary | `UNKNOWN` | yes |
| Debug disabled | `PROVEN` | `STAGING_PROVEN` | `PROVEN` | yes |
| Sensitive error/log handling | `PROVEN` bounded checks | `STAGING_PROVEN` including PR #73 outage checks | `PROVEN` representative secret exclusions | yes |
| Request correlation and structured logging | `PROVEN` | Phase 7 `STAGING_PROVEN` | `PROVEN` | yes |
| Rate limits | `PROVEN` for configured Identity/MFA routes | accepted with middleware active; not independently load-tested | `PROVEN` configured abuse bounds | no |
| CSRF protection | `PROVEN` framework/forms boundary | `STAGING_PROVEN` explicit missing/invalid CSRF rejection in PR #67 | `PROVEN` | yes |
| Authorization bypass attempts | `PROVEN` | `STAGING_PROVEN` | `PROVEN` guest, stale session, no MFA, missing/unknown permission and foreign account authority fail closed | yes |

## Infrastructure-dependent failure and recovery paths

| Feature | Evidence | Staging classification | Production smoke |
|---|---|---|---|
| Platform DB unavailable | PR #73 controlled exact-SHA HTTP outage validation: no false-success mutation, Platform state unchanged, bounded response/log secret checks, recovery read | `STAGING_PROVEN` | yes |
| Legacy engine DB unavailable / failing public read | Phase 7 verifier evidence plus explicit public failure behavior | `STAGING_PROVEN` at documented boundaries | yes |
| Insufficient provisioning DB privileges | real MariaDB grant mutation/verifier | `STAGING_PROVEN` | yes |
| Excessive generic legacy engine DB privileges | real MariaDB grant mutation/verifier | `STAGING_PROVEN` | yes |
| Excessive/denied character-create privileges | MariaDB integration and Phase 7 prohibited-write checks | `STAGING_PROVEN` | yes |
| Redis unavailable/malformed/missing | runtime tests + Phase 7 + live server-state acceptance | `STAGING_PROVEN` | yes |
| Mail unavailable | password-recovery tests + Phase 7 unavailable SMTP endpoint | `STAGING_PROVEN` at transport/failure boundary | yes |
| Invalid production configuration | production verifier + Phase 7 | `STAGING_PROVEN` | yes |
| Interrupted deployment isolation | Phase 7 release workflow | `STAGING_PROVEN` | yes |
| Rollback and redeploy | Phase 7 release workflow | `STAGING_PROVEN` | yes |
| Backup/restore/recovery | Phase 7 real MariaDB controlled recovery; final PR-head measured restore remains staging-only | `STAGING_PROVEN` | yes |

## Required cross-surface acceptance scenarios

| Scenario | Status | Evidence |
|---|---|---|
| New Identity -> login -> MFA -> legacy engine provisioning -> ready binding -> character creation -> public character visibility | `STAGING_PROVEN` | merged PR #67 production-like browser acceptance on exact final PR head plus real-MariaDB privilege/integration evidence |
| Returning Identity -> password recovery/change -> stale-session invalidation -> MFA challenge | `STAGING_PROVEN` | merged PR #67 live HTTP + MailHog browser acceptance |
| Admin bootstrap -> MFA/RBAC -> role management -> CMS create/edit/publish/unpublish -> public visibility/hiding -> audit visibility | `STAGING_PROVEN` | merged PR #67 live browser acceptance plus PR #74 focused regressions |
| Representative authorization/failure abuse: CSRF, guest/stale session, invalid/replayed MFA, foreign account authority, last-admin removal, unauthorized CMS | `STAGING_PROVEN` | merged PR #67 acceptance plus repository/integration regressions |
| Platform DB outage -> no false success / no sensitive diagnostics -> normal recovery | `STAGING_PROVEN` | merged PR #73 dedicated workflow |

## Explicitly not implemented / not accepted as delivered scope

The following remain outside the delivered functional surface and must not be inferred from adjacent functionality:

- public existing-account claim/import flow;
- character deletion;
- character rename;
- self-service legacy engine binding unlink/rebind/transfer;
- account deletion;
- authoritative Platform game-login bridge unless separately authorized and implemented.

## Functional Acceptance follow-up register

| Follow-up | Result | Durable evidence |
|---|---|---|
| FAV-01 — Identity/MFA/session/CSRF live production-like E2E | `CLOSED / STAGING_PROVEN` | merged PR #67; issue #68 closed |
| FAV-02 — provisioning/binding/character/public live production-like E2E | `CLOSED / STAGING_PROVEN` | merged PR #67 + existing real-MariaDB integration/privilege evidence; issue #69 closed |
| FAV-03 — admin/RBAC/CMS/audit live production-like E2E | `CLOSED / STAGING_PROVEN` | merged PR #67 + PR #74 focused regressions; issue #70 closed |
| FAV-04 — Platform DB outage and bounded error/log validation | `CLOSED / STAGING_PROVEN` | merged PR #73; issue #71 closed |
| FAV-05 — focused CMS publication-state and audit-secret regressions | `CLOSED / PROVEN` | merged PR #74; issue #72 closed |

No remaining functional-acceptance staging gap is known for the currently delivered scope.

## Production smoke boundary

The minimal final production-only verification is maintained separately in `docs/testing/PRODUCTION_SMOKE_CHECKLIST.md`.

Production-specific facts remain `UNKNOWN` until direct final-production execution against the exact deployed SHA. This includes final edge/TLS/origin posture, production data-service endpoints and effective grants, production Redis/mail/session/cache/queue topology, deployment/rollback/backup operations and the final critical production smoke/E2E pass.

Passing this matrix does not make Visual / UI / UX Acceptance pass and does not open the Production Go-Live Gate by itself.
