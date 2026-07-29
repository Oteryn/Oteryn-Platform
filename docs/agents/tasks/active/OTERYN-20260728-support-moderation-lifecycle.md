---
task_id: OTERYN-20260728-support-moderation-lifecycle
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
search_first:
  - docs/agents/tasks/active/** for support, ticket, report, moderation, punishment or enforcement ownership
  - open pull requests for Issue #279 or overlapping support/admin/audit/notification paths
  - existing support/legal routes, RBAC registry, audit recorder and notification conventions
optional_reads:
  - docs/architecture/adr/**support**
  - docs/operations/**RETENTION**
---

# OTERYN-20260728-support-moderation-lifecycle

## Goal

Close Issue #279 with a Platform-owned authenticated support and moderation lifecycle: user tickets, bounded player/content/guild reports, moderator queues, account-visible enforcement history, notifications, retention/privacy controls, exact RBAC/MFA authorization and zero-retry responsive browser evidence.

## Acceptance criteria

- [x] Existing support/legal, RBAC, audit, mail and Identity ownership behavior is inventoried before the schema and transition model are finalized.
- [x] Authenticated users can create, list and view their own support tickets, reply when allowed and close/reopen only through explicit state transitions.
- [x] Ticket references are server-generated and browser-supplied identity or owner identifiers never establish authorization.
- [x] Attachments are explicitly not adopted unless a separate reviewed upload model proves MIME/content/size/storage/privacy safety.
- [x] Users can submit bounded player, content and guild reports with category-specific target/evidence metadata, pending limits, anti-spam controls and idempotent duplicate handling.
- [x] Reporters can view their own report history and public-safe outcomes without moderator-private notes or another reporter's identity.
- [x] Exact-permission, confirmed-MFA administrator queues support ticket replies/status changes, report triage/outcomes and enforcement creation/update with object-level authorization.
- [x] Account-visible warnings, punishments and rule-violation history expose only approved user-facing reason/status/effective/expiry/acknowledgement/appeal information.
- [x] Authoritative game-server ban mutation remains outside this Platform-only slice unless a separately proven Canary contract exists.
- [x] Ticket replies, report outcomes and enforcement entries produce deterministic notification state; notification failure does not corrupt domain state.
- [x] Every privileged mutation appends bounded audit metadata without ticket bodies, report evidence bodies, moderator notes, credentials or complete network identifiers.
- [x] Optimistic locking or equivalent deterministic concurrency prevents silent overwrite and duplicate replies/outcomes.
- [x] Retention and privacy rules are configurable, documented and covered by deterministic pruning/anonymization tests.
- [x] English/Polish user and administrator UI covers desktop, tablet and mobile success, empty, validation, denied, stale-conflict, rate-limited and unavailable states.
- [x] Unit, feature, database/concurrency and zero-retry browser tests cover happy path, validation, guest, no-MFA, wrong permission, IDOR, stale state, replay, notification failure, audit and privacy.
- [x] Product-completeness and route-coverage ledgers are updated only after exact evidence exists.
- [x] Every required exact-final-head workflow passes before merge.

## Ownership

```yaml
owned_paths:
  - app/Support/**
  - app/Moderation/**
  - app/Notifications/Support/**
  - app/Http/Controllers/Support/**
  - app/Http/Controllers/Admin/**Support**
  - app/Http/Controllers/Admin/**Moderation**
  - app/Http/Requests/Support/**
  - app/Http/Requests/Admin/**Support**
  - app/Http/Requests/Admin/**Moderation**
  - database/migrations/*support*
  - database/migrations/*report*
  - database/migrations/*enforcement*
  - routes/modules/support.php
  - resources/views/support/**
  - resources/views/admin/support/**
  - resources/views/admin/moderation/**
  - lang/en/support.php
  - lang/pl/support.php
  - config/support.php
  - tests/Unit/Support/**
  - tests/Feature/Support/**
  - tests/Integration/Support/**
  - scripts/acceptance/tests/*support*moderation*
  - scripts/acceptance/coverage/**support**
  - docs/architecture/adr/*support-moderation*
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/operations/SUPPORT_MODERATION_LIFECYCLE.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260728-support-moderation-lifecycle.md
modules:
  - Support
  - Moderation
  - Admin
  - Audit
  - Notifications
  - Testing
dependencies:
  - current Platform Identity and registered-session ownership
  - current exact-permission RBAC and confirmed-MFA administrator boundary
  - current security/admin audit recorders
  - current SMTP test infrastructure
blockers: []
cross_repository_tasks:
  - none; Canary remains read-only and authoritative game-ban mutation is excluded from this Platform-only task
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T08:36:00Z
head: e03fe5f9bf6954e2e6382590e4bb3ea991e30f85
branch: feat/OTERYN-20260728-support-moderation-lifecycle
pr: 293
status: ready
context_routes:
  - agent-governance
  - architecture
  - auth-identity
  - admin-rbac
  - database
  - security
  - web-cms
  - testing
owned_paths:
  - app/Support/**
  - app/Notifications/Support/**
  - app/Console/Commands/PruneSupportRetention.php
  - database/migrations/*support*
  - routes/modules/support.php
  - resources/views/{support,admin/support,admin/moderation}/**
  - lang/{en,pl}/support.php
  - config/support.php
  - tests/Feature/Support/**
  - scripts/acceptance/**support*moderation*
  - docs/architecture/adr/*support-moderation*
  - docs/architecture/{MODULE_CATALOG,DATA_OWNERSHIP,SECURITY_ARCHITECTURE}.md
  - docs/operations/SUPPORT_MODERATION_LIFECYCLE.md
  - docs/testing/{PRODUCT_COMPLETENESS_BENCHMARK.md,product-completeness-benchmark.json,PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md}
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
  - docs/agents/tasks/active/OTERYN-20260728-support-moderation-lifecycle.md
proven:
  - Exact documentation head e03fe5f9bf6954e2e6382590e4bb3ea991e30f85 is based on main c1edc2cc1f5f8298a10582352dddb177bbaa58b3 and preserves the merged Game Catalog.
  - Authenticated owner-scoped tickets, bounded reports, Platform enforcement/appeals, notification delivery state and retention are implemented with additive Platform-owned schema.
  - User IDOR boundaries, moderator-private fields, exact support permissions, confirmed MFA, optimistic locking, bounded audit metadata and no-Canary-write behavior are covered by feature tests.
  - CI run 30435418799 passed Composer validation/audit, formatting, PHPStan and the full PHP test suite at the exact documentation head.
  - Support Moderation Acceptance run 30435418795 passed focused regressions and six zero-retry Chromium scenarios across desktop, tablet and mobile.
  - Portal Acceptance Contract run 30435418852 passed the complete zero-retry account lifecycle plus strict route and product-ledger closure.
  - Acceptance E2E and Visual UX run 30435418753 passed; Phase 7 run 30435418895 passed.
  - Agent Governance run 30435418768, Edge Security run 30435418764, DB Outage run 30435418736, Game Auth concurrency run 30435418743, Synology Preflight run 30435418746, Support Legal run 30435418763 and Build Images run 30435418728 passed.
  - Trust boundary: browser input never establishes ownership or moderator authority; authenticated Platform Identity ownership and exact permission plus confirmed MFA are authoritative.
  - Canary/login-server schema, sessions and native enforcement remain unchanged; Canary is read-only for this lifecycle.
  - The additive Platform migration has no Canary rollback requirement; dropping live Platform support tables requires retention and data review.
  - No secret, production credential, production-only configuration or personal-data artifact is committed.
derived:
  - Issue #279 satisfies its approved Platform-only scope and is ready to merge.
  - Product completeness remains false because required character/profile and community-data gaps #277 and #280 remain.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: Every required exact-head workflow listed in validation passed at e03fe5f9bf6954e2e6382590e4bb3ea991e30f85.
rejected_hypotheses:
  - Static support/legal content satisfies Issue #279: rejected by the authenticated lifecycle and issue contract.
  - Platform enforcement should mutate Canary bans: rejected because no approved cross-repository mutation contract exists.
  - Browser acceptance should raise production login limits: rejected; isolated tests clear only acceptance cache while production limits remain unchanged.
changed_paths:
  - app/Support/**
  - app/Notifications/Support/**
  - app/Console/Commands/PruneSupportRetention.php
  - app/Providers/AppServiceProvider.php
  - app/Admin/AdminPermission.php
  - config/support.php
  - database/migrations/2026_07_28_211000_create_support_moderation_tables.php
  - routes/modules/support.php
  - resources/views/support/**
  - resources/views/admin/{support,moderation}/**
  - lang/{en,pl}/support.php
  - public/css/support.css
  - tests/Feature/Support/SupportModerationLifecycleTest.php
  - scripts/acceptance/**support*moderation*
  - docs/architecture/**support*moderation*
  - docs/operations/SUPPORT_MODERATION_LIFECYCLE.md
  - docs/testing/{PRODUCT_COMPLETENESS_BENCHMARK.md,product-completeness-benchmark.json,PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md}
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
validation:
  - command: CI run 30435418799
    result: PASS
    evidence: Composer metadata/audit, Pint, PHPStan and full PHP tests passed at e03fe5f9bf6954e2e6382590e4bb3ea991e30f85.
  - command: Support Moderation Acceptance run 30435418795
    result: PASS
    evidence: Focused PHP regressions and zero-retry desktop/tablet/mobile Chromium passed.
  - command: Portal Acceptance Contract run 30435418852
    result: PASS
    evidence: Complete account lifecycle and strict route/product ledgers passed.
  - command: Acceptance E2E and Visual UX run 30435418753
    result: PASS
    evidence: Functional, responsive, visual and accessibility acceptance passed.
  - command: Remaining required exact-head workflows
    result: PASS
    evidence: Phase7 30435418895; Governance 30435418768; Edge 30435418764; DB outage 30435418736; Game Auth 30435418743; Synology 30435418746; Support Legal 30435418763; Build 30435418728.
blockers: []
next_action: Mark PR #293 ready and squash-merge it.
```

## Boundaries

This task owns Platform support and moderation records only. It does not mutate Canary bans, accept file attachments, expose reporter identity or moderator-private notes, or claim production deployment.
