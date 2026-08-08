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
- [x] EN/PL copy and responsive/keyboard-compatible semantic markup are implemented; applicable browser acceptance remains pending CI.
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
  - database/migrations/*homepage_template*.php
  - lang/en/homepage_templates.php
  - lang/pl/homepage_templates.php
  - tests/Feature/HomepageTemplates/**
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
  - docs/agents/tasks/archive/OTERYN-20260808-homepage-template-selector.md
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/**
shared_paths:
  - resources/views/admin/dashboard.blade.php
modules:
  - public-web
  - admin
  - persistence
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
policy_version: 2
checkpoint_version: 1
updated_at: 2026-08-08T08:50:00+02:00
status: validating
phase: validate
session_id: chatgpt-20260808T0850+0200-homepage-template-validator
session_role: validator
execution_mode: github_connector
execution_reason: exact-head GitHub CI and full-diff review are the authoritative final validation path after implementation completion
lease_expires_at: 2026-08-08T09:35:00+02:00
task_kind: implementation
implementation_authorized: true
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one cohesive portal-settings aggregate with one public consumer and one administrator lifecycle
validation_level: full
validation_intensity: HEIGHTENED
validation_risk: high
validation_triggers: administrator-authorization,durable-setting-migration,public-presentation-routing,optimistic-concurrency
validation_rationale: public render selection is controlled by an administrator-owned durable setting and must fail closed to a code-owned allowlist
self_review_result: PENDING
self_review_exact_head: none
self_review_evidence: docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md
last_completed_step: isolated both failed validation waves to active-task checkpoint schema and completed the required checkpoint fields before rotating the validation session
heavy_validation_runs: 0
previous_session_heavy_validation_runs: 2
session_rotation_count: 1
stale_takeover_count: 0
human_interruptions: 0
issue: 244
branch: repair/issue-244
head: a51d6635b798f6e0e4a8c0b2f7d23c465a87bdc4
base_sha: 5d8a9bcd46ca45984bb45e467d4837ad8f541b59
implementation_head: 5e186ca5b84c43f5bcc1b35a6a2d520bf86a3a14
previous_ci_head: a51d6635b798f6e0e4a8c0b2f7d23c465a87bdc4
pr: 882
context_routes:
  - public-web-cms
  - admin-rbac
  - database-persistence
  - frontend-ux
  - testing
owned_paths:
  - app/PublicPortal/HomepageTemplates/**
  - app/Http/Controllers/Admin/AdminHomepageTemplateController.php
  - app/Http/Controllers/PublicPortal/PublicHomeController.php
  - routes/modules/homepage-templates.php
  - resources/views/admin/homepage-templates/**
  - resources/views/home-classic.blade.php
  - database/migrations/*homepage_template*.php
  - lang/en/homepage_templates.php
  - lang/pl/homepage_templates.php
  - tests/Feature/HomepageTemplates/**
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
  - docs/agents/tasks/archive/OTERYN-20260808-homepage-template-selector.md
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/**
  - resources/views/admin/dashboard.blade.php
proven:
  - Current main lacked a homepage-template selector and hard-coded the home view.
  - Existing exact permission portal.settings.manage is reused without changing role bundles.
  - Production remains the seeded/default key and maps to the unchanged current home view.
  - The alternative classic view is derived from the previously reviewed production portal presentation present at commit 9a0d7e295b9a43c7b9861bfdcc423b6429766350.
  - Public design-preview routes remain absent.
  - PR 882 was synchronized without force to main 5d8a9bcd46ca45984bb45e467d4837ad8f541b59 at head 1ad1633cd4479bfe1b71e43384197da799b0687e.
  - Agent Governance run 31244537189 isolated the first failure to unsupported nested validation_gate checkpoint syntax.
  - Agent Governance run 31244622332 accepted flattened syntax and then reported the three exact missing required fields context_routes, head and owned_paths.
derived:
  - Keeping Blade view names exclusively in the registry prevents database or browser values from becoming executable view selectors.
  - Early workflow failures on the first two validation heads are checkpoint-governance failures and are not evidence of a product/runtime defect.
unknown: []
conflicts: []
first_failure:
  marker: checkpoint-validation-required-fields
  evidence: Agent Governance run 31244622332 job 93071047880 reported missing context_routes, head and owned_paths after checkpoint parser syntax passed.
rejected_hypotheses:
  - Reintroduce public /design routes; Issue #244 explicitly forbids a public design gallery.
  - Store arbitrary view names in the database; Issue #244 requires a code-owned allowlist.
  - Add or auto-grant a new administrator permission; portal.settings.manage already exists and automatic role expansion is forbidden by scope.
  - Treat the first two workflow waves as product failures; both first concrete errors are active-task governance contract violations.
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
  - tests/Feature/HomepageTemplates/HomepageTemplateMigrationTest.php
  - tests/Feature/HomepageTemplates/HomepageTemplateSelectorTest.php
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
validation:
  - command: bounded static whole-slice review before publish
    result: PASS
    evidence: allowlist-only resolution, default preservation, permission boundary, preview cache/index controls, versioned mutation, rollback and test coverage reviewed before PR
  - command: Agent Governance run 31244537189 on PR 882 head 1ad1633cd4479bfe1b71e43384197da799b0687e
    result: FAIL
    evidence: checkpoint validator rejected nested validation_gate syntax; no runtime defect established
  - command: Agent Governance run 31244622332 on PR 882 head a51d6635b798f6e0e4a8c0b2f7d23c465a87bdc4
    result: FAIL
    evidence: flattened syntax passed parsing; validator then required context_routes, head and owned_paths
blockers: []
next_action: Validate the completed checkpoint contract on the rotated-session head, then inspect the next first relevant CI failure and continue exact-head product validation.
```
