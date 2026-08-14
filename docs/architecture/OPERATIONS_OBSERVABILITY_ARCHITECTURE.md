# Oteryn Operations and Observability Architecture

## Status

**CURRENT on merge of PR #1042 — repository/architecture evidence boundary for `OperationsObservability`.**

This focused document reconciles existing accepted Platform architecture, security invariants and Phase 7 operational evidence. It does not introduce a new production provider choice, runtime implementation or production activation decision, so no new ADR is required for this reconciliation.

Issue #490 remains the shared audit owner for distinct non-UI concerns. The OperationsObservability applicability/evidence contract is terminal through PR #1042, and ADR 0036 now makes the general PlatformAPI disposition terminal by explicit owner deferral. The remaining Issue #490 concerns are PublicEdge live protected-environment proof and direct production evidence required by the go-live gate. This document does not close Issue #490 or convert missing protected-environment evidence into proof.

## Purpose

`OperationsObservability` owns the Platform-side operational evidence model and the cross-cutting expectations needed to operate a release safely. It provides one place to answer:

- what can be proved from repository state;
- what has been exercised in a controlled staging/production-like environment;
- what still requires direct evidence from the exact deployed environment;
- which component owns each operational signal or recovery action;
- what must fail closed when freshness, dependency or production evidence is unavailable.

The architecture deliberately separates **logical target topology**, **implemented capability**, **environment execution evidence** and **production activation authority**.

## Non-authority

OperationsObservability does **not** own:

- product/business rules or gameplay policy;
- Identity, Support, Wallet, Marketplace or other source-module authorization decisions;
- game-server/runtime truth that belongs to the game domain;
- PublicEdge DNS/TLS/WAF/origin policy or protected-environment mutation authority;
- a general Platform API contract;
- private signing keys or software-update trust;
- production credentials, secret management values or raw private infrastructure inventories;
- an assertion that production is healthy merely because repository tests or staging exercises pass.

## Evidence-state model

Operational facts use the narrowest state that direct evidence supports.

| State | Meaning | May prove production? |
|---|---|---|
| `REPOSITORY_PROVEN` | Current exact source/configuration/tests prove a capability or invariant exists in the repository. | No |
| `STAGING_PROVEN` | A dated controlled staging/production-like exercise proves behavior for that exact environment/run. | No |
| `ENVIRONMENT_EVIDENCE_REQUIRED` | The architecture requires a production fact, but repository/staging evidence cannot establish it. | No |
| `PRODUCTION_PROVEN` | Authorized direct evidence identifies the exact production environment, release and observation and proves the stated fact. | Yes, only for the exact stated scope |
| `UNKNOWN` | Available evidence is absent, ambiguous, stale or insufficient even to classify the effective state. | No |

Rules:

1. Evidence never promotes itself upward. `REPOSITORY_PROVEN` does not imply `STAGING_PROVEN`; `STAGING_PROVEN` does not imply `PRODUCTION_PROVEN`.
2. A document saying that a control *should* exist is architecture intent, not evidence that it exists.
3. A successful application test does not prove edge, network, backup, monitoring or external-service deployment state.
4. `PRODUCTION_PROVEN` requires an exact environment identity and release/deployment identity. “Current production” without that envelope is not durable evidence.
5. If evidence expires or no longer identifies the active deployment, the affected fact returns to `ENVIRONMENT_EVIDENCE_REQUIRED` or `UNKNOWN`; stale evidence must not be silently reused as current truth.

## Topology truth layers

Operational topology is represented in three separate layers.

### 1. Logical target topology

`docs/architecture/SYSTEM_ARCHITECTURE.md` defines intended components, trust boundaries and dependency direction. It may describe an edge, origin/reverse proxy, Laravel web tier, Platform database and bounded external dependencies.

This layer answers **what the architecture expects**, not what production currently runs.

### 2. Repository/deployable capability

Exact current source, configuration, migrations, tests, runbooks and validators prove which operational capabilities are implemented or supported by the repository.

Examples include the Laravel health route, request-correlation middleware, logging configuration surfaces, configuration verification, rollback/restore procedures and least-privilege dependency adapters.

