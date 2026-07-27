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

Close the remaining Issue #240 portal acceptance records for Support/Legal, Editorial Media and Wiki public/admin with deterministic zero-retry browser evidence, strict ledger reconciliation and no production claim.

## Acceptance criteria

- [ ] Support/Legal proves all delivered public routes, EN/PL isolation, approved links, missing/unpublished/legal-version states and exact MFA/RBAC administrator editing.
- [ ] Editorial Media proves exact MFA/RBAC, upload validation, private preview/content, deletion and referenced-media protection.
- [ ] Wiki public/admin reconciles every required state with stable exact markers and fills genuine browser gaps without weakening existing tests.
- [ ] Each module has an isolated workflow/config/fixture boundary and zero retries.
- [ ] The canonical ledger moves the four remaining records to `covered` only after exact-head module and repository checks pass.
- [ ] Issue #240 is closed only after the strict ledger contains no open delivered-surface gaps; no `PRODUCTION_PROVEN` claim is made.

## Ownership

```yaml
owned_paths:
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
updated_at: 2026-07-27T20:40:05Z
head: d08062c653a137e1359b5626fda635b170704cd8
branch: test/OTERYN-20260727-portal-acceptance-final-closure
pr: none
status: investigating
context_routes:
  - agent-governance
  - testing
  - web-cms
  - admin-rbac
  - security
  - accessibility
owned_paths:
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
  - The remaining machine-ledger scope is Support/Legal planned, Editorial Media planned, Wiki public partial and Wiki admin partial.
  - Support/Legal has lower-layer feature coverage but lacks route-complete composed browser evidence.
  - Editorial Media has lower-layer authorization, normalization, integrity and storage tests but lacks operator-visible browser lifecycle evidence.
  - Wiki already has published bilingual read/search and trusted-editor create/preview/publish browser paths; the ledger requires explicit reconciliation of missing states and stable markers.
  - No open pull request or active task owns the final Issue #240 paths.
derived:
  - One final task may close all four records while preserving independent module workflows and fail-closed ledger promotion.
  - Lower-layer codec, storage and domain tests should not be duplicated in browser tests; browser evidence should prove composed operator and public behavior.
  - This work remains repository/staging evidence only and cannot establish production correctness.
unknown:
  - Exact browser gaps that remain after Wiki state-by-state reconciliation.
  - Exact-head module and repository workflow results for the final closure branch.
conflicts: []
first_failure:
  marker: none
  evidence: no implementation validation has run on the final closure branch
rejected_hypotheses:
  - Existing feature tests alone close Support/Legal or Editorial Media composed browser contracts.
  - Existing Wiki happy paths alone justify changing both partial records to covered without explicit state reconciliation.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260727-portal-acceptance-final-closure.md
validation:
  - command: branch test/OTERYN-20260727-portal-acceptance-final-closure created from d08062c653a137e1359b5626fda635b170704cd8
    result: PASS
    evidence: branch creation succeeded from the merged PR #259 commit
  - command: module implementation and exact-head workflows
    result: NOT_RUN
    evidence: final closure implementation has not started
blockers:
  - none
next_action: Implement the isolated Support/Legal browser package first, then validate its exact-head module gate before ledger promotion.
```

## Notes

Keep module gates independent and promote ledger records only from durable exact markers after the corresponding zero-retry browser proof passes.
