---
task_id: OTERYN-20260808-homepage-template-selector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
search_first:
  - issue #244
  - pull request #882
  - historical reviewed portal commit 9a0d7e295b9a43c7b9861bfdcc423b6429766350
optional_reads:
  - issue #240
  - issue #326
---

# OTERYN-20260808-homepage-template-selector

## Goal

Implement Issue #244 as one safe Platform-owned vertical slice: a code-owned homepage template registry, durable versioned active selection, MFA-confirmed exact-permission administrator preview/activate/rollback UI, bounded audit evidence and fail-closed public template resolution, while keeping public `/design/*` routes removed.

## Acceptance criteria

- [x] Only code-owned registered template keys can select Blade views.
- [x] Current `home` remains the deterministic default and fallback.
- [x] A previously reviewed classic portal presentation is restored only as a registry template and never as a public design-preview route.
- [x] Admin list/preview/activate/rollback requires auth + confirmed MFA + `portal.settings.manage`.
- [x] Preview is authenticated, `noindex,nofollow` and `no-store`.
- [x] Activation and rollback use optimistic version checks and bounded admin audit events.
- [x] Unknown stored keys fall back publicly and surface an admin warning.
- [x] EN/PL copy and responsive/keyboard-compatible semantic markup are implemented with an exact zero-retry Playwright journey.
- [x] Focused tests cover authorization, default, activation, stale conflict, invalid key, fallback and rollback.
- [ ] Exact-head self-review, required CI and applicable E2E pass after synchronization with current protected `main`.
- [ ] Issue #244 closes, task archives and ownership releases after resulting-main verification.

## Ownership

