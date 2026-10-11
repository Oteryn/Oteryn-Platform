# Oteryn Platform Delivery Roadmap

## Goal

Replace MyAAC with a first-party Oteryn web/application platform without coupling the project to speculative legacy engine or login-server assumptions.

The roadmap is ordered by risk: establish contracts and identity correctness before adding broad mutation features or payments.

## Phase 0 — Architecture and agent bootstrap

**Status: COMPLETE**

Deliverables:

- agent governance and durable task/checkpoint workflow;
- product/system architecture;
- module catalog;
- security architecture;
- data ownership policy;
- cross-repository contract placeholders;
- ADRs for initial direction;
- project state and roadmap.

Exit gate:

- a fresh agent can continue without chat history;
- unknown integration facts are explicitly listed rather than assumed.

## Phase 1 — Laravel application bootstrap

**Status: COMPLETE**

Deliverables:

- supported PHP/Laravel version selected from current maintained releases at implementation time;
- clean Laravel application skeleton;
- Blade-based initial frontend;
- environment template with no secrets;
- test framework and baseline test;
- formatter/linter/static-analysis decision;
- CI for install + tests + lint/static checks;
- local development setup documentation;
- basic health endpoint.

Exit gate:

- reproducible local install;
- clean CI on main;
- no production credentials in repository.

## Phase 2 — legacy engine and login authentication discovery

**Status: COMPLETE FOR CURRENT IMPLEMENTATION BOUNDARIES**

Deliverables:

- verified account/player/guild schema references from actual Oteryn legacy engine;
- verified login-server repository/component and interface;
- password/hash compatibility evidence;
- session/token flow evidence;
- account ban/status semantics;
- game-login revocation behavior;
- world model: single-world or multi-world;
- completed legacy engine/auth contracts in `docs/contracts/**`.

Exit gate:

- no critical auth/data integration question remains `UNKNOWN` for the next implementation scope.

Unresolved production/global-auth facts remain explicit blockers for later authoritative game-login migration; completing Phase 2 does not convert those unknowns into assumptions.

## Phase 3 — Identity foundation

**Status: COMPLETE**

Delivered:

- Platform-owned registration policy and Identity persistence;
- secure Platform web login/logout;
- revocable Platform web-session generation and session invalidation;
- password recovery and authenticated password change with Platform web-session revocation;
- current credential strategy: Platform Identity credentials remain Platform-owned and framework-hashed, with no shared legacy engine password migration/write until the authoritative-Identity rollout gates in `AUTH_GAME_LOGIN_CONTRACT.md` are satisfied;
- current email-verification policy: not required for Phase 3 product scope and therefore intentionally not enabled; any future global requirement must account for alternate legacy engine/login-server authentication paths;
- layered rate limiting for registration, login, recovery, password change and MFA flows;
- append-oriented account security event audit primitives for implemented Identity security events;
- complete opt-in Platform web MFA enrollment, challenge, recovery-code, replay-prevention, disable and session-revocation lifecycle using a maintained TOTP provider;
- reusable `mfa.confirmed` middleware foundation for future privileged routes;
- administrator authentication policy foundation: future privileged/Admin routes must combine `auth`, explicit Phase 6 RBAC/policy authorization and mandatory `mfa.confirmed`; the MFA gate does not classify administrators or grant authorization;
- auth, MFA, rate-limit, session and revocation regression coverage.

Exit gate:

- end-to-end Platform web auth works;
- current game-login compatibility is preserved by credential-boundary non-interference: Phase 3 does not mutate legacy engine reusable credentials or game sessions;
- password reset/change revokes Platform web sessions and does not claim to revoke unrelated legacy engine/login-server credentials;
- administrator authentication policy is defined and the confirmed-MFA gate is tested independently from the future Phase 6 role/permission model.

Phase 3 completion does **not** claim that Platform MFA, email verification, password change or password reset is globally enforced for native legacy engine or external login-server authentication. The cross-path authoritative Identity migration remains a separate later programme governed by `AUTH_GAME_LOGIN_CONTRACT.md`.

