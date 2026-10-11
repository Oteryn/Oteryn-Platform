# Oteryn Platform Production Verification Evidence

## Purpose

This is the durable, non-secret execution record for issue #91 and the authoritative Production Go-Live Gate in `docs/operations/PRODUCTION_READINESS_CHECKLIST.md`.

It is an evidence record, not a deployment configuration file. Never paste credentials, tokens, private keys, connection strings, copied `.env` content, private IP inventories, database dumps, password-reset URLs, TOTP secrets or recovery codes here.

## Current decision

- **Production Readiness: STAGING_PROVEN**
- **Functional Acceptance: STAGING_PROVEN for the currently delivered staging-verifiable functional surface**
- **Visual / UX Acceptance: PASS for the currently delivered staging-verifiable launch scope**
- **Production Go-Live Gate: PENDING PRODUCTION VERIFICATION**
- **Production Verification: NOT STARTED**

No item below is `PRODUCTION_PROVEN` until direct evidence from the actual final production environment is recorded and tied to the exact deployed release.

## Evidence states

Use only:

- `UNKNOWN` — not directly proven for the actual production environment;
- `PRODUCTION_PROVEN` — directly verified in the actual production environment with a non-secret evidence reference;
- `NOT_APPLICABLE` — allowed only for explicitly conditional checklist items, with a written launch-scope/topology rationale.

The gate fails closed. Blank evidence, staging evidence, repository evidence or an owner risk decision cannot convert a production-specific `UNKNOWN` item to `PRODUCTION_PROVEN`.

## Verification identity

| Field | Value | State / rule |
|---|---|---|
| Verification date/time | `UNKNOWN` | Record UTC or timezone-qualified timestamp when execution starts. |
| Verification operator | `UNKNOWN` | Record role or approved operator identity without credentials. |
| Oteryn Platform deployed SHA | `UNKNOWN` | Mandatory exact immutable commit SHA. |
| Oteryn Platform candidate CI evidence | `UNKNOWN` | Link/run identifiers for required checks on the exact deployed SHA. |
| Legacy engine deployed version/SHA/image digest | `UNKNOWN` | Record the immutable deployed identity relevant to production behavior. |
| Login-server deployed version/SHA/image digest | `UNKNOWN` | Record when present/relevant; do not infer from `latest`. |
| Production hostname(s) under verification | `UNKNOWN` | Public hostname only; do not record private endpoints. |
| Deployment/release identifier | `UNKNOWN` | Sanitized provider release ID or equivalent. |
| Rollback target/release | `UNKNOWN` | Non-secret immutable rollback reference. |
| Issue | `#91` | Production Go-Live execution tracker. |

## Hard preconditions before mutation smoke

Every row must be `PRODUCTION_PROVEN` before registration/provisioning/character/CMS/password-reset mutation smoke begins.

| Precondition | State | Non-secret evidence reference / observation |
|---|---|---|
| Exact deployed Platform SHA is proven and matches the release being evaluated | `UNKNOWN` | |
| Required CI/security/governance checks passed for that exact SHA | `UNKNOWN` | |
| Authorized production operator is present and authorized for the planned actions | `UNKNOWN` | |
| Actual production rollback mechanism and rollback target are available | `UNKNOWN` | |
| Applicable production backup scope/policy is active | `UNKNOWN` | |
| A dated usable production restore test exists for the applicable Platform data scope | `UNKNOWN` | |
| Production incident/escalation/on-call path is known | `UNKNOWN` | |

If any mandatory precondition is `UNKNOWN`, mutation smoke is blocked.

## 1. Exact release identity

| Gate item | State | Evidence |
|---|---|---|
| Exact deployed Oteryn Platform SHA recorded | `UNKNOWN` | |
| Relevant deployed legacy engine/login-server identities recorded | `UNKNOWN` | |
| Required CI passed for the exact deployed candidate SHA | `UNKNOWN` | |
| No unreviewed production-only code/config divergence outside the declared deployment mechanism | `UNKNOWN` | |

