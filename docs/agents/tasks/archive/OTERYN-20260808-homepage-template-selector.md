---
task_id: OTERYN-20260808-homepage-template-selector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/REPAIR_PR_ECONOMY.md
search_first:
  - Issue #244
  - PR #882
  - historical reviewed portal commit 9a0d7e295b9a43c7b9861bfdcc423b6429766350
optional_reads:
  - Issue #240
  - Issue #326
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
- [x] Exact-head self-review, all 12 required CI/workflow gates and applicable zero-retry E2E passed after synchronization with protected `main`.
- [x] PR #882 squash-merged to protected `main`, Issue #244 closed, resulting-main delivery verified, task archived and active ownership released.

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
  - docs/agents/tasks/archive/OTERYN-20260808-homepage-template-selector.md
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/**
shared_paths: []
modules:
  - public-web
  - admin
  - persistence
  - testing
blockers:
  - none
cross_repository_tasks:
  - none
```

## Terminal checkpoint

```yaml
policy_version: 2
checkpoint_version: 1
updated_at: 2026-08-08T18:46:00+02:00
status: completed
phase: closeout
terminal_pr_policy: delivered
session_id: chatgpt-20260808T1809+0200-homepage-template-recovery
session_role: implementation_owner
execution_mode: github_connector
execution_reason: The bounded homepage-template selector was delivered through an exact-head validated pull request; this closeout archives lifecycle evidence only and performs no production activation.
task_kind: implementation
implementation_authorized: true
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: full
validation_intensity: HEIGHTENED
validation_risk: high
validation_triggers: administrator-authorization,durable-setting-migration,public-presentation-routing,optimistic-concurrency,audit-contract
validation_rationale: The delivered feature changes an administrator-controlled durable setting that selects a public rendering path, so authorization, allowlisting, rollback, stale-write behavior and browser acceptance required heightened validation.
self_review_result: PASS
self_review_exact_head: eaaae106eff9c22f78f2d400f6963cb057bd6a96
self_review_evidence: docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md
last_completed_step: PR #882 squash-merged to protected main as 4cdcbc3a94ca149bf0a86b4be263d95a4232df8a after exact-head 12/12 workflow PASS and final full-diff self-review; Issue #244 closed automatically and resulting main contains the delivered registry.
issue: 244
branch: repair/issue-244
head: eaaae106eff9c22f78f2d400f6963cb057bd6a96
base_sha: 0c6c630ecc7cb55c3a7ee8eac4d2627a91b751ca
implementation_head: d705be6a216da319f75f20035c27858b2b569518
pr: 882
merge_sha: 4cdcbc3a94ca149bf0a86b4be263d95a4232df8a
context_routes:
  - public-web-cms
  - admin-rbac
  - database-persistence
  - frontend-ux
  - testing
  - ci-build-test
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260808-homepage-template-selector.md
  - docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md
proven:
  - The selector reuses exact permission portal.settings.manage without widening production role bundles.
  - Registered Blade view names remain code-owned; persisted or browser-provided values cannot become arbitrary view paths.
  - Production is the seeded default/fallback and deployment alone does not activate the classic alternative.
  - Administrator preview is authenticated, MFA-confirmed, permission-gated, noindex and no-store.
  - Activation and rollback lock the singleton setting, compare the submitted version and record bounded administrator audit evidence.
  - Unknown persisted keys fail closed to the production template and are surfaced as administrator drift.
  - Public /design/* preview routes remain absent.
  - Portal Exhaustive Audit validates the resulting inventory as 244 discovered routes, 232 classified route records and 12 justified exclusions with zero infrastructure errors.
  - Release-candidate head eaaae106eff9c22f78f2d400f6963cb057bd6a96 passed all 12 required workflows after synchronization with protected main.
  - Final PR #882 review had zero review threads and zero submitted reviews; the full diff was rechecked with no material UNKNOWN or CONFLICT.
  - PR #882 was marked ready and squash-merged with expected-head protection as 4cdcbc3a94ca149bf0a86b4be263d95a4232df8a.
  - GitHub closed Issue #244 automatically through the delivery merge.
  - Resulting protected main points to 4cdcbc3a94ca149bf0a86b4be263d95a4232df8a and contains HomepageTemplateRegistry with production and classic allowlisted mappings.
  - No production template activation, environment mutation, secret access or external-repository mutation was performed.
derived:
  - The code-owned registry plus deterministic fallback keeps public rendering fail closed even if durable data drifts from the registered template set.
  - Separating delivery from docs-only lifecycle closeout preserves the exact runtime release-candidate validation evidence while allowing ownership to be released cleanly.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Reintroduce public /design routes.
  - Store arbitrary view names or PHP classes in durable state.
  - Auto-grant portal.settings.manage to a production role bundle.
  - Weaken exhaustive-audit route-count assertions instead of updating proven inventory.
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20260808-homepage-template-selector.md
validation:
  - command: final full-diff self-review on eaaae106eff9c22f78f2d400f6963cb057bd6a96
    result: PASS
    evidence: All Issue #244 acceptance and safety boundaries checked; zero material findings, zero review threads and zero review submissions.
  - command: Agent Governance run 31266576882
    result: PASS
    evidence: Exact release-candidate head passed live governance validation.
  - command: CI run 31266576866
    result: PASS
    evidence: Exact release-candidate head passed repository CI including classification, formatting, static analysis and runtime tests.
  - command: Portal Exhaustive Audit run 31266576918
    result: PASS
    evidence: Exact release-candidate portal inventory and strict evidence contracts passed.
  - command: Acceptance E2E and Visual UX run 31266576877
    result: PASS
    evidence: Exact release-candidate browser acceptance passed.
  - command: Deep System Validation run 31266576883
    result: PASS
    evidence: Exact release-candidate zero-retry deep browser matrix and evidence compiler passed.
  - command: Portal Acceptance Contract run 31266576908
    result: PASS
    evidence: Exact release-candidate portal acceptance passed.
  - command: Content Scale Acceptance run 31266576868
    result: PASS
    evidence: Exact release-candidate strict content-scale evidence passed.
  - command: Phase 7 Production-Like Validation run 31266576851
    result: PASS
    evidence: Exact release-candidate production-like validation passed.
  - command: Platform DB Outage Validation run 31266576903
    result: PASS
    evidence: Exact release-candidate outage validation passed.
  - command: Game Auth Ticket Concurrency run 31266576897
    result: PASS
    evidence: Exact release-candidate auth concurrency validation passed.
  - command: Edge Security Emulation run 31266576863
    result: PASS
    evidence: Exact release-candidate edge security validation passed.
  - command: Build Synology Staging Images run 31266576861
    result: PASS
    evidence: Exact release-candidate image-build workflow passed.
  - command: resulting-main verification
    result: PASS
    evidence: Protected main is 4cdcbc3a94ca149bf0a86b4be263d95a4232df8a, Issue #244 is closed, and HomepageTemplateRegistry is present on main with the intended allowlist.
blockers:
  - none
next_action: none
```

## Delivery evidence

- Issue: #244.
- Delivery PR: #882.
- Durable evidence: `docs/agents/evidence/OTERYN-20260808-homepage-template-selector/index.md`.
- Validated release-candidate head: `eaaae106eff9c22f78f2d400f6963cb057bd6a96`.
- Exact-head required workflows: 12/12 PASS.
- Deep zero-retry validation: run `31266576883` — PASS.
- Protected-main delivery: `4cdcbc3a94ca149bf0a86b4be263d95a4232df8a`.
- Issue state after merge: closed / completed.
- Production template activation: not performed.
