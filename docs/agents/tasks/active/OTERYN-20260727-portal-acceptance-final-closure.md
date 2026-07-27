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

- [x] Support/Legal proves all delivered public routes, EN/PL isolation, approved links, missing/unpublished/legal-version states and exact MFA/RBAC administrator editing.
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
updated_at: 2026-07-27T21:02:26Z
head: 229dfa54ff353412b8ad96be4837dab5f30a3b82
branch: test/OTERYN-20260727-portal-acceptance-final-closure
pr: 260
status: implementing
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
  - The remaining pre-promotion machine-ledger scope is Support/Legal planned, Editorial Media planned, Wiki public partial and Wiki admin partial.
  - No other open pull request or active task owns the final Issue #240 paths.
  - Support/Legal lower-layer feature coverage remains authoritative for route behavior, legal-version immutability, approved-link policy and bounded audit persistence.
  - The isolated Support/Legal package now consists of a dedicated zero-retry workflow, five-project Playwright config, bounded fixture and one stable browser specification.
  - Support Legal Acceptance run 30304815994 passed on exact implementation SHA 229dfa54ff353412b8ad96be4837dab5f30a3b82.
  - The passing browser package proves all eight typed legacy and localized routes in missing, unpublished and published states, escaped output, EN/PL isolation, legal-version display, approved links and report-a-bug guidance-only behavior.
  - The passing administrator package proves guest redirect, confirmed-MFA enforcement, exact permission denial, legal validation/publication, Polish translation and redacted audit visibility on Chromium desktop, tablet and mobile.
  - The public package also passed bounded Firefox and WebKit desktop projects; every Support/Legal project used zero retries.
  - Portal coverage classification, CI, Agent Governance, Synology Production Target Preflight, Game Auth Ticket Concurrency, Edge Security Emulation and Platform DB Outage Validation passed on the same implementation SHA.
derived:
  - Support/Legal has sufficient durable browser evidence for ledger promotion, but the record remains fail-closed until the manifest and matrix update retrigger exact-head gates.
  - Lower-layer codec, storage and domain tests were not duplicated in Playwright; browser evidence remains focused on composed operator and public behavior.
  - This work remains repository/staging evidence only and cannot establish production correctness.
unknown:
  - Exact browser gaps that remain after Wiki state-by-state reconciliation.
  - Final conclusions of the still-running repository workflows on implementation SHA 229dfa54ff353412b8ad96be4837dab5f30a3b82.
  - Exact-head workflow results after Support/Legal ledger promotion.
conflicts: []
first_failure:
  marker: none
  evidence: the first isolated Support Legal Acceptance run passed its complete zero-retry matrix
rejected_hypotheses:
  - Existing feature tests alone close Support/Legal or Editorial Media composed browser contracts.
  - Existing Wiki happy paths alone justify changing both partial records to covered without explicit state reconciliation.
  - Support/Legal should be promoted before its isolated exact-head browser gate passes.
changed_paths:
  - .github/workflows/support-legal-acceptance.yml
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-acceptance-final-closure.md
  - docs/agents/tasks/active/OTERYN-20260727-announcements-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-announcements-acceptance.md
  - scripts/acceptance/playwright.support-legal.config.mjs
  - scripts/acceptance/seed-browser-support-legal.php
  - scripts/acceptance/tests/support-legal-acceptance.spec.mjs
validation:
  - command: branch test/OTERYN-20260727-portal-acceptance-final-closure created from d08062c653a137e1359b5626fda635b170704cd8
    result: PASS
    evidence: branch creation succeeded from the merged PR #259 commit
  - command: draft PR #260 opened to main
    result: PASS
    evidence: GitHub reports base d08062c653a137e1359b5626fda635b170704cd8
  - command: Support Legal Acceptance run 30304815994 on 229dfa54ff353412b8ad96be4837dab5f30a3b82
    result: PASS
    evidence: complete route/public/localization and exact-MFA-RBAC administrator matrix passed on Chromium desktop/tablet/mobile plus bounded public Firefox/WebKit with zero retries
  - command: Portal coverage classification job in run 30304816114 on 229dfa54ff353412b8ad96be4837dab5f30a3b82
    result: PASS
    evidence: the existing fail-closed manifest remained valid before Support/Legal promotion
  - command: remaining exact-head repository workflows on 229dfa54ff353412b8ad96be4837dab5f30a3b82
    result: IN_PROGRESS
    evidence: no failure invalidating Support/Legal implementation was observed when this checkpoint was written
blockers:
  - none
next_action: Promote the Support/Legal manifest record and coverage-matrix contract with the passing exact markers, then require the retriggered exact-head Support Legal and Portal Acceptance gates before starting Editorial Media.
```

## Notes

Keep module gates independent and promote ledger records only from durable exact markers after the corresponding zero-retry browser proof passes.