## 2. Effective production application configuration

Run in the effective production runtime without printing environment values or secrets:

```text
php artisan production:verify-configuration
```

| Gate item | State | Evidence |
|---|---|---|
| `production:verify-configuration` exits `0` in the effective production runtime | `UNKNOWN` | Record command result, timestamp and release identity only. |

This verifier does not prove database, Redis, queue, logging-provider or edge correctness.

## 3. Dependency and application security gates

| Gate item | State | Evidence |
|---|---|---|
| Required CI/security checks are green on the exact deployed SHA | `UNKNOWN` | |
| Critical/high dependency or application findings reviewed and resolved or handled by eligible documented owner decision | `UNKNOWN` | Risk decisions do not replace mandatory environment evidence. |

## 4. Edge, DNS, TLS and origin exposure

Do not record private origin addresses.

| Gate item | State | Evidence |
|---|---|---|
| Production DNS/proxy mode and TLS termination proven | `UNKNOWN` | Sanitized provider/config reference. |
| Effective WAF/rate-limit/Access policy proven where used | `UNKNOWN` | |
| Direct-origin access is blocked or an eligible risk decision is explicitly recorded | `UNKNOWN` | |
| Ingress firewall/security-group/reverse-proxy rules reviewed | `UNKNOWN` | Sanitized rule summary only. |
| HSTS decision recorded from actual hostname/TLS/subdomain topology | `UNKNOWN` | |

Suggested non-destructive public checks may include response/header inspection through the real production hostname. Do not probe private origin addresses from repository evidence automation.

## 5. Platform database

| Gate item | State | Evidence |
|---|---|---|
| Production DB engine/topology recorded without credentials | `UNKNOWN` | |
| Public exposure and approved network paths verified | `UNKNOWN` | |
| Credential injection/rotation ownership verified | `UNKNOWN` | Do not name secrets. |
| Backup scope, schedule, retention, encryption/access and owner verified | `UNKNOWN` | |
| Dated production restore test completed and recorded | `UNKNOWN` | Include scope, result, recovery time and data-loss observation; no dump. |

A staging restore duration is never production RTO/RPO evidence.

## 6. Legacy engine SQL effective privilege boundaries

Run only with the effective production credential classes already provisioned outside Git:

```text
php artisan legacy:verify-db-privileges
php artisan legacy:verify-provisioning-db-privileges
php artisan legacy:verify-character-create-db-privileges
```

| Gate item | State | Evidence |
|---|---|---|
| Generic legacy engine read-only verifier passes on production credential/network path | `UNKNOWN` | |
| Provisioning verifier passes before provisioning writes are enabled | `UNKNOWN` | |
| Character-create verifier passes before character-create writes are enabled | `UNKNOWN` | |
| Production network paths and credential classes are separate as designed | `UNKNOWN` | |

A failed effective-grant verifier blocks the affected capability.

## 7. Legacy engine runtime Redis

| Gate item | State | Evidence |
|---|---|---|
| Production runtime Redis endpoint/network boundary proven without secrets | `UNKNOWN` | |
| Dedicated read-only ACL/user provisioned | `UNKNOWN` | |
| Redis transport/TLS posture proven where applicable | `UNKNOWN` | |
| Dependency failure and freshness monitoring/alerting proven | `UNKNOWN` | |

## 8. Sessions, cache and queue

| Gate item | State | Evidence |
|---|---|---|
| Effective production session backend recorded and justified against web-instance count | `UNKNOWN` | |
| Effective cache backend and shared-cache requirement recorded | `UNKNOWN` | |
| Effective queue mode recorded | `UNKNOWN` | |
| Worker supervision/retries/failed-job handling proven if async queueing is enabled | `NOT_APPLICABLE` | Change to `UNKNOWN` if asynchronous queues are enabled. |

