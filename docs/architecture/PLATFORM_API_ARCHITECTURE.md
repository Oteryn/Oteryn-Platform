# Oteryn Platform API Architecture

## Status

**CURRENT — accepted focused PlatformAPI architecture under ADR 0036.**

Repository-owner decision: **Option A — explicitly defer the general Platform API until a named consumer exists.**

This document owns the Platform-side activation, adaptation, versioning and compatibility boundary for a future general first-party/public API. It does not create an API endpoint inventory, runtime implementation or production activation authority.

## Decision summary

Oteryn does not expose a general Platform API merely because Laravel has API routing, Passport is configured, specialized game-auth endpoints exist or an audit row expects a disposition.

A general Platform API is intentionally **deferred** until one approved named consumer and use case require it. The future API remains an adapter over existing module application services and source-owned semantics.

Current specialized transports remain outside this general boundary:

- `POST /api/v1/game-auth/tickets` — game-login ticket issuance owned by the GameAuth contract;
- `/internal/v1/game-auth/**` — private Gateway service contracts;
- liveness/readiness/operational probes — OperationsObservability contracts.

None may be reclassified as general PlatformAPI completeness.

## Evidence and status dimensions

Keep these facts separate:

- **architecture disposition:** `DEFERRED` until a named consumer trigger exists;
- **module implementation availability:** no general PlatformAPI implementation is proven on `main`;
- **specialized API-shaped transports:** implemented for their own bounded contracts only;
- **production activation:** not authorized and not implied by accepted architecture.

A route, controller, auth guard or token library proves implementation capability only for its exact scope. It never proves product activation, compatibility commitment or production readiness for a general API.

## Activation triggers

A new PlatformAPI architecture/implementation package may start only when at least one approved concrete trigger exists:

1. a first-party native/desktop/mobile client needs Platform resources not covered by a specialized existing contract;
2. PlayerCompanion requires a non-web machine consumer;
3. Federated Search or another public capability has an approved machine-consumer requirement;
4. an explicit partner/developer API product is promoted into scope.

The trigger must name the consumer and use case. “Useful someday”, “REST would be cleaner”, audit completeness, existing Passport support or the presence of `/api/v1` are not activation triggers.

## Mandatory activation checklist

Before the first stable general endpoint is authorized, one bounded package must define and validate all applicable items below.

### Consumer and resource scope

- named consumer and product use case;
- exact resource/operation inventory;
- source module/application-service owner for every operation;
- public, authenticated-private, privileged or service-to-service classification;
- explicit non-goals and excluded modules/data.

### Contract and versioning

- canonical URL/version namespace;
- response and error envelopes;
- externally observable schema/semantic compatibility rules;
- pagination/filter/sort semantics where applicable;
- deprecation, sunset and withdrawal policy;
- client retry/backoff expectations and retry-safe operations.

Internal PHP class names, Eloquent model shapes and database columns are never compatibility contracts.

### Authentication and authorization

For every non-public operation:

- authentication class and trust model;
- grant/client registration model where applicable;
- explicit scopes/permissions or equivalent server-side authorization;
- token/session creation, expiry, rotation and revocation;
- theft/replay/recovery behavior;
- device/client binding only if explicitly justified;
- CSRF/CORS applicability defined rather than inherited accidentally;
- server-resolved ownership; browser/client account or character identifiers never authorize access by themselves.

Existing Passport configuration may be reused only after these decisions are accepted for the named consumer. It is not blanket authorization for a new API.

### Privacy, enumeration and cacheability

- public/private field allowlist;
- source privacy/publication decisions remain authoritative;
- enumeration policy is explicit for character/account/community resources;
- sensitive/private responses are not shared-cacheable by default;
- owner-private outputs preserve principal/revision fencing equivalent to the source application service;
- query strings, credentials, tokens and private identifiers do not become ordinary telemetry labels or log fields.

### Freshness and failure semantics

Where data can become stale or unavailable, the API preserves source semantics for:

- observation/revision identity;
- freshness/expiry/applicability;
- stale versus unavailable versus empty/not-found;
- partial results when explicitly supported;
- confidence/provenance where the source contract defines it.

Dependency failure must never fabricate `0`, `offline`, empty results, success or completed state.

### Mutations

Every mutation requires, as applicable:

- source-module command/application-service ownership;
- explicit authorization;
- idempotency key/operation identity;
- concurrency and optimistic/locking semantics;
- ambiguous-result reconciliation;
- audit policy;
- rollback/compensation rules where a saga exists.

PlatformAPI never creates a second mutation/business-rule implementation.

### Abuse and operational controls

- per-operation application-level rate/abuse budgets;
- edge limits only as defense in depth;
- bounded request/response sizes;
- timeout and dependency-budget behavior;
- request correlation compatible with OperationsObservability;
- metrics/logging that avoid secrets, raw tokens and high-cardinality private identifiers;
- exact alerts/runbooks only when the delivered API requires them.

### Validation and activation

The implementation package must provide applicable:

- unit/contract tests;
- authorization/privacy negative paths;
- integration tests against real owned application services;
- deterministic rate/idempotency/concurrency tests where relevant;
- client/browser E2E for the named consumer where relevant;
- dependency failure/stale/unavailable evidence;
- exact-head self-review and repository CI;
- exact deployment/environment evidence before any production claim.

## Adaptation boundary

The dependency direction is:

```text
Named API consumer
  -> PlatformAPI transport/serialization adapter
       -> owning module application service/query/command
            -> source-owned domain/integration boundary
```

Forbidden directions include:

```text
PlatformAPI -> raw module tables/models as a shortcut
source module -> PlatformAPI transport types
PlatformAPI -> duplicate formula/policy implementation
PlatformAPI -> raw Canary/Oteryn-v2 persistence
```

PlatformAPI may normalize transport concerns, but source modules retain business authority.

## Known future consumer candidates

### PlayerCompanion

If a non-web consumer is approved later, PlatformAPI may adapt the same PlayerCompanion application services used by browser workflows for bounded calculator execution, owner workspaces, compatible recommendations, tracking/routine preferences or share resolution.

It must preserve ruleset/version/freshness/privacy semantics and must not expose raw user session text or invent a second recommendation/formula path.

### Federated Search

If a machine consumer is approved after federated search exists, PlatformAPI adapts the same `PublicPortal` FederatedSearch application service and normalized result/failure semantics.

It must not independently fan out to CMS, Announcements, Events, Wiki or GameCatalog and must not recreate cross-source grouping/ranking policy.

### Account / character clients

A future first-party authenticated client must consume owner context through accepted `Identity`/`Accounts`/Character Portfolio application boundaries and canonical identifiers. It must not make `canary_account_id`, `canary_player_id` or raw game tables permanent external contracts.

## Security ownership

`SECURITY_ARCHITECTURE.md` remains the mandatory cross-cutting security owner. This focused document adds API-specific activation requirements but does not supersede:

- deny-by-default authorization;
- one authoritative identity policy;
- least privilege;
- untrusted browser/API input validation;
- application-level rate limiting;
- secret/private-data logging prohibitions;
- production edge/origin defense in depth.

Option A adds no runtime API surface and therefore requires no new security mechanism today.

## Operations and production boundary

`OPERATIONS_OBSERVABILITY_ARCHITECTURE.md` owns evidence classification and operational proof. A future API implementation does not become production-proven because tests or an ADR pass.

Production claims require exact deployment identity and the applicable live evidence for ingress, TLS/edge, logs/metrics/alerts, dependency reachability, rollback/recovery and smoke/E2E under separately authorized protected-environment procedures.

## Compatibility policy

No general v1 compatibility promise exists while PlatformAPI is deferred.

When activated, compatibility scope must be defined around externally observable request/response/error semantics, not internal framework structure. Breaking change, deprecation and sunset rules must exist before the surface is described as stable.

## Issue #490 disposition

ADR 0036 makes the **PlatformAPI** finding in shared Issue #490 terminal by explicit owner deferral.

Issue #490 remains open only for its separately owned residual findings, including PublicEdge protected-environment proof and direct production evidence required by the go-live gate. Closing the PlatformAPI slice does not promote those findings to complete.

## Current implementation handoff

None.

A new implementation Issue is created only after an activation trigger names a consumer/use case and the bounded package satisfies this document's activation checklist.

## Explicit non-authority

This document does not authorize:

- new runtime routes/controllers;
- public or authenticated API activation;
- new Passport clients/scopes/grants;
- production/protected-environment operations;
- external repository access or mutation;
- owner-funded Codex/OpenAI/API-token usage.