## Phase 4 — Public website and read-only game data

**Status: COMPLETE**

Delivered:

- shared public Blade layout/navigation for Home, News, Online, Highscores and Servers;
- homepage exact-name character search routed to the existing bounded character profile read path;
- configured server/channel metadata plus fresh per-channel runtime availability/count projection through the dedicated read-only `canary_runtime` Redis boundary;
- Platform-owned published-only public news list/detail with deterministic pagination and escaped plain-text rendering;
- read-only active character profiles;
- read-only level highscores with deterministic ordering and pagination;
- read-only guild detail plus joined paginated membership reads without per-member N+1 queries;
- cluster-wide online-character list from fresh `cluster_sessions` identity joined to public player/channel fields, with explicit dependency-failure semantics and pagination capped at 100 rows per page;
- dedicated read/query boundaries using explicit public field allowlists and database-enforced SELECT-only legacy engine privileges for the implemented SQL read surface;
- caching intentionally absent where it could extend `cluster_sessions` lease expiry or legacy engine runtime Redis TTL freshness.

Closure revalidation found one concrete exit-gate gap: `onlineCharacters()` still terminated in an unbounded `get()`. PR #23 replaced that mass-query path with a `LengthAwarePaginator` defaulting to 100 rows and added route-level regression coverage proving 101 fresh online characters split across two pages.

Known privileged/group-hidden ranking policy, production runtime Redis ACL/endpoint provisioning, exact production wall-clock skew and broader cache policy remain explicit later product/deployment unknowns. They are not silently resolved by Phase 4 completion and do not authorize shared writes or deployment claims.

Exit gate — satisfied by closure revalidation:

- no write access is required for public game-data features;
- query performance avoids obvious N+1/mass-query patterns through bounded lookups, joins and pagination;
- public output is escaped/sanitized correctly for the implemented surfaces.

## Phase 5 — Account and character management

**Status: COMPLETE**

Delivered greenfield scope:

- Oteryn Platform is the authoritative owner of user Identity, account lifecycle policy and credentials;
- immutable `1 Platform Identity <-> 1 legacy engine accounts.id` ownership binding for supported greenfield accounts;
- Platform-originated legacy engine account provisioning with durable pending/ready/conflict state and forward recovery;
- separate least-privilege `canary_provisioning` database principal restricted to the approved account-create/recovery columns;
- non-user random sink credential strategy that preserves the current required legacy engine password representation without making legacy engine reusable passwords a user authentication authority;
- authenticated character creation authorized exclusively through the ready immutable binding;
- ADR 0005 canonical character-name, starter-state, allowed-vocation/sex and maximum-10-active-character product policy;
- separate least-privilege `canary_character_create` database principal restricted to the approved account/player columns;
- locked account-row transaction, natural idempotent recovery, active-character quota enforcement and global-name conflict handling;
- fail-closed effective-grant verifiers and reviewed SQL provisioning templates for both write principals;
- real MariaDB integration coverage for account trigger side effects, denied excessive privileges, provisioning retry/recovery, character starter/default shape, account locking, quota races and global same-name races.

Supported Phase 5 shared-write surfaces are exactly:

1. Platform-originated legacy engine account provisioning governed by the retired legacy-engine contracts (see the ADR index) and the immutable binding contract;
2. Platform-driven character creation governed by `CHARACTER_CREATION_CONTRACT.md` and ADR 0005.

Character deletion/soft deletion and rename/lifecycle operations were not selected for the delivered greenfield scope. They remain optional future capabilities and are forbidden until separately contracted, least-privileged and tested. Existing-account claim/import is outside the greenfield product model.

The separately required authoritative Platform game-login bridge is **not** part of the Phase 5 shared-write exit gate and is not claimed as implemented. Platform-originated accounts currently carry an intentionally undisclosed random sink credential; enabling user game login under Platform credential authority requires separately authorized cross-repository integration with explicit expiry, replay/session and revocation semantics.