Do not introduce infrastructure merely to make a checklist row green.

## 9. Mail

| Gate item | State | Evidence |
|---|---|---|
| Real production mail provider/transport proven without credentials | `UNKNOWN` | |
| Sender-domain readiness proven as applicable | `UNKNOWN` | SPF/DKIM/DMARC summary may be referenced without secrets. |
| Password-recovery delivery completes through the real production mail path | `UNKNOWN` | Do not retain reset URL/token. |
| Bounce/delivery monitoring ownership recorded | `UNKNOWN` | |

## 10. Logging, monitoring and alerting

| Gate item | State | Evidence |
|---|---|---|
| Selected centralized production log sink proven, or explicit absence recorded | `UNKNOWN` | |
| Effective structured-log format and request-ID preservation proven | `UNKNOWN` | Use redacted bounded sample. |
| Retention and access-control policy proven | `UNKNOWN` | |
| Metrics/alerting/on-call destination for critical failures proven | `UNKNOWN` | |
| Representative production logs reviewed for credential/token leakage | `UNKNOWN` | Do not copy sensitive logs into Git. |

## 11. Deployment, migrations and rollback

| Gate item | State | Evidence |
|---|---|---|
| Actual production release/deployment mechanism documented | `UNKNOWN` | |
| Production migration execution mechanism documented | `UNKNOWN` | |
| Rollback boundaries for application code and Platform-owned schema documented | `UNKNOWN` | |
| Emergency rollback operator/authorization path documented | `UNKNOWN` | |

## 12. Identity/admin critical production smoke

Execute only after hard preconditions are satisfied.

| Smoke item | State | Evidence |
|---|---|---|
| Home reachable through real production hostname | `UNKNOWN` | |
| Registration/login surfaces reachable through expected edge path | `UNKNOWN` | |
| Valid Platform Identity login/logout and stale-session fail-closed behavior | `UNKNOWN` | |
| Confirmed-MFA Identity completes real MFA challenge | `UNKNOWN` | Never retain TOTP secret/recovery code. |
| Password-recovery mail delivered and reset completed | `UNKNOWN` | Never retain reset token/link. |
| Password change completes and Platform sessions are revoked as expected | `UNKNOWN` | |
| Allowed admin permission path succeeds with confirmed MFA | `UNKNOWN` | |
| Missing-permission admin path is denied | `UNKNOWN` | |
| Controlled CMS mutation/publication observation/restore completes | `UNKNOWN` | |
| Corresponding privileged audit entry is visible to authorized audit viewer | `UNKNOWN` | |
| First-admin bootstrap is securely complete or has an approved production installation plan | `UNKNOWN` | Do not rerun closed bootstrap unnecessarily. |

## 13. Public game data, account and character production smoke

| Smoke item | State | Evidence |
|---|---|---|
| Read-only public game-data flow works through generic least-privilege legacy engine principal | `UNKNOWN` | |
| Runtime server state works through dedicated Redis principal when enabled | `UNKNOWN` | |
| New Platform Identity reaches ready immutable 1:1 legacy engine binding when provisioning is launch-enabled | `UNKNOWN` | |
| Provisioning retry/recovery behavior is operationally understood | `UNKNOWN` | |
| Bound Identity creates one character through dedicated character-create principal when enabled | `UNKNOWN` | |
| Character quota/name-conflict behavior matches deployed production schema/version | `UNKNOWN` | |
| Created character appears through expected exact-name public profile flow | `UNKNOWN` | |

Conditional account/character items may become `NOT_APPLICABLE` only when the selected launch scope explicitly disables those mutation surfaces, with rationale recorded below.

## 14. Authoritative game-login launch-scope decision

Current repository evidence does not prove a single authoritative Platform-to-game credential path. The current contract remains partially proven with credential-migration/bypass risks unresolved.

