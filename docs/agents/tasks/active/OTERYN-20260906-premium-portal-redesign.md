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

## Goal

Issue #1297; deliver the owner's complete player-facing Laravel/Blade redesign through the existing PR #1298 for human visual review. The expanded owner brief supersedes the earlier conservative presentation. No PR merge, auto-merge, deployment, production access or backend/security contract change.

## Acceptance criteria

- [x] Preserve the full redesign on the single owned branch and PR.
- [x] Implement the public design system, grouped navigation, homepage, editorial/world/community/knowledge page families and account/identity presentation.
- [x] Map application HTML/navigation routes and the additional OAuth consent view in the review ledger.
- [x] Remove the temporary tracked-source recovery test from the repaired candidate.
- [ ] Verify guest/authenticated EN/PL journeys, populated and unavailable states on the repaired candidate.
- [ ] Inspect actual phone/tablet/desktop/wide renders and resolve visual findings.
- [ ] Pass relevant feature/static/browser checks and repository-required exact-head validation.
- [ ] Review the complete final diff and publish matching exact-head evidence.
- [ ] Leave PR #1298 ready for human visual review without merging or deploying.

## Ownership

The sole writer owns public/player Blade presentation, the three portal stylesheets, portal navigation JavaScript, original Oteryn artwork, EN/PL portal translations, focused presentation/browser tests, this task and its review ledger. Admin presentation, controllers, routes, domain/authentication/payment behavior, databases, workflow implementation, protections and deployment configuration are excluded. Existing assertions and the runtime diagnostic guard remain intact. #1294 audit work, #1302/#1309 governance work and inactive catalog consumer #338 are independent.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T11:58:26Z
head: 4910e9dd4d54139847e317a5521688f556831942
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
  - scripts/acceptance/tests/**
  - tests/Feature/HomeTest.php
  - tests/Feature/PublicPortalRedesignTest.php
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
  - docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md
proven:
  - Resume readback found Issue 1297 open and PR 1298 open/draft at 4910e9dd4d54139847e317a5521688f556831942.
  - Protected main 2c3934fece8e072f23ea33ad82f4b4399fc3b45e adopts META policy 3.0.0 at 5ed3f14400af450b5875c091e443da70f2d67ab9; root, bootstrap, applicable agent-document instructions and task contract were reconciled before mutation.
  - Current main product subtrees match the fixed main 3557085 source archive; newer root policies, documentation, tools and workflows are preserved unchanged during branch synchronization.
  - All 22 recovery-package changes were SHA-256 verified against the fixed 4910e9d source and output manifest before editing.
  - Repaired source removes the temporary tracked-source export; no runtime environment, credentials, database, sessions or key material is included.
derived:
  - The recovered complete portal is a candidate requiring repaired exact-head validation, not a completed deployment.
unknown:
  - Final repaired-head CI, browser results and visual acceptance remain unverified until the new publication is tested.
conflicts: []
first_failure:
  marker: INITIAL_FULL_CANDIDATE_REPAIR
  evidence: Historical run 34104987932 passed 14 smoke and 27 portability cases but found 390px populated Polish payment overflow; its 148 renders do not certify later repairs.
rejected_hypotheses:
  - The owner Markdown was unreadable; the complete attached brief is available.
changed_paths:
  - resources/views/**
  - public/css/**
  - public/images/oteryn-citadel.webp
  - lang/en/portal.php
  - lang/pl/portal.php
  - scripts/acceptance/tests/**
  - tests/Feature/PublicPortalRedesignTest.php
  - docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md
validation:
  - command: PHP and JavaScript syntax checks for 15 changed files
    result: PASS
    evidence: Local PHP 8.4 and Node syntax only; Laravel dependencies and required PHP 8.5 runtime are not available locally.
  - command: python tools/agents/checkpoint.py task-path --require-checkpoint
    result: PASS
    evidence: Recovery checkpoint passed the available structural validator; new candidate requires hosted governance checks.
  - command: repaired exact-head Laravel and browser acceptance
    result: NOT_RUN
    evidence: Use existing isolated GitHub Actions runtime after coherent publication; no local runtime or final visual pass is claimed.
blockers: []
next_action: Publish and verify the coherent repaired candidate, inspect its exact-head CI and actual renders, then resolve findings before marking PR 1298 ready for human review.
```

## Evidence and execution

`docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md` preserves the route ledger and historical observations. Read the final PR evidence and rendered manifest for the tested head; a checkpoint's predecessor SHA is not a final readiness claim.

The repaired candidate restores native download-table semantics and payment scrolling, preserves locale in player-tool navigation, shares the redesigned homepage with the registered compact variant, declares exact expected negative HTTP responses without weakening assertions, and expands populated character/support/MFA coverage. The original decorative citadel is not a gameplay screenshot. Only actual Laravel fixture captures are visual application evidence; secret-bearing enrollment/recovery values are masked.

Use the task's isolated sandbox and existing GitHub-hosted acceptance jobs. Local outbound DNS and Composer are unavailable. No workstation, Synology, protected environment, staging, production, payment execution or live identity operation is authorized. One writer owns the branch; no unsupported subagent/model setting or asynchronous continuation is claimed.

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Issue 1297 owner retains PR 1298 for human visual review; revisit disposition after that review and separate integration authorization
source_branch_evidence: Issue 1297; PR 1298; complete owner brief
```
