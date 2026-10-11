# ADR 0006 — Administrator RBAC and privileged audit policy

## Status

Accepted — 2026-07-20

## Context

Phase 6 requires privileged CMS and administrator capabilities without turning authentication or MFA into authorization and without introducing a single unrestricted administrator flag.

The existing Platform Identity boundary already provides authenticated web sessions and a reusable `mfa.confirmed` gate. Those controls establish who the user is and whether the required second factor is confirmed; they do not decide which privileged operation the Identity may perform.

The administrator model must therefore:

- fail closed when authorization state is missing or unknown;
- separate content administration from security/role administration;
- make privilege escalation an explicit capability;
- require confirmed MFA independently from RBAC;
- record administrator mutations without storing secrets;
- provide a safe path to create the first administrator without leaving a permanent authorization bypass.

## Decision

### 1. Explicit permissions are the authorization unit

Privileged web routes require one exact permission key through the server-side `admin.permission` boundary.

The initial permission registry is:

- `admin.access` — enter the administrator surface;
- `admin.roles.manage` — assign and remove administrator roles;
- `audit.view` — view bounded administrator audit records;
- `cms.news.manage` — create and update Platform news posts;
- `cms.pages.manage` — create and update Platform managed pages.

Unknown permission keys fail closed.

There is no wildcard permission and no implicit `is_admin` bypass.

### 2. Initial roles are explicit permission bundles

The initial roles are:

- `content_editor`:
  - `admin.access`;
  - `cms.news.manage`;
  - `cms.pages.manage`;
- `security_admin`:
  - `admin.access`;
  - `admin.roles.manage`;
  - `audit.view`;
- `platform_admin`:
  - all five permissions listed in this ADR.

`platform_admin` is not a wildcard. A future permission is not automatically granted merely because an Identity already has the `platform_admin` role. Adding a new privileged capability requires an explicit permission and an explicit role-permission decision.

### 3. Durable RBAC persistence is Platform-owned

Oteryn Platform owns:

- administrator roles;
- administrator permissions;
- role-permission mappings;
- Identity-role assignments.

No administrator role is assigned by database migration or normal user registration.

Legacy engine account identity, group fields or game-server privileges are not administrator authorization evidence for Oteryn Platform.

### 4. Privileged web access composes three independent gates

Every administrator web capability composes:

`auth` + `mfa.confirmed` + `admin.permission:<exact-permission>`

Passing one gate never implies the others.

MFA does not grant an administrator role. An administrator role does not bypass MFA. Authentication alone does not grant a privileged permission.

### 5. First-administrator bootstrap is one-time and console-only

The first `platform_admin` assignment is created through a dedicated console bootstrap operation.

The bootstrap operation:

- targets an existing Platform Identity by canonical email;
- requires that target Identity to have confirmed MFA;
- succeeds only while no administrator role assignment exists;
- serializes through the administrator-role transaction boundary;
- records an administrator audit event with a bootstrap/system actor;
- becomes unavailable after the first administrator assignment exists.

There is no permanent unrestricted console role-grant command and no web bootstrap endpoint.

### 6. Role lifecycle is an audited privileged operation

Role assignment/removal requires `admin.roles.manage` plus authenticated confirmed-MFA administrator context.

Role changes are transactional and append an administrator audit event only when durable state changes.

The final `platform_admin` assignment cannot be removed through the supported role-management operation. This prevents the supported application path from leaving the installation without any holder of the complete initial Phase 6 permission bundle.

### 7. Privileged CMS mutations are permission-scoped and audited

News mutations require `cms.news.manage`.

Managed-page mutations require `cms.pages.manage`.

The first Phase 6 CMS authoring surface remains plain text. Public output is escaped. Rich HTML, media uploads and arbitrary plugin/code upload are not part of this decision.

CMS state mutation and its administrator audit append occur in the same Platform database transaction where practical.

### 8. Administrator audit is append-oriented and bounded for viewing

Administrator audit events record:

- actor Identity reference when an authenticated actor exists;
- action key;
- target type and optional target identifier;
- minimal non-secret metadata;
- occurrence time.

Audit events must not contain passwords, session tokens, reset tokens, MFA secrets or other credentials.

Viewing audit events requires `audit.view` plus authenticated confirmed MFA and is paginated with a bounded page size.

Audit storage is not a substitute for infrastructure/application logs.

### 9. Cloudflare Access is optional defense in depth

A production deployment may place Cloudflare Access in front of the administrator path family or a dedicated administrator hostname.

Access approval never creates an Oteryn role or permission and never replaces Platform authentication, confirmed MFA or server-side RBAC.

The exact Access hostname/path policy remains environment-specific because the production routing topology is not yet proven.

## Security consequences

- Missing role/permission state fails closed.
- A compromised normal user session cannot reach privileged routes without both confirmed MFA state and an explicit permission.
- Content administrators do not receive role-management or audit permissions by default.
- Security administrators do not receive CMS authoring permissions by default.
- `admin.roles.manage` is explicitly privilege-escalating because it can grant roles; it must therefore remain a dedicated audited permission.
- Future privileged capabilities require explicit permission modeling and tests rather than inheriting an implicit superuser wildcard.
- Direct out-of-band database manipulation remains outside the supported application authorization path and must be controlled operationally.

## Rejected alternatives

### Single boolean administrator flag

Rejected. It cannot express separation between CMS authoring, role management and audit visibility and would violate deny-by-default least privilege.

### Wildcard or implicit super-administrator permission

Rejected. Future capabilities would silently become available without an explicit authorization decision.

### Treat confirmed MFA as administrator authorization

Rejected. MFA proves a second factor; it does not grant a business or security permission.

### Permanent unrestricted console role assignment

Rejected. It would remain an application-supported RBAC bypass after initial installation. The bootstrap path therefore closes after the first assignment exists.

### Cloudflare Access as the only administrator gate

Rejected. Edge access is defense in depth and cannot replace application authentication, MFA, RBAC or audit.

### Rich HTML or plugin/media upload as part of initial CMS management

Rejected. Those features introduce separate sanitization, file-content and executable-upload threat surfaces and are not required for Phase 6 completion.

## Follow-up

Any new administrator capability must:

1. define an explicit permission key;
2. decide explicitly which roles receive it;
3. enforce `auth` + `mfa.confirmed` + the exact permission server-side;
4. add authorization regression coverage;
5. audit privileged state changes where applicable.

Production hardening may deploy the optional Cloudflare Access boundary described in `docs/operations/CLOUDFLARE_ACCESS_ADMIN.md`, but Phase 7 must still validate the actual production topology and origin bypass resistance.
