---
task_id: OTERYN-20260727-portal-acceptance-final-closure
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/adr/0015-machine-enforced-portal-acceptance-ledger.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
search_first:
  - open pull requests and active tasks owning Support/Legal, Editorial Media, Wiki or portal acceptance ledger paths
  - existing Support/Legal, Editorial Media and Wiki feature/browser tests and fixtures
optional_reads:
  - docs/architecture/SECURITY_ARCHITECTURE.md
---

# OTERYN-20260727-portal-acceptance-final-closure

## Goal

Close the remaining Issue #240 portal acceptance records for Support/Legal and Editorial Media plus Wiki public/admin with deterministic zero-retry browser evidence, strict ledger reconciliation and no production claim.

## Acceptance criteria

- [x] Support/Legal proves all delivered public routes, EN/PL isolation, approved links, missing/unpublished/legal-version states and exact MFA/RBAC administrator editing.
- [x] Editorial Media proves exact MFA/RBAC, upload validation, private preview/content, deletion and referenced-media protection.
- [x] Wiki public/admin reconciles every required state with stable exact markers and fills genuine browser gaps without weakening existing tests.
- [x] Each module has an isolated workflow/config/fixture boundary and zero retries.
- [x] The canonical ledger moves the four remaining records to `covered` only after module browser proof passes.
- [ ] Issue #240 is closed only after the strict ledger and every exact-head repository workflow pass; no `PRODUCTION_PROVEN` claim is made.

## Ownership