This layer answers **what this release can do**.

### 3. Exact environment deployment

Direct environment evidence proves the actual deployed topology, effective configuration, monitoring destinations, backup system and recovery state for a named environment/release.

This layer answers **what is actually running**. Production claims are forbidden when this layer is absent.

## Evidence envelope for production claims

Any future `PRODUCTION_PROVEN` claim under this concern must record enough non-secret metadata to make the evidence reproducible and bounded:

```text
environment_identity
release_or_commit_identity
observed_at
observer_or_authorized_operator
control_or_boundary_checked
evidence_source_or_command
result
relevant_dependency_or_provider_identity_without_secrets
rollback_or_recovery_identity_when_applicable
```

Screenshots or provider exports may supplement evidence, but raw credentials, copied `.env` files, private keys, connection strings, database dumps, private IP inventories and personal data are forbidden evidence artifacts.

## Ownership matrix

| Concern | Primary authority | OperationsObservability responsibility | Required production evidence |
|---|---|---|---|
| Release identity | release/deployment process | expose/record exact deployed release identity and correlate evidence to it | exact active release/commit for named environment |
| Application liveness | Laravel/runtime | define and observe the liveness signal | direct probe against exact deployment |
| Dependency readiness | each dependency adapter/runtime contract | aggregate/report only explicitly implemented readiness facts | direct current checks for every claimed critical dependency |
| Request correlation | application/security middleware | preserve correlation semantics and operational visibility | redacted event/proxy evidence showing effective propagation |
| Application logs | application/security + deployment logging config | require safe structured/parseable events and sink visibility | effective channel/format, central sink if required, retention/access |
| Metrics | source module/runtime | require named operational metrics only where implemented | effective backend and samples for active release |
| Alerts/on-call | operations | define ownership/escalation and verify delivery | active rules, destination/rotation identity and dated delivery proof |
| Platform database | persistence/runtime owner | backup/restore/availability evidence and recovery runbooks | effective engine/topology, backup policy, dated restore proof |
| Canary SQL/Redis | Integration/game compatibility boundary | observe connectivity/least-privilege/failure without taking source authority | effective endpoint/network/ACL/grant evidence without secrets |
| Sessions/cache | runtime/configuration | prove effective backend and scaling assumptions | current deployed config and failure/recovery behavior |
| Queue/mail | Notifications/runtime configuration | prove effective transport/worker requirements and failure handling | actual provider/worker config and monitoring when enabled |
| Deployment/migrations/rollback | release/operations | prove ordered deploy, migration and rollback/redeploy procedure | exact production deployment mechanism and a bounded validated procedure |
| Incident recovery | operations + affected source owner | preserve severity, ownership, evidence and recovery runbook | current escalation path plus exercised recovery evidence |
| PublicEdge | PublicEdge | consume edge evidence only after PublicEdge authority produces it | separately authorized DNS/TLS/WAF/origin proof |
| Business/gameplay truth | source module/game domain | observe only; never redefine the truth | source-owner evidence, not an observability inference |

## Liveness versus readiness

The current repository configures Laravel's framework health route at `/health` in `bootstrap/app.php`. That is `REPOSITORY_PROVEN` application liveness capability.

A successful `/health` response **does not** prove that Platform SQL, Canary SQL, runtime Redis, mail, queue workers, external game services or PublicEdge are ready. The current repository does not prove a separate dependency-aware readiness endpoint as a general production contract.

Therefore:

- `/health` may be used as a liveness signal for the Laravel process/application bootstrap;
- dependency readiness must be represented only by explicitly implemented, source-owned checks;
- absence, timeout or staleness of a dependency check must not be translated into `healthy`, `0`, `offline`, `empty` or any other fabricated business/runtime value;
- introducing a dependency-aware readiness endpoint or aggregator is a separate bounded runtime implementation task with an exact dependency inventory, timeout/freshness semantics, privacy rules and tests;
- no load balancer/orchestrator readiness behavior is claimed until the exact deployment proves how it uses the signal.

## Request correlation and safe logging

Current source proves the following application primitive:

- `RequestCorrelation` generates a fresh server-side UUID for each Laravel-handled request;
- the request attribute is exposed to application code as `request_id`;
- normal responses receive `X-Request-ID`;
- request completion emits `http.request.completed` with request ID, method, route name, response status and bounded duration;
- the bounded completion event does not log full URLs, query strings, request bodies, headers or credential values.

This is `REPOSITORY_PROVEN`. Existing controlled Phase 7 evidence also records a request-correlation/logging smoke as `STAGING_PROVEN`.

Neither fact proves which production sink, serializer, retention policy, access controls, reverse-proxy correlation behavior, metrics system or alert destination is active. Those remain `ENVIRONMENT_EVIDENCE_REQUIRED` until direct evidence exists.

## Release identity

Every operational check that can influence a release/go-live claim must bind to the exact release or commit being evaluated. A green historical check for another release may be useful regression evidence but is not current deployment evidence.

Production verification must therefore establish the active release identity before using health, smoke, logging, restore, edge or dependency observations as evidence for that release.

## Dependency evidence

Critical dependencies are evaluated independently. A single application health signal is never a substitute for all dependency evidence.

For each dependency claimed production-ready, evidence must identify:

- dependency class and owner;
- exact effective connection/configuration identity without secret values;
- network/access boundary when relevant;
- least-privilege or ACL verification when the contract requires it;
- success/failure semantics;
- observation freshness;
- operational owner for alerts/recovery.

The Platform may report that evidence is unavailable. It must not manufacture a source-state value to hide missing evidence.

## Backup and restore

Backup availability and restore correctness are separate facts.

Current Phase 7 controlled evidence proves a clean Platform database restore/integrity exercise in the staging/production-like validation profile. That evidence is `STAGING_PROVEN` only.

A production backup/restore claim requires direct evidence for the deployed production data boundary, including at minimum:

- backup scope and owner;
- schedule/trigger and retention;
- encryption/access boundary where applicable;
- independent/off-site copy disposition where required by the selected topology;
- latest successful backup identity;
- dated restore exercise against a safe target;
- integrity result and measured recovery information;
- documented rollback/escalation path.

Until those facts are observed for production, production backup and restore remain `ENVIRONMENT_EVIDENCE_REQUIRED`.

## Deployment, migrations and rollback

Controlled Phase 7 evidence has exercised ordered deployment/configuration verification, migrations, rollback/redeploy and application/database validation in a production-like/staging context. This is `STAGING_PROVEN`.

It does not prove the production hosting provider, process model, release artifact strategy, migration operator, deployment command sequence, zero-downtime characteristics or rollback mechanism. Those are deployment-specific `ENVIRONMENT_EVIDENCE_REQUIRED` facts.

A future production verification must bind deployment and rollback evidence to the same environment/release identity used by the go-live gate.

## Metrics, alerts and on-call

Repository architecture requires operational visibility, but it intentionally does not invent a provider or claim a currently deployed backend.

Production readiness requires direct evidence for the metrics/alerting controls that the selected production topology actually uses, including:

- which operational metrics are collected and why;
- effective backend/destination;
- alert thresholds or conditions for critical controls;
- alert ownership and on-call/escalation destination;
- a dated test or observed delivery path;
- retention/access rules for sensitive operational data.

A log line or optional JSON stderr configuration does not prove metrics or alerting.

## Failure and degraded-state semantics

The operational layer follows fail-closed evidence semantics:

- missing production evidence means **not production-proven**, not healthy;
- stale environment evidence cannot be silently relabelled current;
- missing dependency evidence cannot be converted to a normal business value;
- inability to prove alert delivery, backup recovery or deployed release identity blocks the corresponding production-readiness assertion;
- observability failure does not authorize business-state mutation or gameplay admission;
- rollback evidence never permits rewriting immutable release history or bypassing separate source-domain safety contracts.

## Current evidence matrix

