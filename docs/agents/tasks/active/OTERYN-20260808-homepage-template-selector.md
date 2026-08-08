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
  - historical homepage comparison commit 9a0d7e295b9a43c7b9861bfdcc423b6429766350
optional_reads:
  - issue #240
  - issue #326
---

# OTERYN-20260808-homepage-template-selector

## Goal

Implement Issue #244 as one safe Platform-owned vertical slice: a code-owned homepage template registry, durable versioned active selection, MFA-confirmed exact-permission administrator preview/activate/rollback UI, bounded audit evidence and fail-closed public template resolution, while keeping public `/design/*` routes removed.

## Acceptance criteria

- [ ] Only code-owned registered template keys can select Blade views.
- [ ] Current `home` remains the deterministic default and fallback.
- [ ] Historical approved `home-v2` presentation is restored only as a registry template and never as a public design-preview route.
- [ ] Admin list/preview/activate/rollback requires auth + confirmed MFA + `portal.settings.manage`.
- [ ] Preview is authenticated, `noindex,nofollow` and `no-store`.
- [ ] Activation and rollback use optimistic version checks and bounded admin audit events.
- [ ] Unknown stored keys fall back publicly and surface an admin warning.
- [ ] EN/PL, responsive and keyboard/focus behavior is covered by applicable acceptance.
- [ ] Focused tests cover authorization, default, activation, stale conflict, invalid key, fallback and rollback.
- [ ] Exact-head self-review, required CI and applicable E2E pass before merge.
- [ ] Issue #244 closes, task archives and ownership releases after resulting-main verification.

## Ownership

```yaml
owned_paths:
  - app/PublicPortal/HomepageTemplates/**
  - app/Http/Controllers/Admin/AdminHomepageTemplateController.php
  - app/Http/Requests/Admin/AdminHomepageTemplateRequest.php
  - app/Http/Controllers/PublicPortal/PublicHomeController.php
  - routes/modules/homepage-templates.php
  - resources/views/admin/homepage-templates/**
  - resources/views/home-preview.blade.php
  - public/css/home-preview.css
  - database/migrations/*homepage_template*.php
  - tests/Feature/HomepageTemplates/**
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
  - docs/agents/tasks/archive/OTERYN-20260808-homepage-template-selector.md
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
updated_at: 2026-08-08T08:16:00+02:00
status: implementing
phase: implement
session_id: chatgpt-20260808T0816+0200-homepage-template-selector
session_role: implementer
execution_mode: github_connector
execution_reason: repository-owned multi-file implementation is available through the connected GitHub API; local checkout is unavailable in the sandbox, so exact-head GitHub CI will provide authoritative runtime validation
lease_expires_at: 2026-08-08T09:01:00+02:00
task_kind: implementation
implementation_authorized: true
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: medium
decomposition_decision: single
decomposition_reason: one cohesive portal-settings aggregate with one public consumer and one administrator lifecycle
validation_level: focused
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
issue: 244
branch: repair/issue-244
base_sha: 0582b0e853d1b5e983f664452268e7777c886904
head: 0582b0e853d1b5e983f664452268e7777c886904
pr: none
validation_gate:
  version: 2
  intensity: HEIGHTENED
  risk: high
  triggers:
    - administrator authorization
    - durable setting migration
    - public presentation routing
    - optimistic concurrency
  unknown_or_conflict: []
  rationale: public render selection is controlled by an administrator-owned durable setting and must fail closed to a code-owned allowlist
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
proven:
  - Current main PublicHomeController hard-codes the home view.
  - Existing exact permission portal.settings.manage is available and requires no RBAC expansion.
  - Historical commit 9a0d7e295b9a43c7b9861bfdcc423b6429766350 contains the previously validated home-v2 comparison presentation.
  - Public design-preview routes are absent from current main.
derived:
  - Reusing the historical validated presentation behind a code-owned registry avoids inventing a new product design while satisfying the selector requirement.
unknown: []
conflicts: []
first_failure:
  marker: homepage-template-selection-capability-absent
  evidence: PublicHomeController renders view('home') directly and no durable selector exists on audited main.
rejected_hypotheses:
  - Reintroduce /design/home-v2 publicly; Issue #244 explicitly forbids a public design gallery.
  - Store arbitrary view names in the database; Issue #244 requires a code-owned allowlist.
  - Add a new administrator permission; portal.settings.manage already exists.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-homepage-template-selector.md
validation: []
blockers: []
next_action: Implement the registry, durable versioned setting, safe resolver, admin lifecycle, historical approved template restoration and focused tests on repair/issue-244.
```
