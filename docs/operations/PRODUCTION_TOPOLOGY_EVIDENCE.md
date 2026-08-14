# Oteryn Platform Production Topology Evidence Baseline

## Status

Current repository/staging evidence inventory — reconciled 2026-08-14.

Historical Phase 7 discovery began on 2026-07-20. This revision reconciles that baseline with later merged source and controlled Production Readiness evidence. It preserves the distinction between repository capability, staging/production-like execution evidence and direct production proof.

The governing evidence semantics are defined by `docs/architecture/OPERATIONS_OBSERVABILITY_ARCHITECTURE.md`.

This document must not contain production secrets, credentials, private keys, copied `.env` files, database dumps, private IP inventories or personal data.

## Evidence states

- `REPOSITORY_PROVEN` — exact current source/configuration/tests prove a capability or invariant.
- `STAGING_PROVEN` — a dated controlled staging/production-like exercise proves the exact tested environment/run only.
- `ENVIRONMENT_EVIDENCE_REQUIRED` — the fact is required for a production claim but cannot be proved from repository/staging state.
- `PRODUCTION_PROVEN` — authorized direct evidence identifies the exact production environment and release and proves the bounded fact.
- `UNKNOWN` — available evidence is absent, stale, ambiguous or insufficient.

No evidence state promotes itself automatically to a stronger state.

## Repository-proven and staging-proven facts

### Application shape and liveness

`REPOSITORY_PROVEN`

- Oteryn Platform is one Laravel modular-monolith deployable.
- `SYSTEM_ARCHITECTURE.md` defines a logical target shape with edge/origin/reverse-proxy and Laravel application boundaries; this is architecture intent, not deployed topology proof.
- `bootstrap/app.php` configures Laravel's framework health route at `/health`.
- `/health` is treated as application liveness, not evidence that all external dependencies are ready.
- No separate general dependency-aware readiness endpoint is repository-proven by the current architecture/source review.

`ENVIRONMENT_EVIDENCE_REQUIRED`

- the exact production load-balancer/orchestrator probe contract;
- whether production uses `/health` for liveness, readiness or neither;
- any production dependency-readiness aggregation and freshness policy.

### Edge and origin

`REPOSITORY_PROVEN`

- PublicEdge contracts/tooling define repository-owned DNS/TLS/redirect/HSTS/WAF/tunnel/origin/private-ingress expectations.
- Cloudflare/edge controls never replace Laravel authentication, authorization, MFA or application rate limiting.

`ENVIRONMENT_EVIDENCE_REQUIRED`

- actual production DNS hostname and proxy state;
- effective TLS mode and termination point;
- effective WAF/rate-limit/Access policy identities;
- actual origin provider/reverse proxy;
- direct-origin bypass status;
- ingress/firewall allowlists.

These are separately owned PublicEdge live-proof facts under Issue #490 and require protected-environment authority.

### Web runtime and deployment mechanism

`REPOSITORY_PROVEN`

- CI validates locked Composer dependency metadata, advisory state, formatting/static analysis and repository tests according to the current workflow contract.
- The repository architecture remains provider-neutral.
- Production configuration verification and operational runbooks exist.

`STAGING_PROVEN`

Controlled Phase 7 Production Readiness evidence records an exact production-like/staging exercise for:

- deployment/configuration verification;
- ordered migrations;
- rollback/redeploy behavior;
- application/database validation after recovery.

`ENVIRONMENT_EVIDENCE_REQUIRED`

- actual production hosting/orchestration provider;
- PHP/web process model and reverse proxy;
- production instance count;
- artifact/image strategy;
- effective production release/migration command sequence;
- zero-downtime behavior;
- effective production rollback mechanism and operator.

### Platform database

`REPOSITORY_PROVEN`

- Application configuration supports Platform-owned SQLite and MySQL connections.
- `.env.example` uses SQLite as a local-safe default and does not claim production engine choice.
- Platform migrations and repository-level persistence tests exist.

`STAGING_PROVEN`

Controlled Production Readiness evidence records a clean Platform database restore/integrity exercise against the staging/production-like profile, including a measured restore and schema/integrity verification for that run.

`ENVIRONMENT_EVIDENCE_REQUIRED`

- actual production database engine/topology/endpoint;
- network isolation and TLS-in-transit state;
- credential injection/rotation mechanism;
- HA/replication state;
- production backup technology, schedule, retention and owner;
- dated production restore exercise and recovery measurements.

### Canary SQL boundaries

`REPOSITORY_PROVEN`

The repository defines operation-specific SQL surfaces including:

- `canary` — generic read-only compatibility access;
- `canary_provisioning` — account provisioning boundary;
- `canary_character_create` — character-create boundary;
- additional operation-specific principals where later accepted contracts require them.

