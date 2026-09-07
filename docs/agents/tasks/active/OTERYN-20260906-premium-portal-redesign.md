---
task_id: OTERYN-20260906-premium-portal-redesign
governing_issue: 1297
required_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - resources/views
  - public/css
  - scripts/acceptance
optional_reads: []
---

# Complete Oteryn player portal redesign

## Goal and boundary

Issue #1297; deliver the owner's complete player-facing Laravel/Blade redesign through existing PR #1298 for human visual review. The expanded brief supersedes the earlier conservative presentation. No PR merge, auto-merge, deployment, production access or backend/security contract change.

## Acceptance criteria

- [x] Preserve the full redesign in one owned PR; publish the design system, page families, original artwork and responsive repairs.
- [x] Map 98 application HTML/navigation entries and additional OAuth consent in the review ledger.
- [x] Remove the temporary tracked-source recovery test; preserve upstream governance and other work.
- [ ] Pass repaired exact-head feature/static/browser and required checks.
- [ ] Complete actual EN/PL guest/account and edge-state review across phone/tablet/desktop/wide.
- [ ] Publish matching final evidence and leave PR ready for human review, without merge or deployment.

## Ownership

One writer owns public/player presentation, original artwork, portal translations, focused presentation/browser test alignment and this task's evidence. Controllers, routes, domain/authentication/payment behavior, databases, admin UI, workflows, protections and deployment are excluded. Independent #1294 audit, governance work and inactive catalog consumer #338 remain untouched. Never suppress a failing test or weaken its security invariant to obtain a green result.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T12:49:13Z
head: 8f529c052f2972c3128651cef182976a931259ec
branch: feat/20260906-premium-portal-redesign
pr: 1298
status: validating
context_routes:
  - web-cms
  - auth-identity
  - public-game-data
  - testing
  - agent-governance
owned_paths:
  - resources/views/**
  - public/css/portal-system.css
  - public/css/portal-pages.css
  - public/css/home-production.css
  - public/js/portal-navigation.js
  - public/images/oteryn-world.svg
  - public/images/oteryn-citadel.webp
  - lang/en/portal.php
  - lang/pl/portal.php
  - lang/pl.json
  - scripts/acceptance/tests/**
  - tests/Feature/HomeTest.php
  - tests/Feature/HomePreviewTest.php
  - tests/Feature/PublicPortalRedesignTest.php
  - tests/Feature/Accounts/AccountOverviewTest.php
  - tests/Feature/Operations/SecurityHeadersTest.php
  - tests/Feature/PublicGameData/ServerRuntimeAvailabilityTest.php
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
  - docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md
proven:
  - CI run 34123528221 at 8f529c05 passed formatting, static analysis, runtime tests and platform-gate after the response generic fix.
  - Actual phone-account-pl screenshot in the 015bfbdf manifest exposes English setup and character-state messages inside the Polish account shell; the presentation-only repair translates those messages without changing the read model or permissions.
  - Published 015bfbdf synchronizes protected main 721d560c18ef8a6eefbf4ccd685d8302985bbd3d and preserves its policy 3.0.0 binding and independent work.
  - CI run 34121352649 passed formatting and static analysis; PHPUnit reported 610 tests, eight failures, four skipped and zero errors.
  - The eight failures are presentation assertions across five files; revised assertions retain CSP, ownership, hidden identifiers and truthful runtime data requirements.
  - Acceptance run 34121352815 produced 134 real Laravel fixture screenshots with matching 015bfbdf manifest; all captured records have no missing images, unlabelled controls or horizontal document overflow.
  - Actual contact-sheet inspection covered home, identity, account, knowledge, rankings and downloads on desktop and phone; this is not complete final visual acceptance.
  - Local syntax checks pass for all five revised PHP files; new per-world metric expectations match all four captured failing public responses.
derived:
  - The new visual test's read-only ready-state fixture does not provision the underlying Canary account needed for character creation; its worker restart also collides with repeated Bazaar fixture identity.
unknown:
  - Exact-head results after publication of the Polish account-message repair and five new localization cases remain pending.
  - Browser paths following the interrupted character creation and final production readiness are not proven.
conflicts: []
first_failure:
  marker: EXACT_HEAD_VALIDATION_AND_TOOL_WRITE_BLOCK
  evidence: CI 34121352649 artifact 10018360866; acceptance 34121352815 artifact 10018380839. Browser smoke has one failed case, one setup error and one skipped case.
rejected_hypotheses:
  - The user's Markdown or maintenance freeze prevents preparing the redesign.
changed_paths:
  - lang/pl.json
  - resources/views/identity/account/overview.blade.php
  - tests/Feature/Accounts/AccountOverviewTest.php
  - tests/Feature/HomePreviewTest.php
  - tests/Feature/Operations/SecurityHeadersTest.php
  - tests/Feature/PublicGameData/ServerRuntimeAvailabilityTest.php
  - tests/Feature/PublicPortalRedesignTest.php
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
validation:
  - command: php -l on the five changed PHP test files
    result: PASS
    evidence: Local syntax only; uploaded blob identities match these bytes.
  - command: compare labelled per-world metrics with four captured 015bfbdf Laravel responses
    result: PASS
    evidence: Exact runtime status, player counts, capacity and PvP labels checked per world; unknown/unavailable counts remain em dashes.
  - command: repaired exact-head CI and acceptance
    result: NOT_RUN
    evidence: Pending publication; unchanged blocked browser test is still expected to fail, not skipped or suppressed.
blockers:
  - Tool safeguards rejected the visual-review fixture repair write; published portal-visual-review.spec.mjs remains at blob d285807369ad40c73ca5796c451117615aecd2b7.
  - Earlier tool safeguards rejected proposed payment-foundation and support-legal acceptance test repairs; both retain their original published bytes and enabled execution.
next_action: Verify the published localization candidate with hosted CI and real renders; attach the final matching results and remaining browser-tool blocker without bypassing it or claiming completion.
```

## Evidence and execution

`docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md` preserves route coverage and design scope. The latest PR attestation supersedes historical validation observations. A checkpoint predecessor SHA is not final-head certification.

Use only the isolated sandbox and existing GitHub-hosted application runtime. Local outbound DNS, Composer dependencies and PHP 8.5 are unavailable. No workstation, Synology, staging/production, live identity or payment operation is authorized. No unsupported subagent or background execution is claimed.

Original citadel artwork is decorative, not a gameplay screenshot. Only actual fixture renders are application evidence. No generated credentials, session exports or authentication traces are published.

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Issue 1297 owner retains PR 1298 for human visual review; revisit disposition after that review and separate integration authorization
source_branch_evidence: Issue 1297; PR 1298; complete owner brief
```
