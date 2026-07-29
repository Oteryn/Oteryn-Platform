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

- [ ] Existing support/legal, RBAC, audit, mail and Identity ownership behavior is inventoried before the schema and transition model are finalized.
- [ ] Authenticated users can create, list and view their own support tickets, reply when allowed and close/reopen only through explicit state transitions.
- [ ] Ticket references are server-generated and browser-supplied identity or owner identifiers never establish authorization.
- [ ] Attachments are explicitly not adopted unless a separate reviewed upload model proves MIME/content/size/storage/privacy safety.
- [ ] Users can submit bounded player, content and guild reports with category-specific target/evidence metadata, pending limits, anti-spam controls and idempotent duplicate handling.
- [ ] Reporters can view their own report history and public-safe outcomes without moderator-private notes or another reporter's identity.
- [ ] Exact-permission, confirmed-MFA administrator queues support ticket replies/status changes, report triage/outcomes and enforcement creation/update with object-level authorization.
- [ ] Account-visible warnings, punishments and rule-violation history expose only approved user-facing reason/status/effective/expiry/acknowledgement/appeal information.
- [ ] Authoritative game-server ban mutation remains outside this Platform-only slice unless a separately proven Canary contract exists.
- [ ] Ticket replies, report outcomes and enforcement entries produce deterministic notification state; notification failure does not corrupt domain state.
- [ ] Every privileged mutation appends bounded audit metadata without ticket bodies, report evidence bodies, moderator notes, credentials or complete network identifiers.
- [ ] Optimistic locking or equivalent deterministic concurrency prevents silent overwrite and duplicate replies/outcomes.
- [ ] Retention and privacy rules are configurable, documented and covered by deterministic pruning/anonymization tests.
- [ ] English/Polish user and administrator UI covers desktop, tablet and mobile success, empty, validation, denied, stale-conflict, rate-limited and unavailable states.
- [ ] Unit, feature, database/concurrency and zero-retry browser tests cover happy path, validation, guest, no-MFA, wrong permission, IDOR, stale state, replay, notification failure, audit and privacy.
- [ ] Product-completeness and route-coverage ledgers are updated only after exact evidence exists.
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
updated_at: 2026-07-28T21:05:00Z
head: 285eb5f89b8f83752fa4d5798bb242136b7b9ae6
branch: feat/OTERYN-20260728-support-moderation-lifecycle
pr: none
status: investigating
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
  - lang/{en,pl}/support.php
  - config/support.php
  - tests/**/Support/**
  - scripts/acceptance/**support*moderation*
  - docs/architecture/adr/*support-moderation*
  - docs/architecture/{MODULE_CATALOG,DATA_OWNERSHIP,SECURITY_ARCHITECTURE}.md
  - docs/operations/SUPPORT_MODERATION_LIFECYCLE.md
  - docs/testing/{PRODUCT_COMPLETENESS_BENCHMARK.md,product-completeness-benchmark.json,PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md}
  - docs/agents/{PROJECT_STATE.md,ACTIVE_WORK.md}
  - docs/agents/tasks/active/OTERYN-20260728-support-moderation-lifecycle.md
proven:
  - Issue #279 is open and requires authenticated tickets, bounded reports, moderator queues, enforcement history, notifications, retention/privacy and responsive browser proof.
  - Current support routes provide only public editorial/support/legal pages and exact-permission support-content administration; no authenticated ticket/report/enforcement workflow exists.
  - No open pull request overlaps Issue #279 intent or the declared Support/Moderation paths.
  - Current main for this task is 285eb5f89b8f83752fa4d5798bb242136b7b9ae6.
  - Canary writes are neither required nor authorized for the Platform-owned ticket/report/enforcement record slice.
derived:
  - The safest complete implementation is an additive Platform-owned module with explicit user-safe and moderator-private fields, exact RBAC/MFA authorization, deterministic transitions and no attachment upload.
unknown:
  - Exact current RBAC permission-registry and audit-recorder extension points still require targeted source inspection.
  - Final retention durations and public reason categories must be selected as conservative configurable defaults and documented as product policy.
conflicts: []
first_failure:
  marker: missing-authenticated-support-domain
  evidence: routes/modules/support.php exposes only static content and support-content CMS administration.
rejected_hypotheses:
  - Static support/legal pages satisfy Issue #279; rejected by the issue acceptance criteria and current route inventory.
  - Platform enforcement records should directly mutate Canary bans; rejected because no separately proven shared-write contract exists.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260728-support-moderation-lifecycle.md
validation:
  - command: repository and PR overlap inspection
    result: PASS
    evidence: no overlapping Issue #279 implementation was found; current support routes contain only editorial content.
blockers: []
next_action: Inspect exact RBAC permission registration, audit recorder, Identity model and existing admin-module conventions, then finalize the additive schema and ADR before runtime implementation.
```

## Boundaries

This task owns Platform support and moderation records only. It does not mutate Canary bans, accept file attachments, expose reporter identity or moderator-private notes, or claim production deployment.
