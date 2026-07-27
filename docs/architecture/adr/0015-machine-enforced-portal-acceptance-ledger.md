# ADR 0015 — Machine-enforced portal acceptance ledger

## Status

Accepted — 2026-07-27

## Context

Oteryn Platform has an exact-SHA production-like Playwright harness, a full primary Chromium baseline, bounded Chromium/Firefox/WebKit portability, representative desktop/tablet/mobile coverage, public dependency recovery, keyboard/focus interaction, migration/rollback validation and visual evidence. That architecture is governed by ADR 0008 and remains valid.

The delivered portal has grown beyond the original acceptance inventory. Identity, Account Overview, Canary provisioning, characters, CMS, RBAC, audit, Downloads, Events, Announcements, Support/Legal, Wiki and Editorial Media now expose many browser-visible routes and materially different states. Existing proof is real but distributed across browser specs, Laravel feature tests, database/integration tests, module-specific security tests, release validation and operations evidence.

A green browser suite alone therefore does not answer all of these questions deterministically:

- Is every currently delivered browser-visible route known to the test architecture?
- Are all required states, roles and viewport classes explicitly classified?
- Does a new route silently enter the product without an acceptance owner?
- Which surfaces have complete composed browser evidence, partial evidence or a planned gap?
- Are concurrency, locking and data integrity still proven at the correct lower layer rather than duplicated unreliably in Playwright?
- Does the complete account lifecycle run as one recognizable release-critical profile?

The project needs exhaustive coverage against a versioned delivered-surface contract, while remaining honest that no finite test suite proves universal absence of unknown defects or final production correctness.

## Decision

### 1. Introduce one canonical portal acceptance ledger

`scripts/acceptance/coverage/portal-coverage-manifest.json` is the machine-readable source of truth for delivered portal acceptance classification.

Every delivered browser surface must declare:

- a stable surface identifier;
- one or more current named Laravel routes;
- owner/module;
- required user roles or authorization states;
- required functional/data states;
- required viewport classes;
- required browser scope;
- required evidence layers;
- current coverage state;
- evidence file references and stable marker strings;
- an explicit reason and follow-up when coverage is partial or planned.

Binary/media endpoints, signed previews, redirects and non-page browser resources remain classified in the same ledger as `supporting_endpoint` rather than disappearing from the inventory.

### 2. Discover current routes at runtime and fail on silent drift

The coverage validator obtains the exact route table from the tested application through `php artisan route:list --json`.

It must fail when:

- a named browser-visible GET/HEAD route is not classified;
- a manifest route no longer exists;
- a route is classified more than once;
- an exclusion lacks a bounded reason;
- an evidence file or marker is missing;
- required dimensions are empty;
- a `covered` surface contains an unresolved required evidence gap.

The initial required CI gate enforces **classification completeness** and manifest integrity. A strict mode additionally fails for every `partial` or `planned` required surface. Strict mode becomes the release gate only after the implementation programme closes the recorded gaps; this avoids pretending that architecture scaffolding itself has already produced missing browser evidence.

### 3. Define exhaustive coverage as a cross-layer contract

“Exhaustive” means every delivered surface and required state is explicitly mapped to the smallest deterministic proof layer:

- unit tests for pure policies and transformations;
- Laravel feature/HTTP tests for routes, middleware, validation, authorization, rate limits and server-rendered states;
- database/integration tests for transactions, uniqueness, locking, races, retries and persisted integrity;
- contract tests for Canary/login-server/shared interfaces;
- Playwright E2E for composed journeys, browser behavior, user-visible state transitions, cross-surface authorization and recovery;
- visual/responsive/accessibility evidence for presentation risk;
- deployment/operations validation for topology, migration, rollback, backup/restore and service lifecycle;
- direct production smoke only for the actual deployed production environment.

A route can be completely classified without forcing every invariant into a browser. Conversely, a feature test does not replace composed browser proof where navigation, cookies, session transitions, focus, responsive layout or cross-surface behavior is the risk.

### 4. Make the complete account lifecycle a release-critical browser profile

The acceptance package exposes a dedicated zero-retry account-lifecycle command and workflow profile.

The profile composes existing and new browser scenarios covering:

