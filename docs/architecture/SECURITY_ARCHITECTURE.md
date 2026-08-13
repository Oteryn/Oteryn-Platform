# Oteryn Platform Security Architecture

## Purpose

This document defines mandatory security invariants for Oteryn Platform. It is a baseline for implementation and review, not proof that controls already exist.

## Security principles

1. **Defense in depth.** Cloudflare, reverse proxy, Laravel controls, database permissions and network isolation complement each other.
2. **Deny by default.** Missing or ambiguous authorization must fail closed.
3. **One authoritative identity policy.** Do not implement incompatible authentication policy independently in multiple components.
4. **Least privilege.** Users, administrators, services and database credentials receive only required capabilities.
5. **Explicit trust boundaries.** Browser input, API input and data from external/shared systems are untrusted until validated.
6. **No security by obscurity.** Hidden URLs or origin IP secrecy are not authorization controls.
7. **Security changes are testable.** Fixed vulnerabilities should gain regression tests where practical.

## Threat surfaces

Security-critical surfaces include:

- login/logout;
- password hashing and migration;
- session creation/revocation;
- MFA and recovery;
- password reset and email verification;
- administrator/RBAC operations;
- account and character mutations;
- file/media uploads;
- APIs and webhooks;
- shared database integration;
- future payments/coins/shop;
- infrastructure/origin exposure.

## Authentication

### Passwords

- Use Laravel/PHP framework-supported modern password hashing.
- Preferred target is Argon2id when operational compatibility is proven.
- Never store plaintext or reversibly encrypted passwords.
- Never log passwords.
- Legacy/hash migration must be designed against the actual login-server/Canary contract before implementation.

### Sessions

- Regenerate session identifiers on authentication/privilege transitions as appropriate.
- Revoke or rotate sessions after password reset/change, MFA reset or other security-sensitive state changes according to explicit policy.
- Session cookies must use appropriate Secure, HttpOnly and SameSite behavior for production deployment.
- Do not expose session IDs in URLs or logs.
- Registered Platform web sessions are ownership-scoped server-side; browser-supplied identifiers never establish ownership.
- A revoked, expired or generation-stale registered session must be invalidated before a protected controller executes and redirected to login; public routes may continue only as a guest request.
- Account surfaces may display only bounded browser/timestamp summaries. Raw source addresses are not user-visible and complete network identifiers do not enter audit metadata.

### MFA

Target requirement:

- mandatory MFA for administrator accounts before production readiness;
- optional or product-policy-driven MFA for normal users;
- secure enrollment confirmation;
- MFA secret protection at rest using framework/application encryption facilities where appropriate;
- recovery/reset procedures treated as privileged security actions;
- audit events for enrollment/reset/disable operations.

Oteryn uses authenticator-app TOTP as the delivered second factor. Email-code MFA is not adopted in the account-security lifecycle because the same email channel is already used for recovery; treating that channel as an independent second factor would weaken the intended assurance boundary.

### Recovery and verification

- reset/verification tokens must be cryptographically strong, time-limited and single-purpose;
- avoid account enumeration through materially different public responses;
- rate-limit recovery attempts;
- successful reset should apply the defined session revocation policy.

### Account security lifecycle

The delivered Platform-owned lifecycle additionally requires:

- primary-email changes must be confirmed at the new address and independently cancellable or recoverable from the previous address during a bounded window;
- confirmation and old-address recovery are single-use, rate-limited and revoke every Platform web session and game authorization;
- email-change cooldown prevents rapid repeated identity-channel replacement;
- privacy flags default to private and are read server-side by any public-profile consumer;
- a high-assurance recovery key is shown once, stored only as a keyed verifier and becomes unusable after rotation, revocation or successful use;
- successful recovery-key use resets password and MFA and revokes every Platform web session and game authorization;
- Platform account termination uses a grace period, explicit confirmation, cancellation and audited finalization;
- finalization disables and anonymizes Platform login but never deletes, unlinks, rebinds or transfers Canary-owned accounts or characters;
- the ready Platform-to-Canary binding remains immutable unless a separate reviewed operation contract authorizes a narrowly scoped exceptional mutation;
- security UI, validation, token errors and notification links support exactly English and Polish without persisting raw security tokens as locale state.

## Authorization and RBAC

Authorization is enforced server-side using policies/gates/application rules.

Minimum direction:

- separate content administration from account/security administration;
- avoid a single unrestricted admin flag as the only authorization model;
- privileged account actions require dedicated permissions;
- no web feature may provide arbitrary PHP/code execution or unrestricted plugin upload;
- administrative actions are audited.

Potential roles/permissions are not final until an RBAC ADR/task defines them.

