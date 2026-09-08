---
task_id: OTERYN-20260908-portal-visual-polish
governing_issue: 1333
required_reads:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
search_first:
  - current live Issue #1333, PR #1334, task branch, protected main and exact failed-check evidence
optional_reads: []
---

# OTERYN-20260908-portal-visual-polish

## Goal

Governing GitHub Issue: #1333. Finish the owner-rejected visual details of the integrated Platform redesign, preserve application/security contracts, prove real page families, and deliver through protected main.

Historical admission main was `519f610b9e02641283cc4322c80b74a9462edb11`. The previous #1298 task is terminal; this follow-up preserves its implementation rather than restarting it. Continue the existing PR #1334 and branch `fix/20260908-portal-visual-polish` only.

## Acceptance criteria

- [ ] Replace oversized or blurry thumbnail presentation, background seams and unfinished shared visual hierarchy.
- [ ] Finish representative public, account/security, commerce/support/error and administrator families in EN/PL and at phone/tablet/desktop/wide sizes.
- [ ] Record asset/layout measurements and compare real before/after screenshots, including sparse and populated content.
- [ ] Preserve all routes, security/domain/payment behavior, negative assertions and truthful application states.
- [ ] Pass relevant focused and exact-candidate required checks and complete protected integration/closeout.

## Ownership

```yaml
owned_paths:
  - app/Admin/AdminAuthorization.php
  - public/css/portal-*.css
  - public/css/home-production.css
  - public/images/oteryn-*
  - public/js/portal-navigation.js
  - resources/views/**
  - lang/**
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - tests/Feature/PortalVisualPolishTest.php
  - docs/testing/PORTAL_VISUAL_POLISH_2026-09-08.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/tasks/active/OTERYN-20260908-portal-visual-polish.md
modules:
  - web-cms
  - admin-rbac
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

The navigation enhancement is included solely to repair this task's existing mobile keyboard-focus failure. No authentication, role grant, permission, payment or server boundary is expanded.

## Execution routing

```yaml
execution_target: isolated_workspace
runner_class: github_hosted
equivalent_ci: .github/workflows/acceptance-validation.yml
remote_desktop: denied
remote_desktop_reason: null
requested_host_actions: []
requested_remote_desktop_tools: []
requested_remote_desktop_calls: []
github_preflight:
  verified_at: 2026-09-08T08:32:00Z
  repository: Oteryn/Oteryn-Platform
  default_branch_sha: c2e4ff4d035b50a96c88cf99adfb32d60040e317
  governing_issue: 1333
  pull_request: 1334
  task_head_sha: b0d898ebe29cb571bd5f8c91543cfb0380ef3f75
parallel_execution:
  effort: high
  lane_strategy: single_agent
  decision_basis: Shared visual tokens, templates, navigation and screenshot acceptance require one coherent writer; existing hosted browser jobs provide independent validation.
  lanes:
    - id: portal-polish
      owned_paths: [public/css, public/images, public/js/portal-navigation.js, resources/views, lang, scripts/acceptance/tests/portal-polish-quality.spec.mjs, scripts/acceptance/tests/portal-visual-review.spec.mjs, tests/Feature/PortalVisualPolishTest.php]
      dependencies: []
      branch: refs/heads/fix/20260908-portal-visual-polish
      worktree: isolated-workspace-with-exact-source-and-hosted-runtime-validation
      shared_leases: []
  integration_order: [portal-polish]
