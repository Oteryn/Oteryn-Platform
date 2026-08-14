# ADR 0036 — Platform API activation and first-surface policy

## Status

Proposed — 2026-08-14

Decision owner: repository owner. Shared audit owner: Issue #490. Decision-ready PR: #1044.

## Context

`PlatformAPI` is currently `PLANNED`. Canonical module architecture says a general API is exposed only for a concrete client/use case and must adapt existing module application services and policies rather than become a second business-rule path.

Current `main` proves only specialized API-shaped transport surfaces:

- `POST /api/v1/game-auth/tickets` is a bounded game-login ticket issuance endpoint protected by the Passport-backed `auth:api` guard and a dedicated throttle;
- `/internal/v1/game-auth/**` is a private Gateway-service contract protected by a service credential and dedicated throttles;
- these endpoints are explicitly not a general public/first-party Platform API.

Issue #490 therefore cannot be closed by relabelling those routes. The unresolved product decision is whether Oteryn should activate a general Platform API now, and if so which consumer class should establish the first durable compatibility/security surface.

Two future consumer candidates already exist in accepted architecture, but neither currently requires a general API contract on `main`:

- PlayerCompanion may later need API reuse for calculator execution, owner workspaces, recommendations, tracking/routine preferences and share resolution; current PR #1028 is a browser/private vertical slice, not a general API consumer;
- Federated Search may later expose the same `PublicPortal` search application service through an API once that capability is implemented, without independently fanning out to source modules.

Creating a broad API before a named consumer exists would add a versioning, authentication, privacy, rate-limit and compatibility promise without a demonstrated product requirement.

## Decision question

What should be the first durable disposition for the general Oteryn Platform API?

## Non-negotiable invariants for every option

Regardless of the selected disposition:

1. Existing game-auth and internal Gateway endpoints remain specialized contracts and do not count as general PlatformAPI completeness.
2. PlatformAPI is an adapter over source-module application services/queries. It does not query module persistence directly, bypass policies, recreate business logic or become source-of-truth ownership.
3. Public/private eligibility, publication/privacy decisions, freshness/applicability, authorization and mutation authority remain with their source owners.
4. API errors never fabricate source state such as `0`, `offline`, empty results, success or completed when a dependency is unavailable/stale.
5. Versioning protects externally observable schemas and semantics. Internal PHP class stability is not an API compatibility promise.
6. Authentication class, authorization/scopes, token lifecycle, CSRF/CORS applicability, rate limits/abuse controls, cacheability and privacy are explicit per surface; existing Passport configuration is implementation capability, not blanket authorization for new APIs.
7. Sensitive/private resources default to non-public and non-shared-cache. Browser-supplied account/character identifiers never establish ownership.
8. No API endpoint obtains production activation authority merely because code or a contract is merged. Exact production/security/operations gates remain separate.
9. API telemetry must follow OperationsObservability privacy/secrecy rules and must not use high-cardinality private identifiers or credentials as metric/log labels.
10. Breaking change, deprecation and withdrawal semantics must be explicit before a surface is declared stable.

## Options

### Option A — Explicitly defer the general Platform API until a named consumer exists — RECOMMENDED

Keep `PlatformAPI` as a deliberately deferred product surface. Do not add a general route namespace merely to satisfy an audit row. Existing specialized game-auth/internal contracts remain valid but remain outside PlatformAPI classification.

Activation requires a future bounded architecture/implementation package that identifies one **named consumer and use case** and supplies, before the first stable endpoint:

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

#### Benefits

- follows the existing “concrete consumer first” architecture;
- minimizes public attack surface and long-lived compatibility commitments;
- avoids designing an abstraction around hypothetical consumers;
- allows the first API to inherit the exact privacy/freshness semantics of the product that actually needs it.

#### Costs

- no general-purpose first-party/public API exists immediately;
- future client integration requires a separate activation package before use;
- Issue #490 closes the PlatformAPI finding by explicit owner deferral rather than implementation.

### Option B — Activate a public read-only v1 first

Create a stable anonymous/read-only API for already-public Platform information, beginning only with explicitly approved public read models/application queries. Candidate capabilities are public game-data reads and, once implemented, the same federated-search application service used by PublicPortal.

The first package would need a fixed resource inventory, public enumeration/privacy policy, cache semantics, pagination, conditional/freshness metadata, anonymous abuse/rate controls, response/error versioning and compatibility/deprecation policy.

#### Benefits

- lowest authorization complexity among active API options;
- useful for first-party portal/client reuse and possible ecosystem integrations;
- can validate versioning/rate-limit/observability conventions without private mutations.

#### Costs

- creates a durable public compatibility and abuse surface before a named consumer has demonstrated need;
- public endpoints can amplify scraping/enumeration and stale-data interpretation risks;
- source publication/freshness rules must remain synchronized with web presentation without duplication.

