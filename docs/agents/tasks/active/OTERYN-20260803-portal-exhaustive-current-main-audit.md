---
task_id: OTERYN-20260803-portal-exhaustive-current-main-audit
policy_version: 2
project_lane: oteryn-platform-core
task_kind: audit
execution_mode: github-only
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json
  - docs/testing/product-backend-frontend-completeness.json
  - scripts/acceptance/coverage/portal-coverage-manifest.json
search_first:
  - Issue 326 scope and latest comments
  - PR 381 frozen-target evidence and findings
  - open PRs and active tasks overlapping portal audit evidence paths
  - current route view navigation capability and browser coverage contracts
---

# OTERYN-20260803-portal-exhaustive-current-main-audit

## Goal

Continue the exhaustive Oteryn Platform portal completeness audit from Issue #326 and historical PR #381 on the current trusted `main`, without restarting prior inventory work. Produce fail-closed record-level verdicts for every delivered module, route, rendered screen, role/state variant and content/data collection, then track every material gap in an owner Issue.

## Audit-only boundary

This task may add or update audit tooling, deterministic audit fixtures, evidence, reports, workflow validation and GitHub Issues. It must not implement or repair product behavior, change production/deployment state, mutate live data, or modify external repositories.

## Execution budget

```yaml
run_scope: autonomous_program
large_foreground_runtime_minutes: 120
large_budget_reason: exhaustive current-main reconciliation of 240 historical named routes, 126 historical rendered screens, 18 modules, content inventories and fresh browser evidence after the PHP 8.5 runtime unblock
```

## Acceptance criteria

- [ ] Historical PR #381 evidence is treated as a frozen baseline and explicitly mapped to current `main`; no prior proven inventory is silently discarded or re-created.
- [ ] Every current named route and rendered view is represented exactly once as a rendered screen, form action, redirect, supporting resource or justified exception.
- [ ] Every auditable record has `EXISTS`, `FUNCTIONAL`, `CONTENT_COMPLETE` and `PRODUCTION_COMPLETE` verdicts plus fail-closed final classification.
- [ ] Every user-facing capability maps to backend, frontend, real integration, states, browser evidence and an owner module.
- [ ] Public Portal, Account Center, Admin, Wiki, Game Catalog, Bazaar and application/dependency error surfaces are audited page-by-page and collection-by-collection.
- [ ] Wiki uses an explicit expected category/article/source/localization/media inventory rather than page presence alone.
- [ ] Game Catalog uses explicit supported entity/version/count/field/relation/localization/media/search/filter/pagination/detail/admin-inspection contracts rather than sample records.
- [ ] Fresh zero-retry browser evidence runs through the merged PHP 8.5 runtime on the exact audit head, with desktop/tablet/mobile Chromium and risk-based Firefox/WebKit mapping.
- [ ] Accessibility, EN/PL, overflow, long data, large bounded result sets, 404/419/429/500/503/recovery and applicable missing/broken-media states are explicit rather than implied.
- [ ] Each material finding has a stable ID, severity, exact evidence, impact, disposition and dedicated or explicitly shared owner Issue.
- [ ] One consolidated current-main verdict and machine-readable audit matrix are persisted and exact-head CI passes.
- [ ] Historical PR #381 reaches an intentional terminal state after its evidence is superseded or incorporated.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260803-portal-exhaustive-current-main-audit.md
  - docs/agents/tasks/archive/OTERYN-20260803-portal-exhaustive-current-main-audit.md
  - docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit/**
  - docs/agents/reports/OTERYN-20260803-portal-exhaustive-current-main-audit*.md
  - tools/audit/portal_exhaustive_audit.py
  - tools/audit/test_portal_exhaustive_audit.py
  - .github/workflows/portal-exhaustive-audit.yml
modules:
  - identity
  - accounts
  - characters
  - public_game_data
  - cms_content
  - editorial_media
  - wiki
  - support_moderation
  - admin_rbac_audit
  - wallet_marketplace
  - game_catalog
  - platform_api
  - payments
  - products_entitlements
  - legal_privacy_commerce
  - operations_observability
  - public_edge
  - quality_e2e
dependencies:
  - Issue 326 owner-directed scope
  - historical PR 381 at 2ec4e35a116a051f5841930ef750119458268050
  - merged PHP 8.5 Playwright runtime from PR 477
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-03T08:22:00+02:00
head: 29f3bee63ab4c3ee5847e32101e9000d05f72d0d
branch: audit/OTERYN-20260803-portal-exhaustive-current-main
pr: none
status: investigating
context_routes:
  - agent-governance
  - testing
  - web-cms
  - auth-identity
  - accounts-characters
  - public-game-data
  - admin-rbac
  - api
  - security
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260803-portal-exhaustive-current-main-audit.md
  - docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit/**
  - docs/agents/reports/OTERYN-20260803-portal-exhaustive-current-main-audit*.md
  - tools/audit/portal_exhaustive_audit.py
  - tools/audit/test_portal_exhaustive_audit.py
  - .github/workflows/portal-exhaustive-audit.yml
proven:
  - historical PR 381 audited 27 canonical surface groups 240 named routes 126 rendered screens 43 legacy capabilities and all 18 modules on frozen target b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
  - historical PR 381 final module matrix contained zero policy-v2 complete modules and retained open findings
  - PR 381 exact-head heavy workflows failed because its old frozen base lacked later change-classifier files
  - PR 477 merged a retained PHP 8.5 Playwright runtime and all nine exact-head workflows passed
  - current main contains strict route view navigation and capability ledgers but both explicitly disclaim every-screen every-state and production completeness
  - Issue 326 requires four independent verdicts per auditable record and explicit Wiki and Game Catalog content inventories
derived:
  - a fresh current-main continuation is required because the historical audit branch is not mergeable and cannot provide current exact-head browser proof
unknown:
  - current exact named route rendered view and navigation counts
  - current content completeness of every Wiki article and Game Catalog entity collection
  - current per-surface state browser locale viewport and accessibility closure
  - whether every historical finding remains applicable on current main
conflicts: []
first_failure:
  marker: none yet
  evidence: audit implementation has not run
rejected_hypotheses:
  - route or view existence alone proves functional content or production completeness
  - aggregate 96-test browser smoke proves every screen and state
  - the PHP 8.3 acceptance-runtime blocker still applies after PR 477 merge
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260803-portal-exhaustive-current-main-audit.md
validation:
  - command: live predecessor and dependency preflight
    result: PASS
    evidence: PR 477 and closeout PR 482 merged; PR 381 retained as historical unmergeable audit source; Issue 326 remains open
blockers:
  - none
next_action: inspect current audit validators coverage fragments content inventory sources and existing acceptance workflow contracts before implementing the record-level generator
```

## Notes

The authoritative audit target is the exact head of this task branch after final evidence generation. Historical evidence remains frozen to its recorded SHA and may be reused only with an explicit relationship to current `main`.