1. registration success, validation and duplicate identity handling;
2. login success/failure, redirect behavior and logout;
3. Account Overview authorization and navigation;
4. Canary binding/provisioning states: ready, pending, recoverable, conflict and missing;
5. safe provisioning retry and absence of raw internal identifiers;
6. password recovery through real test SMTP, expiration/replay and session revocation;
7. authenticated password change and revocation of other sessions;
8. MFA enrollment, QR/manual provisioning boundary, confirmation, challenge, invalid/replayed TOTP, recovery-code single use and disable-all-sessions behavior;
9. character creation authorization, validation, ownership, quota/idempotency boundaries and public visibility;
10. returning-user sign-in with MFA and protected-route denial after logout or session invalidation.

Secret-bearing traces, video and automatic screenshots remain disabled. Any explicit screenshot must mask or avoid passwords, reset links, TOTP secrets, recovery codes and session material.

### 5. Require dimensions, not a blind browser/viewport cross-product

Each surface declares the dimensions that are material to its risk:

- `guest`, `authenticated`, `mfa_pending`, `mfa_confirmed`, `permission_denied`, exact privileged role/permission where relevant;
- success, validation, empty, unavailable, stale, conflict, not-found, rate-limited or recovery states as applicable;
- desktop, tablet and mobile where layout or interaction risk exists;
- full Chromium for the composed baseline and bounded Firefox/WebKit where portability evidence is meaningful.

The ledger must not mark a dimension required merely to inflate test count. Required dimensions need a concrete risk statement.

### 6. Separate coverage state from evidence classification

Coverage state:

- `covered` — all required ledger dimensions have current evidence at their assigned layers;
- `partial` — some required dimensions are proven and exact gaps are recorded;
- `planned` — delivered surface is classified but the required evidence package is not yet implemented;
- `supporting_endpoint` — browser-consumed endpoint/resource with bounded non-page evidence;
- `not_delivered` — planned capability is outside the current product and must not be presented as tested.

Environment classification remains unchanged:

- `PROVEN` for deterministic repository evidence at the stated layer;
- `STAGING_PROVEN` for directly exercised controlled production-like evidence;
- `PRODUCTION_PROVEN` only for direct verification in the final production environment;
- `UNKNOWN` otherwise.

`covered` never implies `PRODUCTION_PROVEN`.

### 7. Preserve explicit external boundaries

The ledger and account profile do not authorize or claim:

- the still-separate authoritative Platform-to-game login bridge;
- writes to Canary, login-server, OTClient or another repository;
- final production DNS/TLS/WAF/origin/database/Redis/mail/deployment behavior;
- existing-account import/claim, account deletion, unlink/rebind/transfer, character rename/deletion or other deferred product functions;
- screen-reader conformance from automated keyboard/focus checks alone;
- universal absence of unknown defects.

## Consequences

- Every delivered portal route gains an explicit test owner and coverage status.
- New routes cannot silently bypass acceptance classification.
- Account creation, login, account management, provisioning, security and character entry become one visible release-critical profile instead of a set of unrelated specs.
- Remaining gaps are measurable and executable by bounded follow-up tasks.
- ADR 0008 remains authoritative for proof-layer selection and bounded browser matrices.
- CI cost increases for the critical account profile, but the complete secret-sensitive suite is still not multiplied across every browser and viewport.
- The project can say “complete against the declared delivered-surface contract” only after strict validation passes; it still cannot say “guaranteed bug-free.”

## Rejected alternatives

### Claim completeness from code coverage percentage

Rejected. Line/branch coverage does not prove route discovery, composed navigation, authorization states, dependency recovery, responsive usability or production behavior.

### Put every invariant into Playwright

Rejected. Transaction races, locks, uniqueness and persisted integrity are more deterministic in real-database integration tests. Playwright remains responsible for unique browser/system outcomes.

### Enable strict completeness immediately while known gaps remain

Rejected. That would either break every change indefinitely or pressure maintainers to add dishonest exclusions. Classification completeness is required first; strict completeness is enabled after the bounded implementation programme closes all required gaps.

### Treat an excluded route as untested without explanation

Rejected. Every exclusion must identify why the route is not a portal page, which lower layer proves it, and who owns reclassification if behavior changes.

## Follow-up

Use `docs/agents/prompts/OTERYN-EXHAUSTIVE-PORTAL-ACCEPTANCE-AGENT-PROMPT.md` to close the ledger gaps in bounded PRs. The final programme PR enables strict coverage validation only when every delivered required surface is `covered` or a reviewed `supporting_endpoint`.