Least-privilege grant templates/verifiers exist for approved write boundaries.

`STAGING_PROVEN`

Controlled production-like verification has exercised least-privilege dependency controls for the scoped validation profile.

`ENVIRONMENT_EVIDENCE_REQUIRED`

- actual production endpoints/network paths;
- effective production credentials/grants;
- production verifier results for each active credential class;
- credential rotation/secret-management mechanism.

### Canary runtime Redis

`REPOSITORY_PROVEN`

- A dedicated `canary_runtime` Redis configuration surface exists.
- Its intended boundary is read-only runtime data with a dedicated ACL/user, separate from Platform cache/session credentials.

`STAGING_PROVEN`

Controlled production-like validation has exercised the scoped Redis ACL/dependency controls used by the readiness profile.

`ENVIRONMENT_EVIDENCE_REQUIRED`

- production endpoint/network/TLS state;
- effective production ACL/user provisioning;
- current monitoring and failure-alert ownership.

### Platform sessions and cache

`REPOSITORY_PROVEN`

- Session storage and cache are environment-configurable within the implemented configuration surfaces.
- Secure cookie, HttpOnly and SameSite behavior is defined by current application configuration.
- Repository defaults are safe local/development defaults and are not production topology evidence.

`ENVIRONMENT_EVIDENCE_REQUIRED`

- production session backend;
- production cache backend;
- scaling/instance model and whether shared state is required;
- effective cookie domain/proxy/TLS behavior;
- cache/session failure and recovery behavior for the deployed topology.

### Queue

`REPOSITORY_PROVEN`

- Queue behavior is defined by the current repository configuration and may remain synchronous where no async capability is activated.
- Queue presence in architecture does not imply a production worker fleet exists.

`ENVIRONMENT_EVIDENCE_REQUIRED` when asynchronous work is enabled or required

- effective queue backend;
- worker process supervision;
- retry/dead-letter/failed-job policy;
- monitoring and alert ownership.

### Mail

`REPOSITORY_PROVEN`

- Mail configuration supports non-production and SMTP-style delivery surfaces.
- `.env.example` intentionally avoids production credentials.
- production configuration verification rejects invalid/non-delivery production mail defaults where the current verifier requires a real transport.

`STAGING_PROVEN`

Controlled Production Readiness evidence records a scoped SMTP/dependency exercise for the staging/production-like profile.

`ENVIRONMENT_EVIDENCE_REQUIRED`

- actual production provider/transport;
- sender domain/address readiness;
- SPF/DKIM/DMARC state where applicable;
- credential injection/rotation;
- bounce/delivery monitoring and alert ownership.

### Request correlation, logging and monitoring

`REPOSITORY_PROVEN`

- `RequestCorrelation` generates a fresh server-side UUID for every Laravel-handled request.
- The identifier is attached to the request as `request_id` and emitted on normal responses as `X-Request-ID`.
- `http.request.completed` records only request ID, HTTP method, normalized route name, response status and bounded duration.
- The completion event does not include full URL, query string, request body, request headers or credential values.
- Application logging supports repository-configured channels including stderr/structured-capable output surfaces.
- Security/admin audit primitives exist under their own source-domain contracts.

`STAGING_PROVEN`

Controlled Production Readiness evidence records a request-correlation/logging smoke and secret-free operational verification for the tested profile.

`ENVIRONMENT_EVIDENCE_REQUIRED`

- actual production log channel/serializer;
- centralized log sink and ingestion health;
- retention and access-control policy;
- reverse-proxy/downstream request-ID propagation;
- metrics backend and active metric set;
- alert rules, destination and on-call/escalation identity;
- dated alert-delivery evidence.

An optional JSON/stderr capability is not evidence that centralized production logging, metrics or alerts exist.

### Backups and restore

`REPOSITORY_PROVEN`

- Production architecture and runbooks require backup/restore evidence before a production-ready claim.
- Restore validation is a named go-live evidence category.

`STAGING_PROVEN`

- Controlled Production Readiness evidence records a clean Platform database restore/integrity exercise for the staging/production-like environment.

`ENVIRONMENT_EVIDENCE_REQUIRED`

- production backup technology and scope;
- owner, schedule and retention;
- encryption/access and independent/off-site-copy disposition where required;
- latest successful production backup identity;
- dated production restore result and recovery measurements;
- production restore/rollback owner and escalation path.

## Current production evidence matrix

