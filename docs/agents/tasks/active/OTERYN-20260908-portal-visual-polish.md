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

Finish the owner-rejected visual details of the integrated Platform redesign, preserve application/security contracts, prove real page families, and deliver through protected main using the existing Issue #1333, PR #1334 and branch `fix/20260908-portal-visual-polish` only.

## Acceptance criteria

- [x] Replace oversized or blurry thumbnail presentation and remove stretched decorative background seams.
- [x] Finish shared typography, spacing, density, controls, cards, tables, focus behavior, mobile navigation and footer presentation.
- [x] Cover representative public, account/security, commerce/support/error and administrator families in EN/PL and phone/tablet/desktop/wide viewports.
- [x] Record measured asset/layout diagnostics and bounded before/after visual evidence.
- [x] Preserve routes, domain/security/payment behavior and negative assertions.
- [ ] Pass fresh exact-integration-candidate checks after current-main reconciliation.
- [ ] Complete protected integration, task archival, Issue closeout and source-branch disposition.

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
dependencies: []
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T09:10:00Z
head: 1b9909342a303affe567fbfcb69193a2f2522a80
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
  - Issue #1333 and draft PR #1334 remain the canonical live lifecycle and branch ownership.
  - Commit 937fb0c341aaff277102be59e1513ad399286122 repaired the recorded mobile keyboard-focus regression and the three administrator route/navigation coverage gaps.
  - Acceptance E2E and Visual UX 34205457055 passed 145 configured PR-profile tests with zero failures/errors; Portal Acceptance Contract 34205457081 and CI/platform-gate 34205457014 passed on 937fb0c.
  - The repaired acceptance artifact records 206 real Laravel screenshot records across 21 page families, EN/PL and 320/390/820/1440/1920 px viewports with no recorded missing image, unlabelled-control or document-overflow finding.
  - Commit 1b9909342a303affe567fbfcb69193a2f2522a80 independently passed Acceptance E2E and Visual UX 34206540517, Agent Governance 34206540572, Game Auth Concurrency 34206540745, Content Scale 34206540584, Edge Security 34206540628, DB Outage 34206540573, Phase 7 34206540650, CodeQL 34206540490, Build Synology Staging Images 34206540645, Events 34206540607, CI 34206540524, Wiki Reconciliation 34206540531 and Portal Acceptance Contract 34206540511.
  - Protected main is 7bf4d26efb8cd9e25a4090f283d701f4d3a304e0 and differs from the previous reconciliation only by the independent Synology routing documentation plus active-to-archive task transition from PR #1340.
  - Candidate-only public/images/oteryn-thais-concept.webp is unused and its connector-read binary prefix is not a valid WebP RIFF/WEBP header; final integration removes this broken public asset instead of wiring it into UI.
  - docs/testing/PORTAL_VISUAL_POLISH_2026-09-08.md records measured before/after evidence and explicit performance-claim boundaries.
derived:
  - The final runtime candidate should retain the already-green Portal implementation, inherit protected main 7bf4d26 exactly, remove the unused invalid candidate image and then rerun required exact-head validation.
  - No production deployment, payment/live-auth mutation, protection change or cross-repository work is required for this closeout.
unknown:
  - Fresh exact-head workflow results after this reconciliation commit.
  - Protected integration result and resulting main SHA.
conflicts: []
first_failure:
  marker: none-open-on-validated-portal-runtime
  evidence: both historical task-owned failures are repaired; final candidate validation remains pending after reconciliation.
rejected_hypotheses:
  - The administrator coverage repair requires permission expansion or scanner exemptions.
  - Passing browser automation alone proves unmeasured production performance.
  - The unused candidate artwork should be wired into UI despite failing bounded WebP header recognition.
changed_paths:
  - app/Admin/AdminAuthorization.php
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/tasks/active/OTERYN-20260908-portal-visual-polish.md
  - docs/testing/PORTAL_VISUAL_POLISH_2026-09-08.md
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
  - command: Acceptance E2E and Visual UX run 34205457055 at 937fb0c
    result: PASS
    evidence: 145 configured PR-profile test cases with zero failures/errors plus secret-safe screenshot/manifest evidence.
  - command: Portal Acceptance Contract 34205457081 and CI 34205457014 at 937fb0c
    result: PASS
    evidence: strict coverage, account lifecycle, runtime tests, static analysis and platform-gate passed.
  - command: all PR-triggered required workflows at 1b990934
    result: PASS
    evidence: runs 34206540490 through 34206540745 listed in proven evidence all completed successfully.
  - command: final reconciled exact-head workflows
    result: NOT_RUN
    evidence: starts after publication of the reconciliation commit.
blockers: []
next_action: Publish the current-main reconciliation with the invalid unused candidate removed, rerun exact-head required checks, then archive the task and integrate PR #1334 through the protected path.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: exact integration candidate and protected merge are not terminal yet
source_branch_evidence: PR #1334 remains open
```

## Notes

The task remains Platform-only. The Synology closeout paths included by current-main reconciliation are inherited byte-for-byte from protected main and are not Portal task mutations. No production deployment, live-data operation, payment/auth mutation or protection change is part of PORTAL-POLISH.