### Support and moderation lifecycle

The delivered Platform-only support boundary requires:

- owner-scoped ticket, report and enforcement queries using server-resolved Identity ownership;
- separate exact administrator permissions for ticket, report and enforcement mutations, always composed with authentication and confirmed MFA;
- server-generated public identifiers, idempotent request keys and deterministic row locking/optimistic version checks;
- strict separation between user-visible fields and moderator-private notes/reporter metadata;
- bounded category, body, target, pending/open-count and application rate limits;
- audit metadata limited to identifiers, categories, status, booleans and lock versions, never private bodies or complete network identifiers;
- notification delivery failure recorded independently after the domain transaction;
- configurable retention with supported pruning/anonymization rather than direct SQL;
- no attachment upload and no compatibility-mode Canary ban mutation without separate reviewed contracts;
- native game sanctions cross only the accepted game-enforcement command/result boundary: exact moderator authorization and confirmed MFA gate the Platform decision, while service identity separately authenticates the bounded game command;
- free-form reports, appeal bodies, reporter identity, moderator-private notes, raw network/device values, credentials and secrets do not enter commands or ordinary cross-system telemetry;
- dispatch, queue acceptance, Platform state and notification never prove game enforcement; ambiguous outcomes reconcile the same operation identity and user-visible state remains truthful;
- monotonic decision revisions fence stale apply/replace/revoke/expire operations, and appeal submission cannot mutate game state before an authorized outcome.

## Browser security

- CSRF protection remains enabled for browser state-changing requests.
- Escape untrusted output by default.
- Rich HTML content requires explicit sanitization.
- Never rely only on hidden form fields for authorization.

Phase 7 repository-owned browser security headers are applied to Laravel `web` responses:

- `Content-Security-Policy` with same-origin default/script/style/connect/font boundaries, `form-action 'self'`, `base-uri 'none'`, `frame-ancestors 'none'` and `object-src 'none'`;
- `X-Content-Type-Options: nosniff`;
- `X-Frame-Options: DENY` as legacy frame-denial defense in depth;
- `Referrer-Policy: strict-origin-when-cross-origin`;
- `Permissions-Policy` disabling camera, geolocation, microphone, payment and USB capabilities by default.

Current first-party public/admin styling is loaded from a same-origin static asset so the enforced CSP does not require `style-src 'unsafe-inline'`. The implemented CSP does not grant `unsafe-eval` or inline-script execution.

HTTP Strict Transport Security is intentionally not hard-coded by the application at this stage. HSTS depends on the actual deployed TLS termination, proxy and hostname/subdomain policy and must be validated as part of the production edge/origin topology review before claiming it is safely deployed.

## Input and database security

- Validate input using framework request validation/domain rules.
- Use ORM/query builder/parameterized SQL.
- Never concatenate untrusted SQL.
- Enforce database constraints for durable invariants where appropriate.
- Use transactions for multi-step sensitive mutations.
- Use row locks/atomic updates where race conditions could violate balances, uniqueness or lifecycle rules.

## Admin protection

Target production posture:

```text
Administrator
    |
    v
Cloudflare Access (preferred additional gate)
    |
    v
Oteryn Platform login + mandatory MFA
    |
    v
Server-side RBAC policy
    |
    v
Audited privileged action
```

Cloudflare Access is additional protection. The application must still enforce authentication, MFA and authorization.

## Edge and origin protection

Recommended production direction:

- proxy public web/API traffic through Cloudflare;
- WAF/rate limiting/Turnstile where useful;
- restrict origin firewall to approved ingress paths when practical;
- database is not publicly exposed;
- internal services use private/explicit network rules;
- Canary game TCP protection requires a separate decision because standard HTTP proxying does not automatically protect arbitrary game ports.

These are target directions. Actual deployed Cloudflare, TLS and origin controls remain unproven until the Phase 7 topology evidence requirements are satisfied.

## Rate limiting and abuse prevention

Apply application-level limits at minimum to:

- login;
- registration;
- password reset;
- primary-email change request, confirmation and recovery;
- registered-session revocation;
- recovery-key generation, revocation and use;
- account-termination request and cancellation;
- email verification resend;
- MFA verification/recovery;
- public search endpoints if abused;
- support ticket create/reply/status;
- player/content/guild report submission;
- enforcement acknowledgement/appeal and administrator moderation mutations;
- expensive API endpoints.

Cloudflare limits may supplement, not replace, application limits.

## Audit and logging

Audit security-relevant events such as:

- admin privilege changes;
- sensitive account state changes;
- MFA enrollment/reset/disable;
- password reset completion;
- email change request, confirmation, cancellation and recovery;
- targeted and all-other session revocation actions;
- privacy setting changes;
- recovery-key generation, revocation and use;
- account-termination request, cancellation and finalization;
- support ticket replies/status changes, report outcomes and enforcement/appeal changes;
- future payment/ledger administrative actions.

Phase 7 application-side request correlation uses a fresh server-generated UUID for every Laravel-handled request. The application does not trust a browser-supplied `X-Request-ID` as authoritative correlation evidence.

Normal responses expose the generated identifier as `X-Request-ID`. Request completion logging is deliberately bounded to:

- request ID;
- HTTP method;
- route name when available;
- response status;
- bounded request duration.

The request-completion event does not include full URLs, query strings, request bodies, request headers or credential values. This keeps correlation useful while reducing accidental token/personal-data leakage.

An optional JSON-to-stderr channel exists for deployment environments that collect process stderr. It is not proof that a centralized production log, metrics or alerting sink is deployed, and it does not change the default local logging channel.

Never record:

- raw passwords;
- session tokens or complete registered-session identifiers;
- reset or email-change tokens;
- MFA secrets;
- raw recovery keys;
- complete source addresses;
- payment credentials;
- unnecessary personal data.

## Secrets

- `.env` is never committed.
- `.env.example` contains placeholders only.
- production secrets are injected by deployment/secret-management tooling.
- credentials should be rotated when exposure is suspected.
- different components should use different least-privilege database/service credentials where practical.

## Shared Canary/login-server security

No security guarantee may rely on a rule enforced only by Oteryn Platform if another exposed path can bypass it.

Before production, prove end-to-end behavior for:

- credential validation;
- account disabled/banned state;
- email verification policy if required for game login;
- MFA policy if intended to gate game login;
- session/token creation;
- session/token replay behavior;
- password-change/reset revocation;
- direct Canary/login-server bypass paths.

Any unresolved bypass is a blocker for claiming the policy is enforced globally.

## File uploads

No upload functionality should be added casually.

When introduced it requires:

- explicit allowed types;
- server-side MIME/content validation;
- size limits;
- generated storage names;
- storage outside executable application paths;
- authorization;
- image processing safety where applicable;
- malware scanning decision based on threat/use case.

## Future payments

Payments are out of initial scope. Before introduction require a dedicated threat model covering:

- webhook authenticity;
- idempotency;
- replay protection;
- ledger integrity;
- concurrent purchase handling;
- refunds/chargebacks;
- reconciliation;
- administrator abuse controls;
- auditability.

The platform must not store raw card data unless a future explicitly approved architecture and compliance scope requires it; prefer hosted/provider-controlled payment flows.

## Production readiness security gate

A production release must not be called security-ready until at least:

- auth/session contract with game login is proven;
- administrator MFA is enforced;
- critical routes have authorization tests;
- password reset/revocation behavior is tested;
- email-change, registered-session, recovery-key and termination behavior is tested in both supported locales;
- origin/database exposure is reviewed;
- secrets handling is verified;
- browser security headers are regression-tested and deployed TLS/HSTS posture is reviewed against the actual topology;
- critical dependencies pass the required vulnerability advisory scan;
- backups/restore procedure exists and is operationally tested;
- security-sensitive audit events are available;
- request correlation is available and deployed logging/alerting handling is verified against the actual production sink;
- known critical/high security findings are resolved or explicitly accepted by the owner.

## Public community data controls

Public community reads use a dedicated Canary credential with direct `SELECT` only on `players`, `guilds`, `guild_membership`, `guild_ranks`, `houses`, `player_deaths`, `channels` and `cluster_sessions`. Effective grants are verified without write probes; schema-wide, role-indirect, grant-option and write privileges fail closed.

Public profile assembly applies Platform privacy flags before presentation and excludes internal Identity/binding IDs, account IDs, email, IP data, runtime lease internals, raw death participant payloads, house coordinates and moderator-only enforcement data. Highscore sort columns come from a fixed application allowlist, while category/vocation inputs are validated against bounded enumerations.

Canary/Platform query failures render localized `503` states without SQL or credential disclosure. Empty and not-found responses are never used to hide dependency failure. Guild administration, public enforcement publication and every Canary mutation remain outside this read-only boundary.


## Character profile preference security

Character-profile management is authenticated and owner-scoped by server-resolved ready binding plus current read-only Canary ownership. Browser-supplied player/account identifiers do not authorize writes. Public comments are length-bounded, control-normalized and escaped at render time; audit events exclude comment content. Account privacy remains an upper bound, hidden sibling associations are filtered and generic dependency failures do not disclose SQL details. Main-character selection locks the Identity row and real-MariaDB race acceptance must leave exactly one main. No Canary mutation is authorized.
