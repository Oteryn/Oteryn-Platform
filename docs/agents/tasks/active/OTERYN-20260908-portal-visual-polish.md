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

Historical admission main was `519f610b9e02641283cc4322c80b74a9462edb11`. The previous #1298 task is terminal; this follow-up preserves its implementation rather than restarting it. This task already owns draft PR #1334 and branch `fix/20260908-portal-visual-polish`; a continuation agent must resume them when live rather than create duplicate ownership.

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
  verified_at: 2026-09-08T08:13:19Z
  repository: Oteryn/Oteryn-Platform
  default_branch_sha: 0cd7e76c45736e81cc50db24c6af79ba4a819631
  governing_issue: 1333
  pull_request: 1334
  task_head_sha: 2533faa68edab61916afc20e1c3c3de07b246f98
parallel_execution:
  effort: high
  lane_strategy: single_agent
  decision_basis: Shared visual tokens, templates, navigation and screenshot acceptance require one coherent writer; existing hosted browser jobs provide independent validation.
  lanes:
    - id: portal-polish
      owned_paths: [public/css, public/images, resources/views, lang, scripts/acceptance/tests/portal-polish-quality.spec.mjs, scripts/acceptance/tests/portal-visual-review.spec.mjs, tests/Feature/PortalVisualPolishTest.php]
      dependencies: []
      branch: refs/heads/fix/20260908-portal-visual-polish
      worktree: resolve-or-create-isolated-workspace-from-live-task-branch
      shared_leases: []
  integration_order: [portal-polish]
```

## Next-agent invocation

Use the repository alias:

```text
PORTAL-POLISH
```

The alias is registered in `docs/agents/SHORT_PROGRAM_INVOCATIONS.md` and routes through the existing canonical Portal prompt. It is continuation-only: resolve live state first, resume Issue #1333 / PR #1334 / this task when valid, and never create a second Portal programme, Issue, branch, PR or writer.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T08:13:19Z
head: 2533faa68edab61916afc20e1c3c3de07b246f98
branch: fix/20260908-portal-visual-polish
pr: 1334
status: ready
context_routes:
  - web-cms
  - admin-rbac
  - portal-completion
owned_paths:
  - app/Admin/AdminAuthorization.php
  - public/css/portal-*.css
  - public/css/home-production.css
  - public/images/oteryn-*
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
  - GitHub Issue #1333 is open and remains the governing lifecycle authority.
  - Draft PR #1334 is open on branch fix/20260908-portal-visual-polish; its live head after the prompt and alias handoff commits is 2533faa68edab61916afc20e1c3c3de07b246f98.
  - Protected main was 0cd7e76c45736e81cc50db24c6af79ba4a819631 at the 2026-09-08T08:13:19Z handoff preflight; the task branch and main have diverged and the observed main-side delta is Synology control-plane/task work rather than portal UI paths.
  - Exact implementation candidate 2acc56708461c5f4dce853d837e04f1e56840a8b passed CI run 34197970711, CodeQL 34197970792, Edge Security Emulation 34197970823, Platform DB Outage Validation 34197970747, Game Auth Ticket Concurrency 34197970750, Phase 7 Production-Like Validation 34197970713, Content Scale Acceptance 34197970748, Events Acceptance 34197970758, Wiki Reconciliation Acceptance 34197970781 and Build Synology Staging Images 34197970707.
  - Exact implementation candidate 2acc56708461c5f4dce853d837e04f1e56840a8b failed Acceptance E2E and Visual UX run 34197970780 in the Chromium smoke portal visual-review test because opening the mobile navigation and pressing Tab did not focus the first navigation link.
  - Exact implementation candidate 2acc56708461c5f4dce853d837e04f1e56840a8b failed Portal Acceptance Contract run 34197970723 only on missing global/contextual navigation reference or direct-entry rationale for admin.audit.index, admin.homepage-templates.index and admin.roles.index; the account-lifecycle job in that workflow passed.
  - The canonical Portal prompt now defines the PORTAL-POLISH task-pinned continuation route, and the short-invocation registry exposes that alias without creating a second Portal programme.
derived:
  - The continuation agent must refresh/reconcile the existing branch against live protected main before new product edits and must preserve all existing PR #1334 work.
  - The first task-owned failures to repair are the mobile-navigation keyboard focus regression and the three exact administrator navigation/direct-entry coverage gaps.
  - No new Portal programme, governing Issue, branch or PR is needed while #1333/#1334 ownership remains live.
unknown:
  - Protected main SHA and branch divergence at the next invocation.
  - Whether the two recorded task-owned failure families persist after refreshing from live main.
  - Final corrected rendered quality, browser acceptance and measured resource/layout delta.
conflicts: []
first_failure:
  marker: mobile-navigation-focus
  evidence: Acceptance E2E and Visual UX run 34197970780 at exact 2acc56708461c5f4dce853d837e04f1e56840a8b; portal-visual-review.spec.mjs expected the first .mobile-nav nav link to be focused after Tab, but it remained inactive.
rejected_hypotheses:
  - Agent Governance run 34197970774 does not prove a portal-governance defect; its terminal live-task failure was attributed to the separate Synology build-v2 task around merged PR #1335, and protected main has advanced that lifecycle since.
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
  - command: GitHub Actions CI at 2acc56708461c5f4dce853d837e04f1e56840a8b
    result: PASS
    evidence: run 34197970711
  - command: GitHub Actions security/production-like/content/events/wiki/build checks at 2acc56708461c5f4dce853d837e04f1e56840a8b
    result: PASS
    evidence: runs 34197970792, 34197970823, 34197970747, 34197970750, 34197970713, 34197970748, 34197970758, 34197970781 and 34197970707
  - command: Portal Acceptance Contract at 2acc56708461c5f4dce853d837e04f1e56840a8b
    result: FAIL
    evidence: run 34197970723; three exact administrator navigation/direct-entry gaps, account lifecycle passed
  - command: Acceptance E2E and Visual UX at 2acc56708461c5f4dce853d837e04f1e56840a8b
    result: FAIL
    evidence: run 34197970780; mobile-nav keyboard focus failure in portal-visual-review.spec.mjs
  - command: exact-head governance and prompt validation after PORTAL-POLISH handoff update
    result: NOT_RUN
    evidence: branch checks must run against the final handoff commit before this checkpoint can claim them
blockers:
  - none
next_action: Refresh PR #1334 branch from live protected main without dropping existing work, fix the mobile-nav Tab-focus failure and the three administrator navigation/direct-entry coverage gaps, continue real-page visual polish/review, then rerun focused and required exact-head checks.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: visual implementation, exact-candidate acceptance and protected integration are not terminal
source_branch_evidence: draft PR #1334 remains open
```

## Notes

The task is deliberately handed off through the existing canonical Portal prompt rather than a duplicate global programme. `PORTAL-POLISH` is only a live task-pinned continuation route.

Tracked-source recovery was previously verified against the baseline artifact and removed from the implementation candidate. No production deployment, live-data operation, payment/auth mutation or protection change is part of this task.

The administrator task directory uses a bounded read-only granted-permission projection to avoid per-link authorization queries. Existing `allows`, middleware, roles, sessions and mutation authority are unchanged; revocation remains covered by the task regression test.
