---
task_id: OTERYN-20260908-portal-visual-polish
governing_issue: 1333
required_reads: []
search_first:
  - current public and authenticated page-family renders
optional_reads: []
---

# OTERYN-20260908-portal-visual-polish

## Goal

Governing GitHub Issue: #1333. Finish the owner-rejected visual details of the integrated Platform redesign, preserve application/security contracts, prove real page families, and deliver through protected main.

Admission main: `519f610b9e02641283cc4322c80b74a9462edb11`. The previous #1298 task is terminal; this follow-up preserves its implementation rather than restarting it. The active #1331 staging-build work is path-disjoint and is not owned here.

## Acceptance criteria

- [ ] Replace oversized thumbnail presentation, background seams and unfinished shared visual hierarchy.
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
  verified_at: 2026-09-08T06:36:00Z
  repository: Oteryn/Oteryn-Platform
  default_branch_sha: 519f610b9e02641283cc4322c80b74a9462edb11
  governing_issue: 1333
  pull_request: 1334
  task_head_sha: 4b71868c969766416c57246bf74186168375d125
parallel_execution:
  effort: high
  lane_strategy: single_agent
  decision_basis: Shared visual tokens, templates and screenshot acceptance require one coherent writer; existing hosted browser jobs provide independent parallel validation.
  lanes:
    - id: portal-polish
      owned_paths: [public/css, public/images, resources/views, lang, scripts/acceptance/tests/portal-polish-quality.spec.mjs, scripts/acceptance/tests/portal-visual-review.spec.mjs, tests/Feature/PortalVisualPolishTest.php]
      dependencies: []
      branch: refs/heads/fix/20260908-portal-visual-polish
      worktree: sandbox:/mnt/data/platform-ui-work
      shared_leases: []
  integration_order: [portal-polish]
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T06:36:00Z
head: 4b71868c969766416c57246bf74186168375d125
branch: fix/20260908-portal-visual-polish
pr: 1334
status: implementing
context_routes:
  - web-cms
  - admin-rbac
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
  - docs/agents/tasks/active/OTERYN-20260908-portal-visual-polish.md
proven:
  - PR 1298 is merged and its implementation is contained in admission main.
  - The six decorative scene strips are only 240 by 100 pixels.
  - The isolated workspace has no external DNS and GitHub connector reads/writes are available.
derived: []
unknown:
  - Final corrected rendered quality and resource measurements.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - app/Admin/AdminAuthorization.php
  - public/css/home-production.css
  - public/css/portal-art-direction.css
  - public/css/portal-admin.css
  - resources/views/home.blade.php
  - resources/views/news/index.blade.php
  - resources/views/news/show.blade.php
  - resources/views/events/show.blade.php
  - resources/views/wiki/index.blade.php
  - resources/views/admin/dashboard.blade.php
  - resources/views/admin/partials/navigation.blade.php
  - lang/en/portal_art.php
  - lang/pl/portal_art.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - tests/Feature/PortalVisualPolishTest.php
validation:
  - command: hosted visual-quality baseline
    result: PASS
    evidence: Run 34194603808, artifact 10043606384, exact 4b71868; 24 resource/layout records recovered.
  - command: corrected candidate application and browser checks
    result: NOT_RUN
    evidence: Implementation candidate is being published; no final acceptance claim.
blockers:
  - none
next_action: Publish the coherent visual corrections and inspect exact-candidate rendered evidence.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: implementation and exact-candidate acceptance are still active
source_branch_evidence: pending
```

## Notes

Tracked-source recovery was verified against artifact 10043606384 at 4b71868 and removed from the implementation candidate. No production deployment, live-data operation or protection change is part of this task.

The administrator task directory uses a bounded read-only granted-permission projection to avoid per-link authorization queries. Existing `allows`, middleware, roles, sessions and mutation authority are unchanged; revocation is covered by a regression test.