```yaml
owned_paths:
  - app/PublicPortal/HomepageTemplates/**
  - app/Http/Controllers/Admin/AdminHomepageTemplateController.php
  - app/Http/Controllers/PublicPortal/PublicHomeController.php
  - routes/modules/homepage-templates.php
  - resources/views/admin/homepage-templates/**
  - resources/views/home-classic.blade.php
  - resources/views/admin/dashboard.blade.php
  - database/migrations/*homepage_template*.php
  - lang/en/homepage_templates.php
  - lang/pl/homepage_templates.php
  - tests/Feature/HomepageTemplates/**
  - scripts/acceptance/tests/homepage-template-selector.spec.mjs
  - scripts/acceptance/prepare-homepage-template-selector.php
  - scripts/acceptance/coverage/surfaces/homepage-template-selector.json
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/homepage-template-selector.json
  - scripts/acceptance/coverage/validate-portal-content-scale-evidence.mjs
  - scripts/acceptance/coverage/validate-portal-media-strict-closure.mjs
  - docs/testing/ROUTE_VIEW_NAVIGATION_DELEGATED_BINDINGS.json
  - docs/testing/portal-content-scale-surfaces/homepage-template-selector.json
  - docs/testing/portal-media-state-surfaces/homepage-template-selector.json
  - .github/workflows/portal-exhaustive-audit.yml
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
  - docs/agents/tasks/archive/OTERYN-20260808-homepage-template-selector.md
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/**
shared_paths: []
modules:
  - public-web
  - admin
  - persistence
  - testing
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
policy_version: 2
checkpoint_version: 1
updated_at: 2026-08-08T18:09:51+02:00
status: validating
phase: validate
session_id: chatgpt-20260808T1809+0200-homepage-template-recovery
session_role: implementation_owner
execution_mode: github_connector
execution_reason: Resume the expired validation lease from durable PR/task state, synchronize the non-conflicting current main delta, then require a fresh exact-head validation generation and final full-diff review.
lease_expires_at: 2026-08-08T18:54:51+02:00
task_kind: implementation
implementation_authorized: true
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one cohesive portal-settings aggregate with one public consumer, one administrator lifecycle and its required exact audit evidence
validation_level: full
validation_intensity: HEIGHTENED
validation_risk: high
validation_triggers: administrator-authorization,durable-setting-migration,public-presentation-routing,optimistic-concurrency,audit-contract
validation_rationale: public render selection is controlled by an administrator-owned durable setting and its new routes must remain fully classified by the fail-closed portal audit
self_review_result: PENDING
self_review_exact_head: none
self_review_evidence: docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md
last_completed_step: Recovered expired PR #882 validation ownership, proved prior head 81690cf passed Agent Governance, Portal Exhaustive Audit, CI, Acceptance E2E/Visual UX, Content Scale, Portal Acceptance, Phase 7, DB outage, auth concurrency, edge security and staging-image workflows, then merged current main 0c6c630 into the repair branch without overlapping feature paths.
heavy_validation_runs: 1
previous_session_heavy_validation_runs: 2
session_rotation_count: 2
stale_takeover_count: 1
human_interruptions: 0
issue: 244
branch: repair/issue-244
head: f814f223556f3478ec57808259c40f8e34cbb341
base_sha: 0c6c630ecc7cb55c3a7ee8eac4d2627a91b751ca
implementation_head: 5e186ca5b84c43f5bcc1b35a6a2d520bf86a3a14
previous_ci_head: 81690cf811b8d9b5590b5a8e9a5c616a436a6b3c
pr: 882
context_routes:
  - public-web-cms
  - admin-rbac
  - database-persistence
  - frontend-ux
  - testing
  - ci-build-test
owned_paths:
  - app/PublicPortal/HomepageTemplates/**
  - app/Http/Controllers/Admin/AdminHomepageTemplateController.php
  - app/Http/Controllers/PublicPortal/PublicHomeController.php
  - routes/modules/homepage-templates.php
  - resources/views/admin/homepage-templates/**
  - resources/views/home-classic.blade.php
  - resources/views/admin/dashboard.blade.php
  - database/migrations/*homepage_template*.php
  - lang/en/homepage_templates.php
  - lang/pl/homepage_templates.php
  - tests/Feature/HomepageTemplates/**
  - scripts/acceptance/tests/homepage-template-selector.spec.mjs
  - scripts/acceptance/prepare-homepage-template-selector.php
  - scripts/acceptance/coverage/surfaces/homepage-template-selector.json
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/homepage-template-selector.json
  - scripts/acceptance/coverage/validate-portal-content-scale-evidence.mjs
  - scripts/acceptance/coverage/validate-portal-media-strict-closure.mjs
  - docs/testing/ROUTE_VIEW_NAVIGATION_DELEGATED_BINDINGS.json
  - docs/testing/portal-content-scale-surfaces/homepage-template-selector.json
  - docs/testing/portal-media-state-surfaces/homepage-template-selector.json
  - .github/workflows/portal-exhaustive-audit.yml
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
proven:
  - Current main before the feature lacked a homepage-template selector and hard-coded the home view.
  - Existing exact permission portal.settings.manage is reused without changing production role bundles.
  - Production remains the seeded/default key and maps to the unchanged current home view.
  - The alternative classic view is derived from the previously reviewed production portal presentation at 9a0d7e295b9a43c7b9861bfdcc423b6429766350.
  - Public design-preview routes remain absent.
  - Portal Exhaustive Audit run 31245683153 proved the feature inventory is exactly 244 discovered routes, 232 classified route records and 12 justified exclusions with zero infrastructure errors; the only failure was the then-stale 240/228 workflow assertion.
  - Current head 81690cf incorporated the 244/232 invariant correction and passed Portal Exhaustive Audit run 31265639398.
  - On 81690cf Agent Governance 31265639405, CI 31265639412, Acceptance E2E and Visual UX 31265639393, Content Scale Acceptance 31265639395, Portal Acceptance Contract 31265639404, Phase 7 31265639400, Platform DB Outage 31265639408, Game Auth Ticket Concurrency 31265639414, Edge Security Emulation 31265639396 and Build Synology Staging Images 31265639399 all passed.
  - Protected main advanced after that validation only through Issue #908 contract delivery/closeout; compare a4f3d03..main changed only the PublicGameData projection contract and the archived #908 task, neither overlapping homepage-selector feature paths.
  - Merge commit f814f223556f3478ec57808259c40f8e34cbb341 incorporates protected main 0c6c630ecc7cb55c3a7ee8eac4d2627a91b751ca into the repair branch without altering homepage-selector feature bytes.
derived:
  - Keeping Blade view names exclusively in the registry prevents database or browser values from becoming executable view selectors.
  - The correct audit repair is the exact 244/232 route inventory update rather than weakening or deleting the route-count invariant.
  - Previous PASS evidence is useful regression evidence but cannot replace the new exact-head checks after current-main synchronization and this recovery checkpoint.
unknown: []
conflicts: []
first_failure:
  marker: none-current
  evidence: The prior portal-exhaustive route-count drift was corrected; no current feature failure is proven pending the fresh exact-head workflow generation.
rejected_hypotheses:
  - Reintroduce public /design routes.
  - Store arbitrary view names in the database.
  - Add or auto-grant a production administrator permission.
  - Weaken or remove exhaustive audit exact route-count assertions.
changed_paths:
  - app/Http/Controllers/Admin/AdminHomepageTemplateController.php
  - app/Http/Controllers/PublicPortal/PublicHomeController.php
  - app/PublicPortal/HomepageTemplates/**
  - database/migrations/2026_08_08_061500_create_homepage_template_settings_table.php
  - lang/en/homepage_templates.php
  - lang/pl/homepage_templates.php
  - resources/views/admin/dashboard.blade.php
  - resources/views/admin/homepage-templates/index.blade.php
  - resources/views/home-classic.blade.php
  - routes/modules/homepage-templates.php
  - tests/Feature/HomepageTemplates/**
  - scripts/acceptance/tests/homepage-template-selector.spec.mjs
  - scripts/acceptance/prepare-homepage-template-selector.php
  - scripts/acceptance/coverage/**homepage-template-selector*
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_DELEGATED_BINDINGS.json
  - docs/testing/portal-content-scale-surfaces/homepage-template-selector.json
  - docs/testing/portal-media-state-surfaces/homepage-template-selector.json
  - .github/workflows/portal-exhaustive-audit.yml
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
validation:
  - command: prior release-candidate aggregate workflow generation on 81690cf811b8d9b5590b5a8e9a5c616a436a6b3c
    result: PASS_WITH_ONE_SUPERSEDED_PENDING
    evidence: All required feature/portal workflows listed in proven passed; Deep System Validation 31265639420 was still in progress when current-main synchronization superseded that head.
  - command: current-main overlap compare a4f3d03..0c6c630
    result: PASS
    evidence: Only PublicGameData privacy contract and archived #908 task changed; no feature path overlap.
blockers: []
next_action: Observe the fresh exact-head workflow generation triggered by this recovery checkpoint, inspect only the first relevant failure if any, then perform final full-diff self-review and merge when every required gate passes.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: chatgpt-20260808T1809+0200-homepage-template-recovery
  session_started_at: 2026-08-08T18:09:51+02:00
  checkpointed_at: 2026-08-08T18:09:51+02:00
  last_progress_at: 2026-08-08T18:09:51+02:00
  phase: validate
  exact_head: f814f223556f3478ec57808259c40f8e34cbb341
  pull_request: 882
  active_operation: fresh exact-head GitHub workflow generation
  external_run_ids: []
  operation_started_at: 2026-08-08T18:09:51+02:00
  wait_deadline_at: 2026-08-08T18:54:51+02:00
  check_generation: post-main-sync-recovery
  checks_used: 0
  status: waiting
  safe_to_resume: true
  resume_condition: GitHub workflows for the new repair head become observable and reach terminal required-gate results.
  next_action: Query one aggregate workflow snapshot for the current PR head and continue from the first relevant terminal failure or final self-review when all required gates pass.
```
