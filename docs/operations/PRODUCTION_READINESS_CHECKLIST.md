# Oteryn Platform Production Go-Live Gate

## Status

- **Phase 7 — Production hardening and operations: COMPLETE** under ADR 0007.
- **Production Readiness: STAGING_PROVEN**.
- **Production Go-Live Gate: PENDING PRODUCTION VERIFICATION**.
- **Production Verification: REQUIRED BEFORE GO-LIVE**.

This file retains its existing repository path for continuity, but ADR 0007 reclassifies it from a Phase 7 engineering exit checklist into the authoritative fail-closed Production Go-Live Gate.

A checked repository or staging control is not proof that the equivalent production infrastructure is deployed. Phase 7 completion does not satisfy this gate.

Do not paste secrets, credentials, private connection strings, private IP inventories or copied production `.env` files into this document or pull requests.

## Gate states

The project already uses environment evidence classifications in `docs/operations/PRODUCTION_LIKE_VALIDATION_EVIDENCE.md`:

- `STAGING_PROVEN` — directly demonstrated in controlled production-like validation;
- `PRODUCTION_PROVEN` — directly demonstrated in the final production environment;
- `UNKNOWN` — not yet proven for the stated environment.

This checklist also uses supporting control labels:

- `REPO-PROVEN` — deterministically verifiable from the repository and required CI;
- `ENV-EVIDENCE-REQUIRED` — requires direct evidence from the actual deployed production environment and remains effectively `UNKNOWN` for go-live until proven;
- `CROSS-REPO-BLOCKED` — depends on a separately authorized external repository programme.

`REPO-PROVEN` and `STAGING_PROVEN` evidence can support a release decision, but neither promotes a production-specific item to `PRODUCTION_PROVEN`.

The gate fails closed: it cannot become `PASS` while any mandatory production verification item remains `UNKNOWN` or lacks direct production evidence.

An explicit owner risk decision, where repository policy permits, is a governance decision rather than evidence. It does not convert `UNKNOWN` or `STAGING_PROVEN` into `PRODUCTION_PROVEN`, cannot be used to claim that an unverified production fact was verified, and cannot replace mandatory proof of the actual deployed release identity and core production environment boundaries.

## Supporting production-like evidence

Merged PR #63 closed the staging-verifiable engineering/hardening scope. Its final PR head `7842f78ec4ac2d07d3800ffe8bde9809b055822d` passed Phase 7 Production-Like Validation #9, required CI #759 and Agent Governance #679.

That evidence is `STAGING_PROVEN` only and covers controlled deployment/migrations/rollback/redeploy, database least-privilege verifiers, Redis ACL behavior, SMTP delivery/failure, critical-flow regression coverage, live security/request-correlation/logging smoke and backup/clean restore/integrity checks.

The final-head controlled restore measured `105 ms` with `13/13` tables, `11/11` migrations and a matching validation-SHA probe. This is staging recovery evidence only and is not production RTO/RPO.

## 1. Exact release identity

Before any production-ready or go-live `PASS` claim:

- [ ] Record the exact Oteryn Platform commit SHA being evaluated. `ENV-EVIDENCE-REQUIRED`
- [ ] Record exact deployed legacy engine/login-server versions relevant to authentication/game-login behavior. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove the evaluated commit passed required CI, including Composer advisory audit, Pint, PHPStan and the full test suite. `REPO-PROVEN`
- [ ] Confirm there is no newer unreviewed production-only code/config divergence outside the declared deployment mechanism. `ENV-EVIDENCE-REQUIRED`

## 2. Provider-independent application configuration

Run against the effective production configuration without printing secrets:

```text
php artisan production:verify-configuration
```

Required result: exit code `0`.

The verifier proves only these application invariants:

- `APP_ENV=production`;
- debug disabled;
- application encryption key configured;
- HTTPS, non-localhost/loopback application URL;
- Secure and HttpOnly session cookies;
- delivery-capable default mail transport;
- valid non-test sender address.

- [ ] `production:verify-configuration` passes against the effective production runtime. `ENV-EVIDENCE-REQUIRED`

Do not infer database, Redis, queue, logging-provider or Cloudflare correctness from this command.

## 3. Dependency and application security gates

