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
- [ ] Every required exact-final-head workflow passes before merge.

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
updated_at: 2026-07-29T08:05:00Z
head: 30625312410dcb4c6e19f8eff7caa362fb55200d
branch: feat/OTERYN-20260728-support-moderation-lifecycle
pr: 293
status: validating
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
  - Functional head 30625312410dcb4c6e19f8eff7caa362fb55200d is a clean one-commit diff from current main c1edc2cc1f5f8298a10582352dddb177bbaa58b3 and preserves the merged Game Catalog.
  - Authenticated owner-scoped tickets, bounded reports, Platform enforcement/appeals, notification delivery state and retention are implemented with additive Platform-owned schema.
  - User IDOR boundaries, moderator-private fields, exact support permissions, confirmed MFA, optimistic locking, bounded audit metadata and no-Canary-write behavior are covered by feature tests.
  - Support Moderation Acceptance run 30433007335 passed focused regressions and six zero-retry Chromium scenarios across desktop, tablet and mobile at the functional head.
  - CI run 30433007344 passed Composer validation/audit, formatting, PHPStan and the full PHP test suite at the functional head.
  - Phase 7 run 30433007337, Game Auth concurrency 30433007355, Agent Governance 30433007362, DB Outage 30433007369 and Synology Preflight 30433007342 passed at the functional head.
  - Trust boundary: browsers and ordinary users cannot establish ownership or moderator authority; Platform Identity ownership and exact permission plus confirmed MFA are authoritative.
  - Canary/login-server schema and session compatibility are unchanged; Canary remains read-only and no native ban/account-status mutation is introduced.
  - The additive Platform migration has no destructive rollback requirement for Canary; rollback of live support rows requires retention/data review before dropping Platform tables.
  - No secret, production credential, production-only configuration or personal-data artifact is committed.
derived:
  - Issue #279 is functionally satisfied inside the approved Platform-only boundary, subject to final documentation-head workflow closure and merge.
  - Product completeness remains false because required character/profile and community-data gaps #277 and #280 remain.
unknown:
  - Final workflow conclusions for the documentation reconciliation head are not yet available.
conflicts: []
first_failure:
  marker: none
  evidence: The latest functional exact-head CI and focused support/moderation lifecycle both passed.
rejected_hypotheses:
  - Static support/legal content satisfies Issue #279: rejected by the implemented authenticated lifecycle and issue acceptance contract.
  - Platform enforcement should mutate Canary bans: rejected because no approved cross-repository mutation contract exists.
  - Browser acceptance should raise production login limits: rejected; independent acceptance scenarios clear only isolated test cache while production rate limits remain unchanged.
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
  - command: CI run 30433007344
    result: PASS
    evidence: Composer metadata/audit, Pint, PHPStan and full PHP tests passed at 30625312410dcb4c6e19f8eff7caa362fb55200d.
  - command: Support Moderation Acceptance run 30433007335
    result: PASS
    evidence: Focused PHP regressions and zero-retry desktop/tablet/mobile Chromium matrix passed at 30625312410dcb4c6e19f8eff7caa362fb55200d.
  - command: Product and route ledger validation after reconciliation
    result: NOT_RUN
    evidence: Run in the documentation reconciliation commit before push.
blockers: []
next_action: Validate every required workflow on the documentation reconciliation head, then mark PR #293 ready and merge if all remain green.
```

## Boundaries

This task owns Platform support and moderation records only. It does not mutate Canary bans, accept file attachments, expose reporter identity or moderator-private notes, or claim production deployment.
