# Oteryn Platform Incident and Recovery Runbook

## Status

Provider-neutral Phase 7 runbook baseline — 2026-07-20.

This runbook defines safe decision order and repository-owned checks. It intentionally does not invent provider-specific commands, production endpoints, private network details or backup tooling that are not proven by current deployment evidence.

Never paste secrets, credentials, tokens, private keys, production `.env` contents or unnecessary personal data into incident tickets, chat, Git commits or this document.

## Severity orientation

Treat an event as potentially critical when it involves any of:

- suspected credential/token/session/MFA-secret exposure;
- administrator privilege escalation or unauthorized role changes;
- unauthorized legacy engine shared writes;
- database corruption/loss or unverified restore state;
- broad authentication failure/bypass;
- confirmed remote code execution;
- loss of control of the production origin or secret-injection mechanism.

Exact organizational severity labels/on-call routing remain deployment/operations evidence and are currently `UNKNOWN`.

## Universal first response

1. Preserve evidence without copying secrets into public or repository-visible channels.
2. Identify the exact environment, observed time window and affected commit/release SHA if known.
3. Record representative server-generated `X-Request-ID` values where available.
4. Determine whether the incident is application-only, dependency-specific, shared-write related or infrastructure/provider related.
5. Prefer disabling the affected capability or failing closed over weakening authentication/authorization/privilege boundaries.
6. Do not rotate, revoke or destroy data blindly before identifying rollback/recovery dependencies.
7. After containment, run the relevant repository verifiers and regression suite on the candidate recovery version.

## Application production-configuration failure

Examples:

- debug accidentally enabled;
- non-production environment mode;
- missing application key;
- insecure cookie configuration;
- wrong/non-HTTPS application URL;
- non-delivery mail transport preventing password recovery.

Immediate actions:

1. Stop or block promotion of the affected release when safe to do so.
2. Run:

```text
php artisan production:verify-configuration
```

3. Correct effective environment configuration through the actual deployment/secret-management mechanism. Do not commit production secrets.
4. Re-run the verifier and health checks.
5. Re-test login/session/password-recovery behavior as applicable.
6. Record exact release SHA and corrected configuration class, not secret values.

If the deployment mechanism itself is unknown, escalate to the deployment owner rather than inventing a provider command.

## Suspected Platform credential or session compromise

Immediate containment:

1. Identify affected Platform Identity/identities without exposing credentials.
2. Use existing supported password-change/reset and session-revocation mechanisms where appropriate.
3. For administrator identities, review administrator audit events and role assignments.
4. Require MFA recovery/reset handling as a privileged security action when MFA factors are suspected compromised.
5. Review relevant request IDs and bounded application logs for the incident window.
6. Do not assume legacy engine/login-server sessions are revoked by Platform session revocation; cross-path revocation remains a separate contract boundary.

Recovery verification:

- Platform login works for the recovered Identity;
- previous Platform sessions are revoked according to policy;
- administrator access still requires confirmed MFA plus explicit permission;
- no unauthorized role assignment remains.

## Suspected administrator privilege escalation

1. Restrict administrator access at the strongest available application/edge boundary without removing forensic evidence.
2. Review `admin_audit_events` for bootstrap, role assignment/removal and privileged CMS changes.
3. Confirm current role assignments directly from Platform-owned RBAC state.
4. Remove unauthorized role assignments through the supported audited role-management path when possible.
5. Preserve at least one legitimate `platform_admin`; the supported path prevents deleting the final assignment.
6. Reset/recover affected administrator credentials/MFA as appropriate.
7. Verify deny-by-default routes with an authorized and unauthorized test identity before reopening access.

Cloudflare Access may be an additional containment layer only if deployment evidence proves it is actually configured. Do not rely on it otherwise.

## Suspected legacy engine generic read credential over-privilege

Run:

```text
php artisan legacy:verify-db-privileges
```

Expected behavior: fail closed if the effective generic legacy engine credential exceeds the approved read-only boundary.

Actions on failure:

1. Stop using the affected credential for Oteryn Platform where operationally safe.
2. Rotate/reprovision the credential through the actual secret-management/database owner process.
3. Re-run the verifier before restoring the integration.
4. Review whether unauthorized writes occurred using database/provider evidence.

Do not broaden application permissions to accommodate an over-privileged credential.

## Suspected legacy engine provisioning credential compromise/over-privilege

Run:

```text
php artisan legacy:verify-provisioning-db-privileges
```

If compromise is suspected or the verifier fails:

1. Disable/fence account provisioning capability where possible.
2. Rotate/reprovision only the dedicated provisioning credential.
3. Preserve the generic read-only and character-create credential separation.
4. Re-run the privilege verifier before re-enabling provisioning.
5. Review provisioning/binding state for incomplete `pending` records and use the supported bounded recovery command only after the credential boundary is restored:

```text
php artisan legacy:provision-pending-accounts --limit=100
```

6. Investigate unexpected legacy engine account rows using authorized database evidence.

Never expose the internal random compatibility credential to users.

## Suspected legacy engine character-create credential compromise/over-privilege

Run:

```text
php artisan legacy:verify-character-create-db-privileges
```

If compromise is suspected or the verifier fails:

1. Disable/fence character creation capability where possible.
2. Rotate/reprovision only the dedicated character-create credential.
3. Re-run the verifier before re-enabling character creation.
4. Review unexpected player rows/name/quota anomalies against the exact deployed legacy engine schema/version.
5. Do not use this credential for rename/delete/account mutations; those operations are not authorized by Phase 5 contracts.

## Legacy engine runtime Redis degradation

Symptoms may include unavailable/fail-closed runtime availability rather than fabricated offline/empty state.

Actions:

1. Confirm the application health route separately from runtime Redis behavior.
2. Verify whether the `legacy_runtime` dependency is reachable using the actual approved operational tooling; do not expose Redis credentials.
3. Confirm dedicated read-only ACL/user status if deployment evidence is available.
4. Treat stale/missing runtime data as dependency failure according to the application contract; do not extend freshness artificially with manual cache edits.
5. Escalate network/Redis/provider failures to the actual infrastructure owner.

Recovery evidence should include restored fresh runtime data and the relevant monitoring/alerting event if a production monitoring system exists.

## Mail delivery incident

1. Run `production:verify-configuration` to catch obvious non-delivery/test transport configuration.
2. Confirm provider/transport health using the actual mail provider evidence.
3. Verify password-recovery delivery end to end with a controlled test account.
4. Do not weaken password-recovery token security or bypass ownership checks to compensate for mail failure.
5. Record provider incident/delivery evidence without credentials or message bodies containing sensitive data.

## Logging/monitoring degradation

Application-side facts:

- requests receive a server-generated `X-Request-ID`;
- bounded completion events can be emitted;
- `stderr_json` is an available optional channel.

If centralized logs are missing:

1. Determine whether the application is still emitting logs locally/stderr using the actual deployment tooling.
2. Preserve request IDs from user/error reports where possible.
3. Verify sink/collector routing only with real deployment evidence.
4. Do not claim alerts were delivered unless the configured monitoring/on-call system proves it.
5. Restore collection/retention/access controls through the actual provider mechanism.

A working application-side logger does not prove a working centralized observability pipeline.

## Deployment regression or failed release

Because the actual deployment provider/release mechanism is not yet documented, this repository cannot provide a safe universal rollback command.

Decision order:

1. Identify the exact bad and last-known-good Oteryn Platform SHAs.
2. Identify whether the release executed Platform-owned database migrations.
3. Determine whether rollback is code-only, requires compatible schema state, or requires forward-fix/forward-recovery.
4. Use the provider/deployment owner's documented rollback mechanism.
5. Do not manually reverse database migrations in production without a reviewed data-preservation plan.
6. Re-run:
   - required CI on the recovery SHA;
   - `production:verify-configuration` against effective runtime configuration;
   - applicable legacy engine privilege verifiers;
   - targeted critical flows.
7. Record the incident timeline and root cause.

If no deployment/rollback runbook exists for the real provider, production readiness remains blocked.

## Platform database corruption/loss or restore event

Do not improvise a restore from undocumented backups.

1. Stop/fence writes if continued writes could worsen corruption, using the actual deployment controls.
2. Identify affected data scope and last known healthy point.
3. Escalate to the database/backup owner.
4. Verify backup authenticity/integrity and intended restore target before destructive action.
5. Restore only through the documented operational backup procedure.
6. After restore, verify at minimum:
   - Platform Identity/RBAC/CMS/audit/provisioning migrations/state;
   - immutable Identity-to-legacy engine bindings;
   - pending provisioning state consistency;
   - administrator role assignments;
   - recent security/admin audit availability.
7. Re-run critical application flows and applicable integration privilege verifiers.
8. Record measured recovery time and data loss.

Current repository state does not prove that an operational backup/restore mechanism exists. Until a dated restore test succeeds, production readiness remains blocked.

## Legacy engine shared-data inconsistency after partial failure

For account provisioning, use the existing forward-recovery model and bounded pending-provisioning command after verifying database privileges.

For character creation, rely on the operation contract's idempotent/recovery semantics and exact account binding; do not create ad-hoc raw shared-table fixes through the Platform.

If manual database repair is considered necessary:

- stop application automation affecting the same records;
- obtain database-owner review;
- preserve evidence/backups;
- document the exact invariant and repair;
- add a regression/contract test if the incident exposed a missing invariant.

## Authoritative game-login incident boundary

The Platform-authoritative game-login bridge is not implemented.

Do not claim that Platform password reset, MFA or session revocation controls current native/external game-login sessions globally.

Incidents involving game-login authentication/revocation require the actual legacy engine/login-server contract and may require separately authorized cross-repository action.

## Post-incident closure

Before closing a material incident:

1. Record exact affected/recovery SHAs and environment.
2. Record root cause and containment action without secrets.
3. Record which evidence was PROVEN versus inferred.
4. Run relevant repository regression/privilege/configuration gates.
5. Add regression tests when practical.
6. Update architecture/contracts/runbooks if a durable assumption changed.
7. Document remaining risk or owner-approved risk acceptance.
8. For backup/restore incidents, record the measured recovery result as operational evidence.

## Current environment-specific blockers

The following cannot be made executable from repository evidence alone:

- Cloudflare/WAF/Access incident commands;
- origin firewall/security-group changes;
- database provider failover/restore commands;
- production Redis/provider commands;
- centralized logging/metrics/alerting provider actions;
- deployment/rollback commands;
- backup restore commands.

These must be added only after the actual deployment topology is proven and reviewed.
