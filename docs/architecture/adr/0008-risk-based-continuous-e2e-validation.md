# ADR 0008 — Risk-based continuous E2E validation

## Status

Accepted — 2026-07-21

## Context

Oteryn Platform already has a production-like browser acceptance harness that executes the exact tested application SHA against a real Laravel HTTP runtime with isolated MariaDB, Redis and SMTP dependencies. The currently delivered staging-verifiable functional surface is classified `STAGING_PROVEN`, and the Visual / UX acceptance gate is `PASS` for that same delivered launch scope.

The existing harness is intentionally conservative: Chromium only, one primary desktop viewport, serial execution and secret-safe diagnostics for flows that may expose reset links, MFA enrollment secrets, recovery codes or authenticated session material. This baseline is valuable and must remain stable.

The next validation problem is not simply "add more E2E tests". Several proposed risks belong at different layers:

- browser-engine and responsive behavior require browser/system evidence;
- transaction races, uniqueness, locking and ambiguous commits are more deterministic at database/integration level;
- dependency interruption may require controlled production-like service orchestration rather than UI-only assertions;
- migration/rollback correctness requires existing-data and deployment-state validation;
- observability requires correlation between a user-visible request and sanitized logs/audit evidence;
- long-duration soak and repeated-run flakiness tests are useful but can be too expensive or unstable for every pull request;
- final production behavior cannot be proven by staging automation and remains governed by the Production Go-Live Gate.

Without a durable layering decision, the project risks duplicating low-level invariants in Playwright, multiplying the entire secret-sensitive suite across browsers/viewports, increasing CI flakiness and cost, or incorrectly treating staging breadth as production proof.

## Decision

### 1. Keep the existing exact-SHA production-like acceptance suite as the primary composed functional baseline

The full Chromium production-like suite remains the authoritative automated browser evidence for the currently delivered staging-verifiable functional surface unless a later ADR explicitly replaces that role.

Its evidence remains tied to the exact tested SHA and classified by environment. Passing it can support `STAGING_PROVEN`; it cannot create `PRODUCTION_PROVEN` facts.

### 2. Add coverage by risk and proof layer, not by forcing every invariant into browser E2E

Use the smallest deterministic layer that can prove the required property:

- unit tests for pure policy and transformations;
- Laravel feature/HTTP tests for middleware, validation, authorization, rate limits and server-rendered behavior;
- real-database integration tests for transactions, locking, uniqueness, concurrency and data integrity;
- contract tests for legacy engine/login-server/shared-interface assumptions;
- production-like browser/system E2E for composed user journeys, browser-specific behavior, user-visible failure/recovery and cross-surface authorization;
- deployment/operations validation for migration, rollback, backup/restore and service-topology behavior;
- final production smoke/E2E only for facts that require the actual deployed production environment.

A browser test must add evidence not already provided more reliably at a lower layer.

### 3. Use a bounded browser portability matrix

Do not multiply the full acceptance suite across every browser and viewport by default.

Maintain:

- full production-like acceptance on the primary Chromium desktop profile;
- a bounded critical portability subset on Chromium, Firefox and WebKit;
- representative desktop, tablet and mobile viewport coverage for critical public, Identity/account and privileged surfaces;
- targeted visual/accessibility collection where presentation risk justifies it.

Any broader matrix must be justified by measured defect history or launch requirements.

### 4. Prioritize continuous hardening in ordered slices

The durable priority order is:

**P0 — release-critical, bounded CI evidence**

- critical cross-browser smoke;
- responsive/mobile critical journeys;
- browser-visible security and authorization abuse boundaries;
- exact-SHA evidence metadata by browser/profile;
- representative existing-data migration/upgrade/rollback validation.

**P1 — controlled production-like resilience evidence**

