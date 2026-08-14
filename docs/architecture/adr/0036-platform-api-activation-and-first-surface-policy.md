# ADR 0036 — Platform API activation and first-surface policy

## Status

Accepted — 2026-08-14

Decision owner: repository owner. Shared audit owner: Issue #490. Decision PR: #1044.

Selected option: **Option A — explicitly defer the general Platform API until a named consumer exists.**

## Context

`PlatformAPI` has an accepted ownership boundary but no general API implementation on `main`. Canonical module architecture says a general API is exposed only for a concrete client/use case and must adapt existing module application services and policies rather than become a second business-rule path.

Current `main` proves only specialized API-shaped transport surfaces:

- `POST /api/v1/game-auth/tickets` is a bounded game-login ticket issuance endpoint protected by the Passport-backed `auth:api` guard and a dedicated throttle;
- `/internal/v1/game-auth/**` is a private Gateway-service contract protected by a service credential and dedicated throttles;
- these endpoints are explicitly not a general public/first-party Platform API.

Issue #490 therefore cannot be closed by relabelling those routes. The product question was whether Oteryn should activate a general Platform API now, and if so which consumer class should establish the first durable compatibility/security surface.

Two future consumer candidates already exist in accepted architecture, but neither currently requires a general API contract on `main`:

- PlayerCompanion may later need API reuse for calculator execution, owner workspaces, recommendations, tracking/routine preferences and share resolution; current PR #1028 is a browser/private vertical slice, not a general API consumer;
- Federated Search may later expose the same `PublicPortal` search application service through an API once that capability is implemented, without independently fanning out to source modules.

Creating a broad API before a named consumer exists would add a versioning, authentication, privacy, rate-limit and compatibility promise without a demonstrated product requirement.

## Decision

The repository owner selected **Option A** on 2026-08-14.

Oteryn does **not** activate a general Platform API merely to satisfy an audit row or because Passport/API-shaped specialized routes already exist. `PlatformAPI` remains a deliberately deferred product surface until an approved named consumer and use case require it.

This is an explicit product disposition, not an absence of architecture. It closes the PlatformAPI ambiguity by defining both the non-activation decision and a fail-closed future activation gate.

## Non-negotiable invariants

1. Existing game-auth and internal Gateway endpoints remain specialized contracts and do not count as general PlatformAPI completeness.
2. PlatformAPI is an adapter over source-module application services/queries. It does not query module persistence directly, bypass policies, recreate business logic or become source-of-truth ownership.
3. Public/private eligibility, publication/privacy decisions, freshness/applicability, authorization and mutation authority remain with their source owners.
4. API errors never fabricate source state such as `0`, `offline`, empty results, success or completed when a dependency is unavailable/stale.
5. Versioning protects externally observable schemas and semantics. Internal PHP class stability is not an API compatibility promise.
6. Authentication class, authorization/scopes, token lifecycle, CSRF/CORS applicability, rate limits/abuse controls, cacheability and privacy are explicit per surface; existing Passport configuration is implementation capability, not blanket authorization for new APIs.
7. Sensitive/private resources default to non-public and non-shared-cache. Browser-supplied account/character identifiers never establish ownership.
8. No API endpoint obtains production activation authority merely because code or a contract is merged. Exact production/security/operations gates remain separate.
9. API telemetry follows OperationsObservability privacy/secrecy rules and does not use high-cardinality private identifiers or credentials as metric/log labels.
10. Breaking change, deprecation and withdrawal semantics are explicit before a surface is declared stable.

## Selected posture — Option A

Do not add a general route namespace merely to manufacture completeness. Existing specialized game-auth/internal contracts remain valid but remain outside PlatformAPI classification.

A future activation package must identify one **named consumer and use case** and supply, before the first stable endpoint:

- exact resource/operation inventory and source-module owners;
- public versus authenticated/private classification;
- canonical URL/version namespace and response/error envelope;
- authentication/authorization/scopes and token/session lifecycle where applicable;
- per-operation rate/abuse budgets and client retry/backoff expectations;
- privacy, cacheability, locale, pagination and enumeration rules;
- source freshness/applicability/confidence semantics where applicable;
- idempotency/concurrency semantics for any mutation;
- compatibility, deprecation and sunset rules;
- security, focused/integration/E2E evidence and exact production activation gates.

### Benefits

