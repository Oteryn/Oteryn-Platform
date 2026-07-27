# Oteryn Platform Test Strategy

## Goal

Make security, business invariants, delivered portal behavior and Canary/login-server compatibility verifiable rather than dependent on manual assumptions.

The strategy is risk based: use the smallest deterministic layer that proves an invariant and reserve browser/system E2E for composed behavior that lower layers cannot prove efficiently.

Authoritative decisions:

- ADR 0008 governs risk-based continuous E2E validation, bounded browser matrices and proof-layer selection.
- ADR 0015 governs the machine-enforced delivered-surface acceptance ledger and complete account-lifecycle profile.
- `docs/testing/E2E_COVERAGE_ROADMAP.md` governs additive E2E hardening.
- `docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md` summarizes current delivered-surface coverage and exact gaps.

No repository or staging evidence can create `PRODUCTION_PROVEN` facts. Final production verification remains a separate gate.

## Test layers

### Unit tests

Use for pure domain/application logic:

- validation rules not coupled to HTTP;
- permission and policy decisions with deterministic inputs;
- value objects and transformations;
- token/expiry policy helpers;
- URL/content/media safety policies;
- future ledger calculations.

### Feature/HTTP tests

Use Laravel feature tests for:

- routes and middleware;
- authentication and logout;
- CSRF-relevant browser requests;
- authorization policies and exact permission boundaries;
- validation, errors and rate limiting;
- server-rendered empty/unavailable/conflict/not-found states;
- admin, CMS and public pages;
- signed URL and response-header behavior.

Feature tests remain the preferred deterministic layer for request/authorization behavior that does not require a real browser engine.

### Database integration tests

Use isolated test databases for:

- migrations and rollback compatibility;
- transactions and atomic mutations;
- row locking and concurrency;
- uniqueness constraints;
- account/character integration adapters;
- idempotent retry and ambiguous-commit recovery;
- data-integrity invariants across approved shared-write boundaries;
- media reference and deletion locks.

Never point automated tests at production data.

Browser E2E must not replace real-database integration as the primary proof for locking, transaction races, uniqueness or persisted integrity. Browser scenarios may assert the user-visible outcome of a conflict only when that adds unique composed evidence.

### Contract tests

Required for shared Canary/login-server assumptions.

Contract tests verify only evidence-backed schemas/interfaces and fail visibly when incompatible changes occur. Examples:

- required shared columns/types exist;
- read queries return expected shapes;
- approved account/character mutations preserve invariants;
- auth/session/game-ticket behavior matches the documented contract;
- external revisions/SHAs match the tested compatibility package.

### End-to-end and production-like browser tests

The exact-SHA Playwright harness under `scripts/acceptance/**` and `.github/workflows/acceptance-validation.yml` runs against:

- the exact tested application SHA;
- a real Laravel HTTP runtime;
- isolated MariaDB Platform and Canary acceptance schemas;
- operation-specific Canary database principals;
- a dedicated Redis runtime principal;
- MailHog SMTP;
- Chromium as the primary complete browser;
- bounded Firefox/WebKit projects;
- deterministic synthetic fixtures;
- conservative secret-safe diagnostics.

Browser E2E proves composed outcomes such as navigation, cookies/sessions, cross-surface authorization, browser-visible validation, responsive interaction, focus behavior and user-visible dependency recovery.

Passing controlled browser/system evidence may support `STAGING_PROVEN`. It does not prove the final production environment.

## Machine-enforced delivered-surface acceptance contract

`scripts/acceptance/coverage/portal-coverage-manifest.json` is the canonical inventory of delivered portal surfaces.

Each surface declares:

- stable surface ID and owner;
- exact named Laravel routes;
- required roles/authorization states;
- required functional/data states;
- required viewport and browser scope;
- required proof layers;
- current state: `covered`, `partial`, `planned`, `supporting_endpoint` or `not_delivered`;
- evidence file paths and stable markers;
- exact remaining gaps.

`npm --prefix scripts/acceptance run test:coverage-contract` validates manifest integrity and the exact runtime route table. It fails for:

- an unclassified named route;
- a stale/nonexistent manifest route;
- duplicate route ownership;
- missing dimensions;
- missing evidence files or markers;
- a `covered` record with unresolved gaps;
- a `partial` or `planned` record without an exact gap.

`npm --prefix scripts/acceptance run test:coverage-contract:strict` additionally fails while any delivered required surface remains `partial` or `planned`. Strict mode becomes a release gate only after all truthful gaps are closed.

“Exhaustive” means complete against this versioned delivered-surface contract at the correct proof layers. It is not a guarantee that unknown defects cannot exist.

## Complete account-lifecycle profile

`.github/workflows/portal-acceptance-contract.yml` runs a dedicated zero-retry Chromium profile through:

```text
npm --prefix scripts/acceptance run test:account-lifecycle
```

The profile composes the complete currently delivered account journey:

