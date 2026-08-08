---
task_id: OTERYN-20260808-homepage-template-selector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/REPAIR_PR_ECONOMY.md
search_first:
  - issue #244
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
- [ ] Exact-head self-review, required CI and applicable E2E pass before merge.
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
updated_at: 2026-08-08T09:18:00+02:00
status: validating
phase: validate
session_id: chatgpt-20260808T0850+0200-homepage-template-validator
session_role: validator
execution_mode: github_connector
execution_reason: exact-head GitHub CI and full-diff review are the authoritative final validation path after implementation completion
lease_expires_at: 2026-08-08T10:03:00+02:00
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
last_completed_step: reduced Portal Exhaustive Audit infrastructure errors from 13 to zero and proved the remaining failure is the workflow exact-count invariant still pinned to the pre-feature 240/228 route inventory
heavy_validation_runs: 1
previous_session_heavy_validation_runs: 2
session_rotation_count: 1
stale_takeover_count: 0
human_interruptions: 0
issue: 244
branch: repair/issue-244
head: a3db80c1e78c6365213fe1e7cd97ec7d6c949764
base_sha: 5d8a9bcd46ca45984bb45e467d4837ad8f541b59
implementation_head: 5e186ca5b84c43f5bcc1b35a6a2d520bf86a3a14
previous_ci_head: a3db80c1e78c6365213fe1e7cd97ec7d6c949764
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
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
  - docs/agents/tasks/archive/OTERYN-20260808-homepage-template-selector.md
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/**
proven:
  - Current main lacked a homepage-template selector and hard-coded the home view.
  - Existing exact permission portal.settings.manage is reused without changing production role bundles.
  - Production remains the seeded/default key and maps to the unchanged current home view.
  - The alternative classic view is derived from the previously reviewed production portal presentation present at commit 9a0d7e295b9a43c7b9861bfdcc423b6429766350.
  - Public design-preview routes remain absent.
  - Agent Governance passed after the checkpoint contract was corrected.
  - Composer package discovery passes after removing the nonexistent controller base class and following repository controller conventions.
  - Portal Exhaustive Audit run 31245111891 reduced the original 13 infrastructure errors to four classification gaps.
  - Portal Exhaustive Audit run 31245683153 on a3db80c1e78c6365213fe1e7cd97ec7d6c949764 produced infrastructure_error_count 0 with 244 discovered named routes, 232 classified route records and 12 justified exclusions.
  - The remaining run 31245683153 failure is the workflow assertion still requiring the pre-feature exact counts 240/228.
derived:
  - Keeping Blade view names exclusively in the registry prevents database or browser values from becoming executable view selectors.
  - The audit workflow count assertion must advance atomically with the four newly classified named routes; weakening or deleting the count invariant is unnecessary.
unknown: []
conflicts: []
first_failure:
  marker: portal-exhaustive-route-count-contract-drift
  evidence: Portal Exhaustive Audit run 31245683153 generated zero infrastructure errors at 244/232/12, then Verify generated identity and audit validity failed on the stale 240/228 exact-count assertions.
rejected_hypotheses:
  - Reintroduce public /design routes; Issue #244 explicitly forbids a public design gallery.
  - Store arbitrary view names in the database; Issue #244 requires a code-owned allowlist.
  - Add or auto-grant a production administrator permission; portal.settings.manage already exists and the browser fixture grants it only in isolated acceptance data.
  - Weaken the exhaustive audit or remove exact route-count assertions; the correct repair is to advance the exact expected counts to the proven new inventory.
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
  - scripts/acceptance/coverage/validate-portal-content-scale-evidence.mjs
  - scripts/acceptance/coverage/validate-portal-media-strict-closure.mjs
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_DELEGATED_BINDINGS.json
  - docs/testing/portal-content-scale-surfaces/homepage-template-selector.json
  - docs/testing/portal-media-state-surfaces/homepage-template-selector.json
  - .github/workflows/portal-exhaustive-audit.yml
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
validation:
  - command: Agent Governance after checkpoint repair
    result: PASS
    evidence: run 31244685510 on head 38adf0c82d7de487cafe012cfe7653d32be36d91
  - command: Portal Exhaustive Audit run 31245111891
    result: FAIL
    evidence: four remaining classification infrastructure errors after the first route/view coverage repair
  - command: Portal Exhaustive Audit run 31245683153
    result: FAIL
    evidence: audit itself completed with infrastructure_error_count 0; terminal workflow assertion remained pinned to pre-feature route counts 240/228 instead of proven 244/232
blockers: []
next_action: Update only the proven Portal Exhaustive Audit exact route counts and source identities, then rerun exact-head CI/E2E and inspect the next first relevant failure.
```