- follows the existing “concrete consumer first” architecture;
- minimizes public attack surface and long-lived compatibility commitments;
- avoids designing an abstraction around hypothetical consumers;
- allows the first API to inherit the exact privacy/freshness semantics of the product that actually needs it.

### Costs

- no general-purpose first-party/public API exists immediately;
- future client integration requires a separate activation package before use;
- Issue #490's PlatformAPI finding is terminal by explicit owner disposition rather than implementation.

## Alternatives considered

### Option B — Activate a public read-only v1 first

Create a stable anonymous/read-only API for already-public Platform information, beginning only with explicitly approved public read models/application queries. Candidate capabilities are public game-data reads and, once implemented, the same federated-search application service used by PublicPortal.

This was not selected because it would create a durable public compatibility, scraping/enumeration and abuse-control surface before a named consumer has demonstrated need.

### Option C — Activate an authenticated first-party account/client v1 first

Create a stable authenticated API intended only for an identified Oteryn first-party client, using source-module application services and a purpose-designed client/scopes/token lifecycle rather than inheriting game-auth ticket semantics.

This was not selected because it creates a materially larger security/privacy and compatibility surface before an approved client requires it.

### Option D — Activate a broad mixed public + authenticated API now

Rejected. This would maximize attack surface and compatibility commitments, encourage duplicated orchestration/direct persistence access and conflict with the concrete-consumer-first architecture.

## Activation triggers

Revisit this decision only when at least one concrete trigger exists:

- an approved first-party native/desktop/mobile client needs Platform resources not covered by a specialized existing contract;
- PlayerCompanion requires a non-web consumer API;
- Federated Search or another public capability has an approved machine-consumer requirement;
- an explicit partner/developer API product is promoted into scope.

A trigger starts a new bounded decision/implementation package; it does not itself authorize endpoints.

## Consequences

- `PlatformAPI` remains non-implemented and is product/programme `DEFERRED` until a named consumer activation trigger exists;
- module-catalog implementation status may remain `PLANNED` because that status describes repository implementation availability rather than launch disposition;
- Issue #490's PlatformAPI audit finding becomes terminal by owner disposition, while PublicEdge/direct-production evidence remains separately open;
- no general `/api/v1` resource family is added by this architecture closeout;
- specialized game-auth/internal APIs retain their existing owners/contracts;
- `PLATFORM_API_ARCHITECTURE.md` becomes the focused canonical owner for the activation gate and shared invariants, not a fictional endpoint inventory;
- no speculative implementation handoff is created;
- future API implementation requires a named consumer and a bounded package satisfying the activation checklist.

## Existing canonical documents

No content change is required in `MODULE_CATALOG.md` or `PORTAL_COMPLETENESS_ARCHITECTURE.md` for this decision:

- `MODULE_CATALOG.md` already states that endpoints exist only for a concrete client/use case, that module services/policies must be reused and that specialized game-auth/internal endpoints are not general PlatformAPI; its `PLANNED` status is an implementation-availability label rather than launch disposition;
- `PORTAL_COMPLETENESS_ARCHITECTURE.md` already says a versioned first-party API becomes justified by concrete consumers and must reuse module services/authorization/version/freshness semantics.

`SECURITY_ARCHITECTURE.md` also already owns the applicable generic API security invariants: API input is untrusted, expensive/public search APIs require application-level limits, private identifiers do not authorize ownership, and production/edge controls remain defense in depth. Option A adds no runtime API surface and therefore requires no new security mechanism.

## Rejected shortcuts

- counting `POST /api/v1/game-auth/tickets` as a complete general Platform API;
- exposing controllers/models directly to avoid application-service contracts;
- declaring Passport availability equivalent to an accepted token/scopes product design;
- making all current web routes available through JSON automatically;
- using one global rate limit, cache policy or authorization rule for unrelated modules;
- promising indefinite v1 compatibility without an explicit deprecation/sunset contract.

## Future implementation handoff

There is **no implementation handoff now**.

When an activation trigger exists, the new package must:

1. name the exact consumer/use case;
2. define the first resource/operation inventory and source owners;
3. satisfy the activation checklist in `PLATFORM_API_ARCHITECTURE.md`;
4. create a bounded implementation Issue only after architecture/security scope is accepted;
5. validate focused, integration/E2E and negative paths on exact final head;
6. keep production activation behind independent security/operations/protected-environment gates.

No external repository, protected environment, production operation or owner-funded AI service is authorized by this ADR.