- [x] Required CI runs `composer audit --no-interaction`. `REPO-PROVEN`
- [x] Current repository uses deny-by-default administrator authorization with exact permissions. `REPO-PROVEN`
- [x] Administrator web routes require Platform authentication and confirmed MFA. `REPO-PROVEN`
- [x] CSP/browser security headers have regression coverage. `REPO-PROVEN`
- [x] Application request correlation is server-generated and does not trust inbound request IDs. `REPO-PROVEN`
- [ ] Re-run required CI for the exact production candidate SHA. `ENV-EVIDENCE-REQUIRED`
- [ ] Review unresolved critical/high dependency or application findings and resolve or explicitly record an eligible owner risk decision. `ENV-EVIDENCE-REQUIRED`

A risk decision does not substitute for direct verification of mandatory production environment facts.

## 4. Edge, TLS and origin exposure

Required evidence is defined in `docs/operations/PRODUCTION_TOPOLOGY_EVIDENCE.md`.

- [ ] Prove production DNS/proxy mode and TLS termination. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove effective WAF/rate-limit/Access policy where used. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove whether direct-origin access is blocked or explicitly risk-accepted where policy permits. `ENV-EVIDENCE-REQUIRED`
- [ ] Review ingress firewall/security-group/reverse-proxy rules. `ENV-EVIDENCE-REQUIRED`
- [ ] Decide HSTS only after the real TLS and hostname/subdomain policy is proven; record the decision and rationale. `ENV-EVIDENCE-REQUIRED`

The application intentionally does not hard-code HSTS before this evidence exists.

## 5. Platform database

- [ ] Record the production database engine/topology without credentials. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove the database is not publicly exposed except for explicitly approved network paths. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove credential injection/rotation ownership. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove backup scope, schedule, retention, encryption/access policy and restore ownership. `ENV-EVIDENCE-REQUIRED`
- [ ] Complete and record a dated production restore test with scope, result, recovery time and data-loss observation. `ENV-EVIDENCE-REQUIRED`

Go-live `PASS` is blocked until the applicable production restore evidence exists. Staging restore measurements do not establish production RTO/RPO.

## 6. Legacy engine SQL privilege boundaries

The repository provides these verifiers:

```text
php artisan canary:verify-db-privileges
php artisan canary:verify-provisioning-db-privileges
php artisan canary:verify-character-create-db-privileges
```

- [ ] Run the generic legacy engine read-only verifier against the effective production credential. `ENV-EVIDENCE-REQUIRED`
- [ ] Run the provisioning verifier against the effective production provisioning credential before enabling provisioning writes. `ENV-EVIDENCE-REQUIRED`
- [ ] Run the character-create verifier against the effective production character-create credential before enabling character creation writes. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove production network paths and credential classes are separate as designed. `ENV-EVIDENCE-REQUIRED`

Failure of any effective-grant verifier is a fail-closed deployment blocker for that capability. Staging principals do not prove production effective grants.

## 7. Legacy engine runtime Redis

- [ ] Prove the production runtime Redis endpoint and network boundary without secrets. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove the dedicated read-only ACL/user is provisioned in production. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove production Redis transport/TLS posture where applicable. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove monitoring/alerting for dependency failure and freshness behavior. `ENV-EVIDENCE-REQUIRED`

Staging ACL validation does not prove the production ACL or network boundary.

## 8. Sessions, cache and queue

Repository defaults and staging choices are not production topology evidence.

- [ ] Record the effective production session backend and justify it against the number of web instances. `ENV-EVIDENCE-REQUIRED`
- [ ] Record the effective cache backend and whether shared cache is operationally required. `ENV-EVIDENCE-REQUIRED`
- [ ] Record the effective queue mode. Current repository behavior is synchronous; if asynchronous queues are introduced, prove worker supervision, retries and failed-job handling. `ENV-EVIDENCE-REQUIRED`

Do not add Redis or asynchronous infrastructure merely to satisfy this checklist; introduce only what the proven topology/use case requires.

## 9. Mail

- [ ] Prove real production mail transport/provider without exposing credentials. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove sender-domain readiness as applicable to the chosen provider. `ENV-EVIDENCE-REQUIRED`
- [ ] Verify password recovery delivery end to end through the production mail path. `ENV-EVIDENCE-REQUIRED`
- [ ] Record bounce/delivery monitoring ownership. `ENV-EVIDENCE-REQUIRED`

The successful staging SMTP path is `STAGING_PROVEN`; it is not proof of the production provider or sender-domain configuration.

## 10. Logging, monitoring and alerting

Repository/staging-proven application primitives:

- server-generated `X-Request-ID`;
- bounded `http.request.completed` event context;
- optional JSON-to-stderr channel;
- Identity security-event audit;
- administrator audit;
- representative production-like secret-leakage smoke.

Production evidence still required:

- [ ] Prove the selected centralized production log sink or record that no centralized sink exists. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove effective production structured-log format and request-ID preservation. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove retention and access-control policy. `ENV-EVIDENCE-REQUIRED`
- [ ] Prove metrics/alerting/on-call destination for critical application/dependency failures. `ENV-EVIDENCE-REQUIRED`
- [ ] Confirm representative production logs do not expose credentials/tokens. `ENV-EVIDENCE-REQUIRED`

## 11. Deployment and rollback

Controlled production-like procedure evidence is `STAGING_PROVEN` for clean deployment, migrations, rollback, interrupted-release isolation and redeploy. The final provider implementation remains unproven.

- [ ] Document the actual production release/deployment mechanism. `ENV-EVIDENCE-REQUIRED`
- [ ] Document when/how production migrations are executed. `ENV-EVIDENCE-REQUIRED`
- [ ] Document production rollback boundaries for application code and Platform-owned schema changes. `ENV-EVIDENCE-REQUIRED`
- [x] Prove rollback or forward-recovery procedure in a safe non-production or controlled environment. `STAGING_PROVEN`
- [ ] Identify who can execute emergency production rollback and how access is controlled. `ENV-EVIDENCE-REQUIRED`

No provider-specific rollback command is committed because the provider/deployment model is currently unproven.

## 12. Identity/admin critical flows

The full exact-SHA production-like regression suite is `STAGING_PROVEN`. For the exact production candidate/environment, final smoke remains required:

- [ ] Web login/logout. `ENV-EVIDENCE-REQUIRED`
- [ ] Password reset delivery and completion. `ENV-EVIDENCE-REQUIRED`
- [ ] Password change and Platform session revocation. `ENV-EVIDENCE-REQUIRED`
- [ ] MFA enrollment/challenge/recovery. `ENV-EVIDENCE-REQUIRED`
- [ ] Administrator MFA plus allowed/denied RBAC paths. `ENV-EVIDENCE-REQUIRED`
- [ ] First-admin bootstrap procedure has either been completed securely or has an approved production installation plan. `ENV-EVIDENCE-REQUIRED`

## 13. Account/character shared writes

Production-like provisioning/binding and character-creation coverage is `STAGING_PROVEN`. If enabled in the target production environment:

- [ ] Greenfield account provisioning succeeds under the verified production operation-specific principal. `ENV-EVIDENCE-REQUIRED`
- [ ] Provisioning retry/recovery behavior is operationally understood for production. `ENV-EVIDENCE-REQUIRED`
- [ ] Character creation succeeds under the verified production operation-specific principal. `ENV-EVIDENCE-REQUIRED`
- [ ] Character quota/name-conflict behavior is verified on the deployed production schema/version. `ENV-EVIDENCE-REQUIRED`

Do not enable uncontracted character/account mutations.

## 14. Authoritative game login

If Platform-originated authoritative game login is part of the selected launch scope:

- [ ] Platform-originated users can enter the authoritative game-login path using Platform credential authority. `CROSS-REPO-BLOCKED`
- [ ] Exact-account binding, assertion expiry/audience, replay resistance and revocation semantics are proven end to end. `CROSS-REPO-BLOCKED`
- [ ] Direct bypass paths are reviewed. `CROSS-REPO-BLOCKED`

Current state: this bridge is not implemented. ADR 0007 keeps it outside Phase 7 engineering completion, but the Production Go-Live Gate remains blocked on it when the selected launch scope requires Platform-originated game login.

## Production Go-Live decision

A go-live `PASS` requires:

1. all applicable repository gates passing on the exact production candidate SHA;
2. all mandatory applicable production environment items directly verified and recorded without secrets;
3. the required production backup policy plus dated production restore evidence completed;
4. critical/high findings resolved or handled by an explicit eligible owner decision without fabricating production evidence;
5. the authoritative game-login requirement resolved if Platform-originated game login is part of launch scope;
6. final critical production smoke/E2E checks passing against the exact deployed SHA.

Until these conditions are satisfied, **Production Go-Live Gate remains PENDING PRODUCTION VERIFICATION** and production verification remains required before go-live.

Phase 7 engineering completion is independent from this gate under ADR 0007. No production-ready or go-live `PASS` claim follows from `STAGING_PROVEN` evidence alone.