```yaml
owned_paths:
  - .github/workflows/portal-acceptance-contract.yml
  - .github/workflows/support-legal-acceptance.yml
  - .github/workflows/editorial-media-acceptance.yml
  - .github/workflows/wiki-reconciliation-acceptance.yml
  - scripts/acceptance/playwright.support-legal.config.mjs
  - scripts/acceptance/playwright.editorial-media.config.mjs
  - scripts/acceptance/playwright.wiki-reconciliation.config.mjs
  - scripts/acceptance/seed-browser-support-legal.php
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/seed-browser-wiki-reconciliation.php
  - scripts/acceptance/tests/support-legal-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/wiki-reconciliation-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-acceptance-final-closure.md
  - docs/agents/tasks/active/OTERYN-20260727-announcements-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-announcements-acceptance.md
modules:
  - Support / Legal
  - Editorial Media
  - Wiki public
  - Wiki administration
  - Testing / Acceptance E2E
  - Agent governance
dependencies:
  - PR #259 merged as d08062c653a137e1359b5626fda635b170704cd8
  - Issue #240
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T21:50:00Z
head: f0ca6a7ce7366c7152477d2b9669cd646a917cb5
branch: test/OTERYN-20260727-portal-acceptance-final-closure
pr: 260
status: validating
context_routes:
  - agent-governance
  - testing
  - web-cms
  - admin-rbac
  - security
  - accessibility
owned_paths:
  - .github/workflows/portal-acceptance-contract.yml
  - .github/workflows/support-legal-acceptance.yml
  - .github/workflows/editorial-media-acceptance.yml
  - .github/workflows/wiki-reconciliation-acceptance.yml
  - scripts/acceptance/playwright.support-legal.config.mjs
  - scripts/acceptance/playwright.editorial-media.config.mjs
  - scripts/acceptance/playwright.wiki-reconciliation.config.mjs
  - scripts/acceptance/seed-browser-support-legal.php
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/seed-browser-wiki-reconciliation.php
  - scripts/acceptance/tests/support-legal-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/wiki-reconciliation-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-acceptance-final-closure.md
  - docs/agents/tasks/active/OTERYN-20260727-announcements-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-announcements-acceptance.md
proven:
  - PR #259 merged as d08062c653a137e1359b5626fda635b170704cd8 after every exact-head repository workflow passed; Announcements is covered.
  - Support Legal Acceptance run 30304815994 passed the complete zero-retry route/public/localization and exact-MFA-RBAC administrator matrix on 229dfa54ff353412b8ad96be4837dab5f30a3b82 across Chromium desktop/tablet/mobile and public Firefox/WebKit.
  - Editorial Media Acceptance run 30307178424 passed the complete zero-retry validation, safe upload, private content/thumbnail, reference-lock, deletion, exact-MFA-RBAC and audit matrix on 9df5d2d7a881290c382f4155884094e8b3e1d04b across Chromium desktop/tablet/mobile.
  - Wiki Reconciliation Acceptance run 30307604327 passed the complete zero-retry public route/search/error/recovery/localization and administrator validation/preview/conflict/review/publication/revision/archive/audit matrix on ee0324a5b086f7bfeb4bb662974780cae3273163 across Chromium desktop/tablet/mobile, Firefox and WebKit.
  - Commit b4b1533e00081821c4d9bac3cb79c453f838355e promoted Support/Legal, Editorial Media, Wiki public and Wiki administration to covered with exact evidence markers and empty gaps.
  - The coverage matrix records all four final surfaces as covered and makes strict delivered-surface closure required by Portal Acceptance Contract.
  - The permanent Portal Acceptance Contract now executes test:coverage-contract:strict; both temporary promotion workflows were removed.
  - No application schema, session, Canary compatibility or production configuration changed; fixtures operate only on isolated acceptance data.
derived:
  - The declared delivered-surface ledger has no remaining planned or partial records after evidence-backed promotion.
  - Lower-layer storage, integrity and domain invariants remain at deterministic feature/integration layers; Playwright proves only composed public and operator behavior.
  - Repository and controlled production-like evidence cannot establish final production correctness or create a PRODUCTION_PROVEN fact.
unknown:
  - Exact-head conclusions for the final strict-ledger checkpoint commit and all repository workflows.
  - Real production behavior remains unverified under Issue #91.
conflicts: []
first_failure:
  marker: wiki-reconciliation-cross-project-reset
  evidence: initial Wiki reconciliation runs exposed ambiguous public locators, stale Polish translation after English editing and self-referencing revision cleanup; exact locators, synchronized bilingual revision input and dependency-safe fixture reset corrected each root cause without retries or weakened states.
rejected_hypotheses:
  - Existing feature tests alone close Support/Legal or Editorial Media composed browser contracts.
  - Existing Wiki happy paths alone justify both partial records without explicit state reconciliation.
  - Browser retries, forced clicks or reduced browser/viewport scope are acceptable fixes for deterministic failures.
  - Repository or staging evidence establishes production correctness.
changed_paths:
  - .github/workflows/portal-acceptance-contract.yml
  - .github/workflows/support-legal-acceptance.yml
  - .github/workflows/editorial-media-acceptance.yml
  - .github/workflows/wiki-reconciliation-acceptance.yml
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-announcements-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-acceptance-final-closure.md
  - docs/agents/tasks/archive/OTERYN-20260727-announcements-acceptance.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - scripts/acceptance/playwright.editorial-media.config.mjs
  - scripts/acceptance/playwright.support-legal.config.mjs
  - scripts/acceptance/playwright.wiki-reconciliation.config.mjs
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/seed-browser-support-legal.php
  - scripts/acceptance/seed-browser-wiki-reconciliation.php
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/support-legal-acceptance.spec.mjs
  - scripts/acceptance/tests/wiki-reconciliation-acceptance.spec.mjs
validation:
  - command: Support Legal Acceptance run 30304815994 on 229dfa54ff353412b8ad96be4837dab5f30a3b82
    result: PASS
    evidence: complete five-project route/public/localization and exact-MFA-RBAC administrator matrix passed with zero retries
  - command: Editorial Media Acceptance run 30307178424 on 9df5d2d7a881290c382f4155884094e8b3e1d04b
    result: PASS
    evidence: complete Chromium desktop/tablet/mobile lifecycle passed with zero retries
  - command: Wiki Reconciliation Acceptance run 30307604327 on ee0324a5b086f7bfeb4bb662974780cae3273163
    result: PASS
    evidence: complete five-project public and administrator reconciliation lifecycle passed with zero retries
  - command: final strict Portal Acceptance Contract and repository workflow set after checkpoint
    result: NOT_RUN
    evidence: triggered by this checkpoint commit and required before readiness, merge and Issue #240 closure
blockers:
  - none
next_action: Require every module and repository workflow on the checkpoint head; fix the first failure, then mark PR ready, squash-merge and close Issue #240.
```

## Notes

Keep module gates independent. `covered` means complete against the declared repository acceptance contract and never implies `PRODUCTION_PROVEN`.