| Decision / gate | State | Evidence / rationale |
|---|---|---|
| Is Platform-originated authoritative game login required for this launch? | `UNKNOWN` | Must be explicitly decided before go-live conclusion. |
| If required: Platform-originated users enter the authoritative game-login path using Platform credential authority | `UNKNOWN` | Cross-repository proof required. |
| If required: exact binding, assertion/session expiry/audience, replay resistance and revocation proven end to end | `UNKNOWN` | Cross-repository proof required. |
| If required: alternate/direct bypass paths reviewed and controlled | `UNKNOWN` | Cross-repository proof required. |

If the launch requires Platform-originated game login, the Production Go-Live Gate remains blocked until the separately authorized bridge is implemented and proven end to end. If it is not part of launch scope, record the explicit scope decision and rationale; do not silently mark the bridge proven.

## 15. Edge and browser security smoke

| Smoke item | State | Evidence |
|---|---|---|
| Public hostname resolves through intended edge/reverse-proxy path | `UNKNOWN` | |
| TLS certificate/termination and HTTP-to-HTTPS behavior verified | `UNKNOWN` | |
| Origin exposure/firewall restriction verified | `UNKNOWN` | |
| CSP, X-Content-Type-Options, X-Frame-Options, Referrer-Policy and Permissions-Policy present as expected | `UNKNOWN` | |
| Production session cookies are Secure and HttpOnly with intended SameSite behavior | `UNKNOWN` | |
| HSTS behavior matches the recorded production topology decision | `UNKNOWN` | |

## 16. Backup and recovery evidence

| Gate item | State | Evidence |
|---|---|---|
| Current production backup schedule/scope/retention/encryption/access policy recorded | `UNKNOWN` | |
| Latest dated production restore test recorded | `UNKNOWN` | |
| Recovery time and data-loss observation recorded from production restore test | `UNKNOWN` | |

## Launch-scope declarations

Complete before final smoke:

| Scope decision | Value | Rationale |
|---|---|---|
| Registration enabled at launch | `UNKNOWN` | |
| Legacy engine account provisioning enabled at launch | `UNKNOWN` | |
| Character creation enabled at launch | `UNKNOWN` | |
| Runtime Redis-backed server status enabled at launch | `UNKNOWN` | |
| Administrator/CMS production mutations included in smoke | `UNKNOWN` | |
| Platform-originated authoritative game login required at launch | `UNKNOWN` | |

## Failures and remediation

Record failures without secrets. A failed mandatory item remains non-passing until reverified after remediation.

| Timestamp | Gate/smoke item | Result | Sanitized failure marker | Remediation/reference | Reverification |
|---|---|---|---|---|---|
| `UNKNOWN` | | | | | |

## Final decision

| Decision field | Value |
|---|---|
| Exact deployed Platform SHA evaluated | `UNKNOWN` |
| All mandatory applicable production gate items directly proven | `NO` |
| Final critical production smoke passed | `NO` |
| Required production restore evidence present | `NO` |
| Required game-login bridge proven or explicitly out of selected launch scope | `NO` |
| Production Go-Live Gate | `PENDING PRODUCTION VERIFICATION` |
| Production readiness classification | `STAGING_PROVEN` |

### Completion rule

Set the Production Go-Live Gate to `PASS` only when:

1. every mandatory launch-applicable production item is `PRODUCTION_PROVEN` with direct non-secret evidence tied to the exact deployed release;
2. all conditional `NOT_APPLICABLE` entries have an explicit valid launch-scope/topology rationale;
3. required production backup policy and dated restore evidence exist;
4. final critical production smoke passes;
5. the authoritative game-login requirement is either directly proven end to end when required or explicitly documented as outside the selected launch scope;
6. no unresolved mandatory failure remains.

Until then keep:

```text
Production Readiness: STAGING_PROVEN
Production Go-Live Gate: PENDING PRODUCTION VERIFICATION
Production Verification: REQUIRED BEFORE GO-LIVE
```