| Boundary | Current evidence state | Evidence / limitation |
|---|---|---|
| Logical Platform topology | `REPOSITORY_PROVEN` architecture intent | `SYSTEM_ARCHITECTURE.md`; not deployed topology proof |
| Laravel `/health` route | `REPOSITORY_PROVEN` | `bootstrap/app.php`; liveness only |
| General dependency-aware readiness endpoint | `UNKNOWN` / not repository-proven | no accepted current general readiness contract was found |
| Server-generated request correlation | `REPOSITORY_PROVEN` | `RequestCorrelation` middleware |
| Bounded request completion log shape | `REPOSITORY_PROVEN` | `http.request.completed` event fields in middleware |
| Request-correlation/logging controlled smoke | `STAGING_PROVEN` | Phase 7 Production Readiness evidence |
| Deployment/migration/rollback controlled exercise | `STAGING_PROVEN` | Phase 7 Production Readiness evidence |
| Platform DB clean restore/integrity controlled exercise | `STAGING_PROVEN` | Phase 7 Production Readiness evidence |
| Production deployed release identity | `ENVIRONMENT_EVIDENCE_REQUIRED` | must be observed on exact production environment |
| Production web/process/orchestration topology | `ENVIRONMENT_EVIDENCE_REQUIRED` | provider/runtime/instance/deploy facts require direct evidence |
| Production centralized log sink/format | `ENVIRONMENT_EVIDENCE_REQUIRED` | repository log channels are capability, not effective deployment proof |
| Production metrics backend | `ENVIRONMENT_EVIDENCE_REQUIRED` | no direct production evidence |
| Production alerts/on-call | `ENVIRONMENT_EVIDENCE_REQUIRED` | no direct production evidence |
| Production retention/access policy | `ENVIRONMENT_EVIDENCE_REQUIRED` | no direct production evidence |
| Production Platform DB backup/restore | `ENVIRONMENT_EVIDENCE_REQUIRED` | staging restore does not prove production backups |
| Production dependency network/ACL state | `ENVIRONMENT_EVIDENCE_REQUIRED` | requires exact deployed credentials/network evidence without secrets |
| Production mail/queue effective operation | `ENVIRONMENT_EVIDENCE_REQUIRED` where enabled/required | depends on selected deployment |
| PublicEdge DNS/TLS/WAF/origin proof | separate `PublicEdge` evidence gate | Issue #490 residual protected-environment concern |

## Issue #490 disposition

The **OperationsObservability applicability/profile contract and repository/staging evidence reconciliation** are terminal for Issue #490 through PR #1042.

ADR 0036 also makes the **PlatformAPI** product-surface disposition terminal by explicit owner deferral until an approved named consumer/use case exists.

Issue #490 intentionally remains open only for residual concerns that neither terminal architecture disposition can and must not claim to solve:

- PublicEdge live protected-environment proof;
- direct production environment evidence required by the Production Go-Live Gate, including effective deployed observability, topology, backups and recovery controls.

A future protected-environment verification may update operational evidence records from `ENVIRONMENT_EVIDENCE_REQUIRED` to `PRODUCTION_PROVEN`, but it does not need to redefine this architecture unless the ownership model itself changes.

## Validation and change policy

Changes to this document must preserve:

- repository/staging/production evidence separation;
- exact environment/release identity for production claims;
- liveness/readiness distinction;
- source-domain authority and PublicEdge separation;
- secret-free evidence artifacts;
- fail-closed handling of missing or stale evidence.

If a future change transfers durable authority, changes production trust boundaries or selects a material architecture/security option not already implied by accepted sources, it requires the normal ADR/owner-decision process rather than silent editing here.

## Related authority and evidence

- `docs/architecture/ARCHITECTURE_AUTHORITY.md`
- `docs/architecture/SYSTEM_ARCHITECTURE.md`
- `docs/architecture/MODULE_CATALOG.md`
- `docs/architecture/SECURITY_ARCHITECTURE.md`
- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`
- `docs/operations/PRODUCTION_TOPOLOGY_EVIDENCE.md`
- `docs/operations/PRODUCTION_READINESS_CHECKLIST.md`
- `docs/operations/PRODUCTION_VERIFICATION_EVIDENCE.md`
- `docs/operations/INCIDENT_RECOVERY_RUNBOOK.md`
- Issue #490