- deterministic dependency interruption and recovery where it adds assertions beyond existing Phase 7/outage validation;
- user-visible data-integrity outcomes after retry/recovery;
- sanitized observability correlation across request ID, audit/log evidence and expected failure behavior;
- browser-visible outcomes of selected concurrency conflicts while retaining race correctness at integration level.

**P2 — scheduled/manual confidence profiles**

- soak/repeated-run flakiness detection;
- long-duration session/cache behavior;
- larger-volume repeated critical journeys;
- optional performance budgets when stable measurement infrastructure exists.

P2 is non-blocking by default. A bounded check may be promoted into required CI only after it is deterministic, provides unique release value and has acceptable execution cost.

### 5. Preserve secret-safe diagnostics

Raw Playwright traces, videos and automatic screenshots remain disabled by default for secret-bearing flows.

A test may opt into richer artifacts only when the captured surface is demonstrably non-secret or the artifact is sanitized by construction. Evidence must never persist passwords, reset links, TOTP secrets, recovery codes, session cookies, production credentials or private endpoints.

### 6. Failure injection must be deterministic and recovery-aware

A dependency-failure scenario is accepted only when the harness can:

1. establish a known precondition;
2. inject the failure at a controlled boundary;
3. prove fail-closed behavior and absence of false success;
4. prove relevant persisted state remains valid;
5. restore the dependency;
6. prove normal recovery.

Do not add a second workflow that merely restates evidence already proven by Phase 7 or the Platform DB outage workflow without a new assertion.

### 7. Production verification remains separate

Issue #91 and `docs/testing/PRODUCTION_SMOKE_CHECKLIST.md` remain the production-only execution boundary.

Cross-browser, chaos, migration, observability or soak success in repository/staging automation does not promote production edge, TLS, origin, database, Redis, mail, deployment, backup or runtime facts to `PRODUCTION_PROVEN`.

### 8. The authoritative game-login bridge remains a separate cross-repository requirement

When the separately authorized Platform-originated game-login bridge exists, its critical end-to-end properties must be added to the validation architecture, including credential authority, expiry/replay, revocation, disabled/banned state and character usability.

This ADR does not authorize writes to legacy engine or login-server repositories.

## Consequences

- The project gains broader E2E confidence without turning Playwright into a duplicate of every lower-level test layer.
- Cross-browser and mobile regressions can be detected with bounded CI cost.
- Concurrency and integrity assertions remain deterministic where they are strongest.
- Resilience tests must prove recovery, not only error pages.
- Migration/rollback behavior becomes a first-class validation concern for releases with existing data.
- Long-running tests can detect flakiness and leaks without making every PR depend on unstable soak infrastructure.
- Secret-bearing acceptance flows retain conservative artifact handling.
- The evidence taxonomy remains honest: staging breadth does not equal production proof.

## Rejected alternatives

### Run the entire full acceptance suite on every browser and viewport for every PR

Rejected as the default. It multiplies CI time and flakiness, including secret-sensitive flows, without proving proportionally more risk. A bounded portability subset is preferred unless measured defects justify expansion.

### Move all concurrency and data-integrity validation into Playwright

Rejected. Browser automation is a poor primary proof layer for transaction locking, races and ambiguous commits. Real-database integration remains authoritative for those invariants, with browser tests limited to composed user-visible outcomes.

### Treat chaos testing as random service killing

Rejected. Non-deterministic disruption produces weak evidence and flaky CI. Failure injection must have controlled preconditions, bounded disruption and explicit recovery proof.

### Make soak tests required on every pull request

Rejected by default. Soak/repeated-run profiles are scheduled/manual until they are proven deterministic, bounded and release-critical.

### Use expanded staging E2E as a substitute for final production verification

Rejected. The Production Go-Live Gate remains independent and fail closed.

## Follow-up

Execute `docs/testing/E2E_COVERAGE_ROADMAP.md` in bounded slices under task `OTERYN-20260721-e2e-coverage-hardening`. Record exact-SHA/browser/profile evidence and split follow-up work when a slice would make one PR too broad.
