# Oteryn Exhaustive Portal Acceptance Agent Prompt

Use the following prompt in a fresh agent session to close the remaining machine-classified portal acceptance gaps.

```text
Continue Oteryn Platform acceptance work from the current repository state. Do not rely on previous chat history.

REPOSITORY WRITE ALLOWLIST:
- Writes are allowed only in Oteryn/Oteryn-Platform.
- Treat blakinio/canary, upstream Canary, login-server, OTClient and every other repository as read-only unless the user explicitly authorizes a separate write task.

PROGRAM: Exhaustive Delivered-Surface Portal Acceptance
RECOMMENDED_MODE: CODEX
MODE_REASON: local Laravel inspection, Playwright implementation, fixtures, validation and CI repair are required.

GOAL:
Close every `partial` and `planned` delivered portal surface in `scripts/acceptance/coverage/portal-coverage-manifest.json` through bounded, risk-layered test packages. Finish with strict route/state/role/viewport/evidence validation passing on the exact final head.

DEFINITION OF COMPLETE:
- Every currently delivered named portal route is classified exactly once.
- Every required state, role, viewport, browser scope and evidence layer is explicit.
- Every delivered surface is `covered` or a reviewed `supporting_endpoint`.
- `npm --prefix scripts/acceptance run test:coverage-contract:strict` passes.
- The existing full and critical acceptance contracts remain green.
- The result is complete against the declared delivered-surface contract, not a claim that unknown defects cannot exist.

MANDATORY READS:
- AGENTS.md
- docs/agents/REPOSITORY_MAP.md
- docs/agents/CONTEXT_ROUTING.md
- docs/agents/PROJECT_STATE.md
- docs/agents/BUILD_TEST_MATRIX.md
- docs/agents/CONTEXT_HANDOFF.md
- docs/agents/tasks/TASK_TEMPLATE.md
- docs/architecture/TEST_STRATEGY.md
- docs/architecture/adr/0008-risk-based-continuous-e2e-validation.md
- docs/architecture/adr/0015-machine-enforced-portal-acceptance-ledger.md
- docs/testing/E2E_COVERAGE_ROADMAP.md
- docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
- docs/acceptance/VISUAL_UX_ACCEPTANCE_MATRIX.md
- scripts/acceptance/coverage/portal-coverage-manifest.json
- scripts/acceptance/coverage/validate-portal-coverage.mjs
- scripts/acceptance/package.json
- scripts/acceptance/playwright.config.mjs
- .github/workflows/acceptance-validation.yml
- .github/workflows/portal-acceptance-contract.yml

CONTEXT ROUTES:
- agent-governance
- architecture
- testing
- web-cms
- auth-identity when Identity/account behavior is touched
- accounts-characters when account/character behavior is touched
- admin-rbac for privileged surfaces
- public-game-data for game-data surfaces
- security for every authorization, session, upload or untrusted-content boundary
- database only when migrations, transactions or persistence fixtures are involved

SEARCH FIRST:
- active tasks and open PRs for overlapping acceptance, route, fixture, view, module and workflow paths;
- all current named routes using `php artisan route:list --json`;
- nested AGENTS.md files affecting intended paths;
- existing feature/unit/integration/contract tests before adding browser duplication;
- existing Playwright fixtures/helpers/specs and visual/accessibility collectors;
- current module routes, controllers, requests, policies, views and exact permissions;
- current staging/full evidence for the target surface;
- any scheduled stability or soak evidence relevant to the changed profile.

MANDATORY WORKFLOW:
1. Perform the lean preflight once.
2. Run `npm --prefix scripts/acceptance run test:coverage-contract` and record the exact current classification result.
3. Select exactly one bounded gap package from the priority order below.
4. Create a dedicated active task from the repository template with exact owned paths.
5. Create a task branch from current synchronized main and open a draft PR early.
6. Inspect the selected surface at every applicable proof layer.
7. Implement the smallest complete evidence package; do not refactor unrelated product code.
8. Add/update deterministic fixtures and browser tests only when they provide unique composed proof.
9. Run focused lower-layer tests, the target Playwright project/profile and the coverage validator.
10. Run the full applicable exact-head repository checks and inspect failures/artifacts.
11. Fix root causes; never weaken, skip or delete a valid check.
12. Change a manifest status to `covered` only after all declared dimensions have exact evidence.
13. Update the human-readable matrix when the status/gap materially changes.
14. Keep the task checkpoint current and leave exactly one next_action.
15. Merge only when the repository merge gate is satisfied, then archive the task separately if repository policy requires it.

PRIORITY ORDER:
1. Downloads public/admin/localization lifecycle.
2. Events public/admin/status lifecycle.
3. Announcements administration/localization and homepage composition lifecycle.
4. Support and legal public/admin/localization route-complete lifecycle.
5. Editorial Media administrator upload/library/delete/reference-lock lifecycle.
6. Public Wiki evidence reconciliation against every required state.
7. Wiki administration evidence reconciliation across roles, lifecycle, revisions, conflict and signed previews.
8. Route-ledger drift cleanup and any newly discovered delivered route.
9. Final strict-gate activation after every required surface is closed.

DO NOT COMBINE ALL PRIORITIES INTO ONE PR.
Each package should normally own one module or one tightly coupled evidence boundary. Split work when fixtures, permissions, uploads, localization or lifecycle orchestration would make review broad.

ACCOUNT-LIFECYCLE NONREGRESSION GATE:
Every package that touches Identity, navigation, layouts, sessions, permissions, account or character surfaces must run:
- `npm --prefix scripts/acceptance run test:account-lifecycle`
- `npm --prefix scripts/acceptance run test:coverage-contract`

The account profile must continue to prove:
- registration and duplicate/validation behavior;
- invalid and successful login;
- logout and protected-route denial;
- Account Overview and ready/pending/recoverable/conflict/missing provisioning states;
- safe provisioning retry and no raw internal identifiers;
- password recovery/change with session revocation and replay denial;
- MFA enrollment/challenge/replay/recovery/disable lifecycle;
- character creation validation, ownership, quota/idempotency and public visibility;
- returning-user MFA journey.

LAYERING RULE:
Use the smallest deterministic proof layer:
- unit for pure policy/transformation;
- feature/HTTP for routes, validation, middleware, rate limits, policy and server-rendered states;
- database/integration for transactions, uniqueness, locks, races, idempotency and persisted integrity;
- contract for Canary/login-server/shared interface assumptions;
- Playwright for composed navigation, browser/session behavior, cross-surface authorization, responsive interaction and visible recovery;
- visual/accessibility for presentation risks;
- operations/release validation for migration, rollback, dependency topology and recovery;
- production smoke only in the final real production environment.

Do not add Playwright tests merely to increase test count. A browser scenario must name the unique proof it adds.

SURFACE ACCEPTANCE TEMPLATE:
For the selected manifest surface, verify and record:
- every route name still exists and belongs only to that surface;
- direct URL and normal navigation where applicable;
- guest/authenticated/MFA/permission states that change behavior;
- success and every required validation, empty, stale, unavailable, conflict, not-found, rate-limit or recovery state;
- desktop/tablet/mobile only where declared;
- Chromium full path and bounded Firefox/WebKit only where declared;
- no unexpected console/page/request/server errors;
- no raw framework, SQL, secret, token or internal identifier leakage;
- CSRF, ownership/IDOR, privilege escalation, replay and sanitization boundaries where relevant;
- accessibility labels, headings, keyboard activation and visible focus where declared;
- deterministic cleanup and successful recovery after any injected failure;
- exact evidence file and stable marker in the manifest.

SECRET AND ARTIFACT SAFETY:
- Raw trace, video and automatic screenshots remain disabled for password, reset-link, TOTP, recovery-code and authenticated-session flows.
- Mask inputs, textareas and code blocks in any explicit failure screenshot for secret-bearing pages.
- Never persist passwords, reset URLs, TOTP secrets, recovery codes, cookies, OAuth/game tickets, production credentials or private endpoints.
- Use only synthetic test identities and data.
- Do not copy production databases or personal data.

MODULE-SPECIFIC MINIMUMS:

Downloads:
- public empty/current release/platform filter and invalid platform behavior;
- approved direct HTTPS URL policy and browser-visible safe metadata;
- administrator create/edit/publish and exact permission/MFA denial;
- English/Polish effective content and stale/incomplete translation behavior;
- no executable upload/proxy/fetch behavior.

Events:
- public index/detail, empty, upcoming/active/archived/cancelled and not-found states;
- administrator create/edit/status transitions;
- exact manage versus publish permissions;
- optimistic-lock conflict and bounded audit evidence;
- English/Polish behavior and responsive layouts.

Announcements:
- deterministic active-window homepage composition and none-active state;
- administrator create/edit and PL translation;
- stale conflict, invalid link and permission denial;
- escaped plain text and bounded audit metadata.

Support/Legal:
- every typed public route in EN and PL;
- published, unpublished and missing states;
- legal version/effective-date presentation;
- approved support/contact links only;
- administrator edit/translation, validation, exact permission/MFA and audit.

Editorial Media:
- supported JPEG/PNG/WebP upload success;
- MIME/container/decode/dimension/pixel/size rejection paths;
- private content/thumbnail authorization;
- deletion and reference-lock behavior;
- integrity failure and storage failure;
- exact permission/MFA denial;
- secret-safe browser artifacts.

Wiki public/admin reconciliation:
- do not invent new scenarios before mapping existing specs to every manifest requirement;
- correct stale paths/markers;
- add only genuinely missing route/state/role/viewport evidence;
- preserve publication freshness, locale isolation, restricted Markdown, signed preview, optimistic locking, revisions, media reference and exact permission contracts.

STRICT-GATE ACTIVATION:
Do not enable strict closure while any truthful gap remains. The final gate PR must:
- prove every delivered required surface is `covered` or reviewed `supporting_endpoint`;
- run manifest validation, strict validation, account lifecycle, critical and full acceptance on the exact final head;
- include no unexplained exclusion or empty evidence marker;
- fail if a new named route is introduced without classification;
- update TEST_STRATEGY, the coverage matrix and project/task state;
- preserve production and game-login nonclaims.

DEFERRED / NOT AUTHORIZED:
- writes to Canary, login-server, OTClient or another repository;
- authoritative Platform-to-game login implementation;
- existing Canary account import/claim;
- account delete/unlink/rebind/transfer;
- character rename/delete;
- payments, webshop, coins or commerce;
- production deployment or production-data testing;
- broad product redesign unrelated to an observed acceptance defect.

STOP CONDITIONS:
- overlapping owned paths with an active task/PR;
- requirement to expose or persist a secret/private artifact;
- destructive migration without explicit scope and rollback evidence;
- unresolved authorization or cross-repository compatibility conflict;
- inability to reproduce an evidence claim deterministically;
- pressure to mark `covered`, skip a route or enable strict mode without proof;
- pressure to treat staging evidence as production proof.

DELIVERY REPORT:
For each completed package report:
- task ID, PR, exact head and base;
- surface ID and route names;
- previous and final coverage status;
- states/roles/viewports/browsers proved;
- lower-layer and browser commands/results;
- CI workflow run/job IDs and artifact digest where available;
- exact remaining unknowns/nonclaims;
- exactly one next action.
```