```

## Next-agent invocation

`PORTAL-POLISH` resolves this existing live ownership through the canonical Portal prompt. It does not create a new programme, Issue, branch, PR or parallel writer. After terminal closeout the alias is status-only.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T08:32:00Z
head: b0d898ebe29cb571bd5f8c91543cfb0380ef3f75
branch: fix/20260908-portal-visual-polish
pr: 1334
status: validating
context_routes:
  - web-cms
  - admin-rbac
  - portal-completion
owned_paths:
  - app/Admin/AdminAuthorization.php
  - public/css/portal-*.css
  - public/css/home-production.css
  - public/images/oteryn-*
  - public/js/portal-navigation.js
  - resources/views/**
  - lang/**
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - tests/Feature/PortalVisualPolishTest.php
  - docs/testing/PORTAL_VISUAL_POLISH_2026-09-08.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/tasks/active/OTERYN-20260908-portal-visual-polish.md
proven:
  - Issue 1333 and draft PR 1334 remain open; the head field records the continuation admission head before this material repair commit.
  - Protected main advanced from 0cd7e76c45736e81cc50db24c6af79ba4a819631 to c2e4ff4d035b50a96c88cf99adfb32d60040e317 through PR 1338; its delta is confined to Synology build inputs, their test and the separate task packet.
  - Implementation 2acc56708461c5f4dce853d837e04f1e56840a8b passed CI 34197970711, CodeQL 34197970792, Edge Security 34197970823, DB Outage 34197970747, Game Auth Concurrency 34197970750, Phase 7 34197970713, Content Scale 34197970748, Events 34197970758, Wiki Reconciliation 34197970781 and staging-image build 34197970707.
  - Acceptance run 34197970780 failed mobile Tab focus; its failure snapshot shows the disclosure closed. Its manifest contains 205 real Laravel screenshot records with no reported missing assets, unlabelled controls or document overflow; this does not make the failed run a PASS.
  - A deterministic Chromium component reproduction opens the visible native mobile disclosure during resize before the queued media-query event; the old unconditional close handler closes it and loses first-link focus, while the hidden-disclosures-only fix preserves both. Escape focus restoration and hiding the mobile disclosure at desktop width also pass.
  - The administrator links already existed at runtime; the repository navigation scanner recognizes literal route calls rather than route variables. Three explicit match arms make audit, homepage templates and roles discoverable without changing authorization or exempting routes.
  - Baseline artifact 10043606384 from run 34194603808 has verified SHA256 b7096f699bc388d7514c73f07d2a40a404ff97f600565ced9e3be4a22a69e1c0 and tracked-source archive identity 4b71868c969766416c57246bf74186168375d125.
  - Implementation artifact 10044731213 from run 34197970780 has verified SHA256 86c41460dc9372074ccb2ddd09bb1611d7c41a02f2573b79a79285106c0b48dd.
derived:
  - The queued responsive dismissal explains a reproducible version of the reported focus failure; the unchanged real keyboard acceptance path must independently confirm the repair on the new exact head.
  - The branch reconciliation must preserve all existing Portal work and import main's unrelated Synology files byte-for-byte; no separate Synology repair is authorized here.
unknown:
  - Hosted exact-head acceptance, navigation contract and required checks for the commit containing this checkpoint.
  - Final whole-family visual assessment, before/after resource comparison and protected integration result.
conflicts: []
first_failure:
  marker: mobile-navigation-focus
  evidence: Historical run 34197970780 at 2acc56708461c5f4dce853d837e04f1e56840a8b; repaired candidate awaits hosted confirmation.
rejected_hypotheses:
  - Missing administrator navigation requires removing route coverage or changing permissions; existing links only needed explicit literal references for the scanner.
  - Offline component execution or prior-head screenshot records alone prove final Laravel acceptance.
changed_paths:
  - app/Admin/AdminAuthorization.php
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/tasks/active/OTERYN-20260908-portal-visual-polish.md
  - lang/en/portal_art.php
  - lang/pl/portal_art.php
  - public/css/home-production.css
  - public/css/portal-admin.css
  - public/css/portal-art-direction.css
  - public/js/portal-navigation.js
  - resources/views/admin/dashboard.blade.php
  - resources/views/admin/partials/navigation.blade.php
  - resources/views/events/show.blade.php
  - resources/views/home.blade.php
  - resources/views/news/index.blade.php
  - resources/views/news/show.blade.php
  - resources/views/wiki/index.blade.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - tests/Feature/PortalVisualPolishTest.php
validation:
  - command: Chromium native-disclosure resize ordering reproduction before and after the prepared JavaScript repair
    result: PASS
    evidence: Old handler closes the menu and loses first-link focus; repaired handler preserves them and passes Escape and desktop-hide checks. Component-only evidence, not Laravel E2E.
  - command: node --check on portal-navigation.js and portal-polish-quality.spec.mjs; PHP syntax checks on changed Blade PHP blocks; literal route reference check
    result: PASS
    evidence: Prepared files parse and all three previously missing administrator routes are discoverable; no scanner or authorization rule changed.
  - command: local node --test scripts/acceptance/coverage/test-route-view-navigation-*.mjs
    result: BLOCKED
    evidence: Both suites stop before assertions because php artisan route:list cannot load missing vendor/autoload.php in the recovered tracked-only workspace; hosted runtime validation is required.
  - command: hosted acceptance and required exact-head checks for this repair
    result: NOT_RUN
    evidence: Must run after publication; previous implementation-head results are not promoted to the new candidate.
blockers:
  - none
next_action: Verify publication of the reconciled existing branch, inspect fresh exact-head hosted checks and screenshots, repair any task-owned failure, then finish measured visual acceptance and protected merge-queue closeout.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: visual acceptance, exact-candidate required checks and protected integration are not terminal
source_branch_evidence: draft PR #1334 remains open
```

## Notes

Local GitHub DNS and a complete local Laravel dependency installation are unavailable. GitHub connector writes plus repository-native hosted validation remain available; no Remote Desktop or machine-policy bypass is used. Offline browser probes are explicitly component evidence.

The temporary tracked-source export was removed in the earlier implementation and remains removed. No production deployment, live-data operation, payment/auth mutation or protection change is part of this task. The bounded administrator granted-permission projection still preserves route middleware authority and its revocation regression.