Exit gate — satisfied by closure revalidation:

- every implemented shared write is documented by an explicit operation-specific contract;
- authorization, idempotency/failure and concurrency invariants are covered by unit/feature and real MariaDB integration tests;
- the generic `legacy` connection remains database-enforced read-only;
- shared mutations are isolated to the two dedicated operation-specific least-privilege connections above;
- no additional undocumented raw legacy engine write path is approved or claimed by Phase 5.

## Phase 6 — CMS, Admin, RBAC and Audit

**Status: COMPLETE**

Delivered:

- Platform-owned news management with published-only public reads and permission-scoped create/update administration;
- Platform-owned managed pages with published-only public reads, permission-scoped create/update administration and escaped plain-text rendering;
- durable explicit role/permission persistence and Identity-role assignment with no administrator assigned by default;
- explicit initial permission registry and role bundles governed by ADR 0006, with no wildcard or implicit unrestricted administrator authorization path;
- one-time console-only first `platform_admin` bootstrap requiring an existing MFA-confirmed Platform Identity and closing after the first administrator assignment exists;
- administrator role assignment/removal behind `admin.roles.manage`, with transactional audit recording and supported-path protection against removing the final `platform_admin`;
- dedicated privileged routes that independently combine `auth`, Phase 3 `mfa.confirmed` and the exact `admin.permission:<permission>` required by the operation;
- append-oriented administrator audit events for bootstrap, role and CMS mutations, without credentials or secrets;
- bounded administrator audit visibility behind `audit.view`, authentication and confirmed MFA;
- optional Cloudflare Access administrator-gate deployment documentation as defense in depth only; application auth/MFA/RBAC remain authoritative;
- no arbitrary code/plugin upload, rich HTML authoring or media upload feature.

Phase 6 implementation merged through:

- PR #44 / `170d52393e543c8033ebd896f42fb43f3fccdf42` — deny-by-default Admin/RBAC foundation;
- PR #45 / `be25d6ec3e0512bb9615329f99f16fff294d8b1d` — first-admin bootstrap, audited role lifecycle, privileged news/pages and administrator audit.

Closure revalidation against merged `main` confirmed:

- every current administrator route requires Platform authentication, confirmed MFA and an exact explicit permission;
- unknown permissions fail closed and no wildcard authorization path exists;
- role, CMS and audit regression tests cover denied and authorized paths, including missing MFA/permission, first-admin bootstrap, final-platform-admin protection, CMS publication/XSS behavior and bounded audit visibility;
- privileged role and CMS mutations append administrator audit events inside the Platform transaction boundary where practical;
- Legacy engine/login-server credentials, sessions, schema and game-login behavior are unchanged by Phase 6;
- Cloudflare Access remains an optional production deployment decision and is not claimed as deployed.

Exit gate — satisfied by closure revalidation:

- authorization policies are deny by default;
- privileged operations are covered by explicit authorization and confirmed-MFA tests;
- administrator state-changing actions delivered by Phase 6 are auditable.

## Phase 7 — Production hardening and operations

**Status: COMPLETE**

Delivered engineering/hardening scope:

- production topology evidence model that separates repository/staging proof from actual deployed production facts;
- provider-independent fail-closed production configuration verification;
- required Composer advisory scanning and bounded dependency-update automation;
- CSP and browser security headers with regression coverage;
- server-generated request correlation and structured JSON request-completion logging primitives;
- production-readiness/go-live checklist plus incident and recovery runbooks;
- database-enforced generic legacy engine read-only boundary and fail-closed effective-grant verification for generic, provisioning and character-create principals;
- controlled production-like deployment, migration, rollback, interrupted-release isolation and redeploy validation;
- controlled production-like Redis ACL/key/command validation and missing/malformed/unavailable dependency semantics;
- delivery-capable SMTP validation plus unavailable-mail behavior;
- exact-SHA critical feature/integration regression coverage across Identity, admin/RBAC/CMS, account/binding, character creation and public game-data flows;
- live production-like health, CSP/security-header, Secure/HttpOnly cookie, request-correlation, structured-log and representative sensitive-error checks;
- real production-like MariaDB backup, clean restore, integrity verification and restored-environment smoke with staging-only recovery timing.