### Option C — Activate an authenticated first-party account/client v1 first

Create a stable authenticated API intended only for an identified Oteryn first-party client. It would expose a narrowly approved owner-private resource/operation set through source-module application services. Existing Passport-backed API authentication is a reusable implementation primitive, but scopes, client registration, grant/token lifecycle and resource authorization would be designed specifically for the approved consumer rather than inherited from game-auth ticket issuance.

#### Benefits

- directly supports future native/desktop/mobile first-party client workflows;
- creates a reusable authenticated adapter around already-authorized module services;
- avoids making private workflows depend on HTML/session presentation.

#### Costs

- materially larger security/privacy surface than Option B;
- requires explicit client trust, token theft/revocation, scope, device/session, CORS/CSRF applicability and abuse decisions;
- premature account/client schemas can become difficult compatibility constraints as native CharacterId/account integration evolves.

### Option D — Activate a broad mixed public + authenticated API now — REJECTED BASELINE

Expose many public and private module capabilities under a common general v1 before a concrete consumer boundary is selected.

#### Why rejected

- maximizes attack surface and compatibility commitments;
- encourages direct cross-module/persistence access and duplicated orchestration;
- makes authorization, privacy, caching and deprecation policy too broad to validate as one bounded package;
- conflicts with the current concrete-consumer-first module architecture.

## Trade-off summary

| Option | Product immediacy | Security surface | Compatibility commitment | Architecture fit | Recommendation |
|---|---:|---:|---:|---:|---|
| A — defer to named consumer | Low now | Lowest | None beyond activation contract | Best | **Recommended** |
| B — public read-only v1 | Medium | Medium | Public/stable | Good if a public consumer is named | Acceptable alternative |
| C — authenticated first-party v1 | Medium/High | High | Private client/stable | Good if a first-party client is named | Acceptable alternative |
| D — broad mixed v1 | High | Highest | Broad/stable | Poor | Reject |

## Recommendation

Select **Option A** now.

This is an explicit product disposition, not an absence of architecture. It closes the audit ambiguity by stating that no general Platform API is launch-required until a named consumer justifies one. It also provides a fail-closed activation checklist so a future implementation cannot silently grow from the specialized game-auth namespace into a general API.

The recommendation should be revisited when one of these concrete triggers occurs:

- an approved first-party native/desktop/mobile client needs Platform resources not covered by a specialized existing contract;
- PlayerCompanion requires a non-web consumer API;
- Federated Search or another public capability has an approved machine-consumer requirement;
- an explicit partner/developer API product is promoted into scope.

## Consequences if Option A is accepted

- `PlatformAPI` remains non-implemented and is explicitly `DEFERRED` rather than ambiguously `PLANNED`;
- Issue #490's PlatformAPI audit finding becomes terminal by owner disposition, while PublicEdge/direct-production evidence remains separately open;
- no general `/api/v1` resource family is added by the architecture closeout;
- specialized game-auth/internal APIs retain their existing owners/contracts;
- `PLATFORM_API_ARCHITECTURE.md` records the activation gate and shared invariants, not a fictional endpoint inventory;
- the next API implementation requires a named consumer and a bounded handoff that satisfies the activation checklist.

## Consequences if Option B or C is accepted

The accepted option becomes architecture authority, but this architecture PR still does not implement runtime endpoints. The canonical focused architecture must define the exact first-surface profile and create a separate implementation handoff. Runtime implementation, migrations, auth/scopes, rate-limit configuration, browser/client E2E and production activation remain separately validated delivery work.

## Rejected shortcuts

- counting `POST /api/v1/game-auth/tickets` as a complete general Platform API;
- exposing controllers/models directly to avoid application-service contracts;
- declaring Passport availability equivalent to an accepted token/scopes product design;
- making all current web routes available through JSON automatically;
- using one global rate limit, cache policy or authorization rule for unrelated modules;
- promising indefinite v1 compatibility without an explicit deprecation/sunset contract.

## Implementation handoff after acceptance

After owner selection, the same bounded architecture package should:

1. mark this ADR `Accepted` with the chosen option;
2. add `PLATFORM_API_ARCHITECTURE.md` as the focused owner for API activation/versioning/adaptation invariants;
3. route it through `ARCHITECTURE_AUTHORITY.md`;
4. reconcile `MODULE_CATALOG.md`, `PORTAL_COMPLETENESS_ARCHITECTURE.md` and portal work allocation;
5. reconcile `SECURITY_ARCHITECTURE.md` only for accepted API security invariants not already owned elsewhere;
6. remove ARCH-DEC-0005 from the active backlog;
7. for Option B/C, create a separate bounded implementation Issue/handoff with exact first-surface resources and tests;
8. for Option A, create no speculative implementation handoff until an activation trigger occurs.

No external repository, protected environment, production operation or owner-funded AI service is authorized by this ADR.