| Boundary | Current state | Reason |
|---|---|---|
| Logical target topology | `REPOSITORY_PROVEN` | architecture documents define the intended shape only |
| Laravel `/health` liveness capability | `REPOSITORY_PROVEN` | exact source configures the route |
| General dependency-aware readiness | `UNKNOWN` | no current general repository contract was proven |
| Request-correlation/log shape | `REPOSITORY_PROVEN` | exact middleware source |
| Request-correlation/logging controlled smoke | `STAGING_PROVEN` | Production Readiness evidence |
| Deploy/migrate/rollback controlled exercise | `STAGING_PROVEN` | Production Readiness evidence |
| Platform DB clean restore controlled exercise | `STAGING_PROVEN` | Production Readiness evidence |
| Active production release identity | `ENVIRONMENT_EVIDENCE_REQUIRED` | direct production observation required |
| Production web/orchestration topology | `ENVIRONMENT_EVIDENCE_REQUIRED` | provider/runtime/instance facts require direct evidence |
| Production edge/origin state | `ENVIRONMENT_EVIDENCE_REQUIRED` | separately authorized PublicEdge live proof |
| Production centralized logging | `ENVIRONMENT_EVIDENCE_REQUIRED` | no direct production evidence |
| Production metrics | `ENVIRONMENT_EVIDENCE_REQUIRED` | no direct production evidence |
| Production alerts/on-call | `ENVIRONMENT_EVIDENCE_REQUIRED` | no direct production evidence |
| Production retention/access policy | `ENVIRONMENT_EVIDENCE_REQUIRED` | no direct production evidence |
| Production Platform DB backup/restore | `ENVIRONMENT_EVIDENCE_REQUIRED` | staging restore cannot prove production backups |
| Production dependency network/ACL state | `ENVIRONMENT_EVIDENCE_REQUIRED` | direct deployed evidence required |
| Production mail/queue effective operation | `ENVIRONMENT_EVIDENCE_REQUIRED` when required | depends on exact selected deployment |

## Minimum acceptable production evidence

Evidence must be non-secret, dated where practical, and bound to the exact environment/release.

| Boundary | Minimum acceptable evidence |
|---|---|
| Release identity | exact active production release/commit identity and observation timestamp |
| Edge/DNS/TLS | sanitized provider/config summary proving hostname, proxy/TLS mode and relevant control identifiers |
| Origin ingress | sanitized firewall/security-group/reverse-proxy evidence proving intended ingress and direct-origin disposition |
| Web runtime | deployment manifest/runbook/effective platform summary proving runtime, process model, instance strategy, release and rollback procedure |
| Platform DB | sanitized effective engine/network/backup topology summary; no passwords or connection strings |
| Canary SQL | sanitized endpoint/network boundary plus successful least-privilege verifier evidence for each active production credential class |
| Runtime Redis | sanitized endpoint/network/ACL summary plus effective dedicated read-only ACL evidence |
| Sessions/cache | sanitized effective runtime configuration and scaling model |
| Queue | effective configuration plus worker/supervision/retry evidence if asynchronous workers are enabled |
| Mail | effective provider/transport, sender readiness and monitoring summary without credentials |
| Logs | effective production channel/format/sink/retention/access summary plus redacted structured request event showing request ID without query/body/credential data |
| Metrics/alerts | backend/rule/on-call ownership summary plus dated delivery or observation evidence |
| Backups | production backup policy plus dated restore-test record with scope, integrity result and recovery measurements |

A copied production `.env` file is never acceptable evidence.

## Production-readiness relationship

`docs/operations/PRODUCTION_READINESS_CHECKLIST.md` remains the current Phase 7 evidence ledger:

- Engineering Complete: completed;
- Production Readiness: `STAGING_PROVEN`;
- Production Go-Live Gate: pending direct production verification.

Repository/staging progress may continue independently, but no documentation change here can satisfy a production-only gate without the direct evidence envelope required by `OPERATIONS_OBSERVABILITY_ARCHITECTURE.md` and the applicable PublicEdge/security contracts.

## Issue #490 applicability

This baseline is the operational evidence source for the OperationsObservability slice of Issue #490.

After PR #1042 merges, the repository applicability/profile contract and current repository/staging evidence inventory for OperationsObservability are considered reconciled. Issue #490 remains open for:

- Platform API disposition;
- PublicEdge protected-environment proof;
- direct production evidence for topology, observability, backup/restore and recovery controls required by the go-live gate.

## Current blocker boundary

The repository alone cannot prove the actual deployed production topology or protected-environment control state.

Therefore it must not claim any of the following without direct current environment evidence:

- edge/WAF/Access controls are active;
- origin bypass is blocked;
- databases/Redis are privately isolated;
- HSTS/TLS policy is safely deployed for the actual hostnames;
- centralized logs/metrics/alerts/on-call are operational;
- production backups are current and restorable;
- production mail delivery is operational;
- the production release/migration/rollback procedure is proven;
- the exact active production release is the version being evaluated.