Phase 7 closure evidence:

- PR #63 merged as `61f72ddda5c253f26c7d59aa7b6fce3506f120dc`;
- final PR head `7842f78ec4ac2d07d3800ffe8bde9809b055822d` passed Phase 7 Production-Like Validation #9, required CI #759 and Agent Governance #679;
- controlled evidence is classified `STAGING_PROVEN` only;
- final-head controlled restore measured `105 ms` with `13/13` tables, `11/11` migrations and matching validation-SHA probe; this is not production RTO/RPO.

Exit gate — satisfied under ADR 0007:

- required Phase 7 hardening/operations mechanisms are delivered in the repository;
- required repository CI, including dependency advisory audit, formatting, static analysis and full tests, passed on the exact final staging-validation PR head;
- controlled production-like validation passed for the staging-verifiable deployment/rollback, configuration, database privilege, Redis, SMTP, critical-flow, security-smoke and backup/restore boundaries;
- staging evidence remains explicitly separate from final production evidence.

Phase 7 completion is an engineering/hardening milestone. It does **not** claim that the final production environment is verified or approved for go-live. ADR 0007 separates the final production verification into the fail-closed Production Go-Live Gate below.

## Production Go-Live Gate — operational release gate

**Status: PENDING PRODUCTION VERIFICATION**

**Production Readiness: STAGING_PROVEN**

**Production Verification: REQUIRED BEFORE GO-LIVE**

The authoritative gate is `docs/operations/PRODUCTION_READINESS_CHECKLIST.md`.

Go-live cannot pass until mandatory final-production facts are directly verified for the selected launch scope. Staging evidence cannot be promoted to `PRODUCTION_PROVEN`.

The gate includes direct production verification of:

- exact deployed Oteryn Platform SHA and relevant legacy engine/login-server versions;
- production DNS/Cloudflare/WAF/Access/TLS behavior;
- direct-origin exposure and ingress firewall/reverse-proxy restrictions;
- production Platform/legacy engine DB topology, network isolation and effective grants;
- production runtime Redis endpoint/ACL/network/TLS state;
- production session/cache/queue topology and worker behavior where applicable;
- production mail provider/domain/delivery monitoring;
- production logging/metrics/alerts/retention/access/on-call routing;
- actual provider deployment/migration/rollback mechanism and emergency operator authorization;
- production backup policy/schedule and dated production restore evidence;
- final production health/readiness and critical smoke/E2E checks against the exact deployed SHA.

If Platform-originated authoritative game login is required for launch scope, the separately authorized game-login bridge must also be resolved before the gate can pass.

An explicit owner risk decision, where policy permits, does not fabricate `PRODUCTION_PROVEN` evidence and cannot be used to claim an unverified production fact was verified.

## Phase 8 — Payments, coins and shop

**Status: DEFERRED**

Start only after core platform and identity are stable.

Deliverables:

- dedicated payment ADR/threat model;
- provider integration;
- signed webhook verification;
- idempotency/replay controls;
- immutable transaction ledger;
- reconciliation;
- refunds/chargebacks;
- shop fulfillment contract with legacy engine;
- admin and fraud controls.

Exit gate:

- financial consistency tested under retries/concurrency;
- payment/provider security reviewed separately.

## Cross-cutting rule

A phase may be split into small tasks and PRs. Agents should not implement an entire phase as one large change.

Before each task:

1. create an active task record;
2. claim owned paths;
3. load routed context;
4. prove required external facts;
5. implement the smallest complete vertical slice;
6. test and update documentation/contracts.
