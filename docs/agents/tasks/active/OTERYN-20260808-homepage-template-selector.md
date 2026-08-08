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
- [x] The reviewed classic presentation exists only as a registered template; public `/design/*` routes stay removed.
- [x] Admin list/preview/activate/rollback requires auth + confirmed MFA + `portal.settings.manage`.
- [x] Preview is authenticated, `noindex,nofollow` and `no-store/private`.
- [x] Activation and rollback use optimistic version checks and bounded admin audit events.
- [x] Unknown stored keys fail publicly to the production template and surface an administrator warning.
- [x] EN/PL, responsive layout, keyboard focus and cache controls have zero-retry Playwright coverage.
- [x] Focused tests cover authorization, default, activation, stale conflict, invalid key, fallback, rollback and migration default.
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
  - scripts/acceptance/coverage/**homepage-template-selector*
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
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
updated_at: 2026-08-08T18:12:19+02:00
status: validating
phase: validate
session_id: chatgpt-20260808T1809+0200-homepage-template-recovery
session_role: implementation_owner
execution_mode: github_connector
execution_reason: Resume the expired validation lease from durable state, synchronize current protected main, then require a fresh exact-head validation generation and final full-diff review.
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
validation_rationale: public render selection is controlled by an administrator-owned durable setting and its routes must remain fully classified by the fail-closed portal audit
self_review_result: PENDING
self_review_exact_head: none
self_review_evidence: docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md
last_completed_step: Recovery incorporated protected main 0c6c630 into PR #882, reviewed the critical authorization/registry/store/migration/public-render/test paths, and identified the first new exact-head failure as unsupported checkpoint result metadata rather than a product defect.
heavy_validation_runs: 1
previous_session_heavy_validation_runs: 2
session_rotation_count: 2
stale_takeover_count: 1
human_interruptions: 0
issue: 244
branch: repair/issue-244
head: b2212ec7c932c1023db898cf8e7068f54e604796
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
  - scripts/acceptance/coverage/**homepage-template-selector*
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
  - scripts/acceptance/coverage/validate-portal-content-scale-evidence.mjs
  - scripts/acceptance/coverage/validate-portal-media-strict-closure.mjs
  - docs/testing/ROUTE_VIEW_NAVIGATION_DELEGATED_BINDINGS.json
  - docs/testing/portal-content-scale-surfaces/homepage-template-selector.json
  - docs/testing/portal-media-state-surfaces/homepage-template-selector.json
  - .github/workflows/portal-exhaustive-audit.yml
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
proven:
  - The selector uses the existing exact permission portal.settings.manage without widening production role bundles.
  - Registered Blade view names are code-owned; persisted/browser template keys cannot become arbitrary view paths.
  - Production is the migration/default/fallback key and deployment alone does not activate the classic alternative.
  - Mutations lock the singleton row, compare the submitted expected version, and audit activation/rollback.
  - Unknown persisted keys fall back through the registry and are surfaced as drift to administrators.
  - Portal Exhaustive Audit previously proved the feature inventory as 244 discovered routes, 232 classified route records and 12 exclusions with zero infrastructure errors; current 81690cf corrected the stale exact-count assertion.
  - On 81690cf Agent Governance 31265639405, Portal Exhaustive Audit 31265639398, CI 31265639412, Acceptance E2E and Visual UX 31265639393, Content Scale Acceptance 31265639395, Portal Acceptance Contract 31265639404, Phase 7 31265639400, Platform DB Outage 31265639408, Game Auth Ticket Concurrency 31265639414, Edge Security Emulation 31265639396 and Build Synology Staging Images 31265639399 all passed.
  - Protected main advanced after 81690cf only through Issue #908 contract delivery/closeout; compare a4f3d03..0c6c630 changed only the PublicGameData contract and archived #908 task, with no homepage-selector overlap.
  - Merge commit f814f223556f3478ec57808259c40f8e34cbb341 incorporated current main without changing homepage-selector feature bytes.
  - Exact-head generation for b2212ec produced Portal Exhaustive Audit PASS; CI and Agent Governance failed solely because the task validation record used an unsupported result enum.
derived:
  - The earlier PASS generation is regression evidence but cannot replace fresh exact-head gates after current-main synchronization and checkpoint correction.
unknown: []
conflicts: []
first_failure:
  marker: checkpoint-validation-result-enum
  evidence: CI run 31266318778 classify-changes log rejected PASS_WITH_ONE_SUPERSEDED_PENDING; product/runtime code was not the failing path.
rejected_hypotheses:
  - Reintroduce public /design routes.
  - Store arbitrary view names in the database.
  - Add or auto-grant a production administrator permission.
  - Weaken exhaustive audit route-count assertions.
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
  - docs/testing/ROUTE_VIEW_NAVIGATION_DELEGATED_BINDINGS.json
  - docs/testing/portal-content-scale-surfaces/homepage-template-selector.json
  - docs/testing/portal-media-state-surfaces/homepage-template-selector.json
  - .github/workflows/portal-exhaustive-audit.yml
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
validation:
  - command: prior required feature/portal workflows on 81690cf811b8d9b5590b5a8e9a5c616a436a6b3c
    result: PASS
    evidence: Agent Governance, Portal Exhaustive Audit, CI, Acceptance E2E/Visual UX, Content Scale, Portal Acceptance, Phase 7, DB outage, auth concurrency, edge security and staging-image workflows all completed successfully.
  - command: current-main overlap compare a4f3d03..0c6c630
    result: PASS
    evidence: Only PublicGameData privacy contract and archived #908 task changed; no feature path overlap.
  - command: b2212ec exact-head CI / Agent Governance
    result: FAIL
    evidence: Unsupported task checkpoint validation result enum only; corrected by this commit.
blockers: []
next_action: Observe the new exact-head workflow generation, inspect only the first relevant failure if any, then perform final full-diff self-review and merge when every required gate passes.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: chatgpt-20260808T1809+0200-homepage-template-recovery
  session_started_at: 2026-08-08T18:09:51+02:00
  checkpointed_at: 2026-08-08T18:12:19+02:00
  last_progress_at: 2026-08-08T18:12:19+02:00
  phase: validate
  exact_head: b2212ec7c932c1023db898cf8e7068f54e604796
  pull_request: 882
  active_operation: fresh exact-head GitHub workflow generation after checkpoint enum correction
  external_run_ids: [31266318778, 31266318746]
  operation_started_at: 2026-08-08T18:12:19+02:00
  wait_deadline_at: 2026-08-08T18:54:51+02:00
  check_generation: post-checkpoint-enum-correction
  checks_used: 1
  status: waiting
  safe_to_resume: true
  resume_condition: GitHub workflows for the corrected head become observable and required gates reach terminal results.
  next_action: Query one aggregate workflow snapshot for the current PR head and continue from the first relevant terminal failure or final self-review when all required gates pass.
```