- registration success, validation and duplicate identity;
- invalid/successful login and logout;
- protected-route redirect/denial;
- Account Overview navigation and safe account data;
- Canary provisioning/binding states: ready, pending, recoverable, conflict and missing;
- recoverable retry and absence of raw Canary identifiers/provisioning names;
- password recovery through real test SMTP;
- reset token expiry/replay denial;
- password change and old-password denial;
- invalidation of other sessions after reset/change;
- MFA enrollment, confirmation, challenge, invalid/replayed TOTP, recovery-code single use and disable-all-sessions behavior;
- character creation authorization, validation, reserved/duplicate/quota/ownership/idempotency boundaries;
- public visibility of a created character;
- returning-user MFA sign-in.

The profile remains primary-Chromium because it handles reset links, TOTP secrets, recovery codes and authenticated session material. Cross-browser evidence stays bounded to representative safe flows under ADR 0008.

Raw traces, video and automatic screenshots remain disabled for secret-bearing flows. Explicit failure screenshots must mask inputs, textareas and code blocks.

## Implemented acceptance profiles

Current projects/profiles include:

- `chromium-primary` — complete primary Chromium baseline at `1440x1000`;
- `portability-chromium`, `portability-firefox`, `portability-webkit` — bounded critical portability at desktop size;
- `responsive-desktop`, `responsive-tablet`, `responsive-mobile` — representative responsive journeys;
- `resilience-chromium` — deterministic dependency failure, restoration and successful recovery;
- `accessibility-chromium` — bounded real keyboard/focus interaction;
- `soak-chromium` — scheduled/manual read-only public-surface calibration;
- `account-lifecycle` — complete zero-retry Identity/account/provisioning/security/character journey selected by `@portal-account`;
- `portal coverage classification` — machine-enforced live route/evidence contract.

The existing required pull-request `critical` profile preserves primary smoke plus bounded portability, responsive, resilience and accessibility coverage.

The existing `full` profile preserves the complete primary Chromium functional baseline plus resilience/accessibility before exploratory visual/accessibility collection.

The separate `Portal Acceptance Contract` workflow requires route classification and the complete account lifecycle for affected changes.

The complete secret-sensitive suite is not multiplied across every browser/viewport merely to increase test count.

## Responsive, portability and accessibility rules

Use representative dimensions based on concrete risk:

- public navigation and content on desktop/tablet/mobile;
- Identity entry forms and MFA challenge on small screens;
- Account Overview and character creation on desktop/mobile;
- table-heavy administrator surfaces on desktop/tablet/mobile;
- critical public/admin flows in bounded Chromium/Firefox/WebKit projects.

Accessibility browser evidence may prove:

- labels and accessible names;
- semantic headings/landmarks;
- keyboard traversal and activation;
- visible focus;
- bounded overflow and status communication.

Automated checks do not prove full screen-reader conformance. Manual assistive-technology evidence remains separate.

## Repeated-run stability validation

`.github/workflows/acceptance-stability.yml` provides scheduled/manual repeated critical execution:

- fresh isolated dependencies per iteration;
- zero Playwright retries;
- `fail-fast: false` so one failure does not hide later evidence;
- distinct run/iteration identities;
- classification of the first failure as product, harness or infrastructure.

A test that fails one iteration is not considered healthy because another iteration passes.

## Read-only soak validation

`.github/workflows/acceptance-soak.yml` runs bounded repeated public reads over home, online, highscores and servers.

It records calibration metrics such as:

- measured duration and navigation distributions;
- Laravel process-tree RSS;
- Redis key count before/after.

The soak performs no Identity, MFA, password, account, character or privileged mutation. No performance budget is enforced until repeated evidence establishes normal variance.

## E2E layering and expansion rules

Before adding browser/system coverage:

1. Identify the concrete risk.
2. Search existing unit/feature/integration/contract/operations evidence.
3. State what unique browser/system proof is missing.
4. Keep the scenario deterministic and exact-SHA.
5. Preserve secret-safe artifacts.
6. Classify the result by environment rather than aspiration.
7. Add the exact route/state/role/viewport/evidence mapping to the manifest.

Do not add browser tests solely to increase test count.

Continuous hardening preserves these rules:

- full primary-browser acceptance remains the composed functional baseline;
- bounded Chromium/Firefox/WebKit proves representative portability;
- representative desktop/tablet/mobile profiles cover material responsive risks;
- keyboard/focus interaction is required where declared;
- dependency interruption must prove known pre-state, fail-closed behavior, restoration and recovery;
- concurrency/data integrity remains primarily database integration evidence;
- migration/rollback uses representative synthetic existing data, never production dumps;
- observability correlation uses sanitized request IDs/log outcomes where deterministic;
- repeated-run and soak remain scheduled/manual until evidence justifies stronger gates.

## Security regression tests

Every confirmed vulnerability fix should add a focused regression where practical.

Priority areas:

- IDOR/account ownership;
- privilege escalation;
- CSRF;
- XSS/sanitization;
- SQL injection/query safety;
- session fixation/revocation;
- password reset enumeration/replay;
- MFA bypass;
- rate-limit bypass;
- upload/media validation and authorization;
- shared-data races;
- future webhook/payment replay.

Use browser E2E for representative composed abuse boundaries only when it adds proof beyond deterministic feature/security tests.

## Failure and recovery validation

A controlled failure scenario must establish:

1. known pre-state;
2. deterministic failure injection;
3. fail-closed/no-false-success behavior;
4. relevant persisted-state integrity;
5. dependency restoration;
6. successful recovery.

Search existing Phase 7, Platform DB outage and browser resilience evidence before adding new orchestration. Do not duplicate a workflow without a new assertion.

## Migration, upgrade and rollback validation

For persistent-data/schema releases:

- create representative synthetic pre-upgrade data;
- apply the exact candidate transition;
- verify schema/data invariants;
- run bounded candidate smoke;
- exercise controlled rollback only where the contract supports it;
- verify post-rollback state;
- redeploy/reapply idempotently where required.

Migration/browser smoke complements but does not replace migration/database integration tests. Never use copied production dumps in CI.

Durable evidence is recorded in `docs/testing/E2E_MIGRATION_ROLLBACK_EVIDENCE.md`.

## Observability correlation validation

The controlled running-HTTP path proves that a concrete response `X-Request-ID` maps to exactly one structured `http.request.completed` event with the same identifier and expected method/status.

Durable evidence is `docs/testing/E2E_OBSERVABILITY_CORRELATION_EVIDENCE.md`.

This does not prove production edge propagation, centralized shipping, retention, alerts or distributed tracing.

## Test data and artifact safety

- use factories/seeders designed for tests;
- use synthetic identities, email addresses and content;
- never copy production dumps or personal data;
- never commit/persist passwords, reset links, TOTP secrets, recovery codes, session cookies, game/OAuth tickets, production credentials or private endpoints;
- cross-repository fixtures must identify the represented schema/version evidence;
- resilience mutations may affect only disposable acceptance-scoped principals and must restore them;
- repeated runs use isolated cache/session/limiter/dependency state;
- richer diagnostics are allowed only for demonstrably non-secret or sanitized surfaces.

## CI direction

Mandatory application CI includes:

1. strict Composer validation/install;
2. Composer advisory audit;
3. formatting;
4. PHPStan/Larastan level 10;
5. full tests.

Acceptance/release workflows provide:

- required `critical` browser profile;
- required portal route-classification contract for affected changes;
- required complete zero-retry account-lifecycle profile for affected changes;
- standalone smoke, portability, responsive, resilience, accessibility, soak and full profiles;
- full exact-SHA primary Chromium functional acceptance and visual collection;
- scheduled/manual repeated-run and soak evidence;
- required representative existing-data migration/rollback/redeploy validation;
- required response-to-log observability correlation.

Do not broaden the complete secret-sensitive suite to every browser/viewport before measured evidence shows proportional value.

## Merge expectations

- inspect the full changed-file list and diff;
- security-critical changes require focused regression tests;
- shared-data changes require contract/integration evidence;
- do not merge on evidence from an old head;
- do not weaken/delete/skip valid failing tests;
- classify unavailable environments honestly;
- E2E evidence identifies exact SHA, profile, project/browser and applicable viewport;
- browser-specific skips require explicit reason and do not silently count as coverage;
- repeated-run failure must not be hidden by retries;
- resilience must prove cleanup/restoration/recovery;
- a new named route must be classified before merge;
- a manifest record becomes `covered` only after every declared dimension has proof;
- strict coverage is enabled only after every delivered gap is truthfully closed.

## Production readiness E2E matrix

Before a production-ready/go-live claim, directly verify the applicable launch scope in the final production environment:

| Flow | Required |
|---|---|
| Registration | Yes if enabled |
| Web login/logout | Yes |
| Password change/reset | Yes |
| Session revocation | Yes |
| Admin MFA and authorization denial | Yes |
| Account provisioning | Yes if enabled |
| Character creation/public visibility | Yes if enabled |
| Critical CMS/public visibility | Yes if enabled |
| Canary/game login | Yes only if the authoritative bridge is in launch scope |
| Ban/disabled/expiry/replay | Yes when an authoritative applicable path exists |
| Backup/restore/deployment/rollback | Operational production evidence |

The production execution boundary is issue #91 plus `docs/operations/PRODUCTION_READINESS_CHECKLIST.md`, `docs/operations/PRODUCTION_VERIFICATION_EVIDENCE.md` and `docs/testing/PRODUCTION_SMOKE_CHECKLIST.md`.

The authoritative Platform-originated game-login bridge remains separately authorized. When implemented, add exact cross-repository E2E for credential authority, expiry/replay, revocation, disabled/banned state and character usability.

Repository/staging evidence never substitutes for direct final-production proof.
