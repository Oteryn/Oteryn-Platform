---
task_id: OTERYN-20260724-announcements-events
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/contracts/PUBLIC_PORTAL_EXTENSION_CONTRACT.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
search_first:
  - docs/agents/tasks/active/**
  - open pull requests affecting announcements, events, CMS, RBAC, audit, module routes or localization
  - existing CMS/Admin/RBAC/Audit services, requests, controllers and tests
  - existing date/time and localization conventions
optional_reads: []
---

# OTERYN-20260724-announcements-events

## Goal

Deliver isolated, audited Announcements and Events modules with deterministic UTC scheduling, exact authorization, safe localized public content, module-local routes and reusable homepage integration providers/components without modifying homepage or shared navigation/footer implementation files.

## Acceptance criteria

- [x] Announcements support title, body, severity, publication state, start/end boundaries and validated internal or approved external action links.
- [x] Public announcement queries expose only active approved records and ticker boundary behavior is deterministic and tested.
- [x] Announcement administration requires `auth`, confirmed MFA and `portal.announcements.manage`; mutations are audited with bounded metadata and stale edits fail explicitly.
- [x] Events support localized title, slug, summary and safe body, UTC start/end, featured flag, optional news relation and draft/scheduled/active/completed/cancelled states.
- [x] Public `/events` and `/events/{slug}` expose only approved public records, including upcoming, archived, cancelled and empty states as specified.
- [x] Event administration requires `auth`, confirmed MFA and exact `events.manage` / `events.publish` permissions; mutations are audited with bounded metadata and stale edits fail explicitly.
- [x] Localized slug uniqueness and deterministic timezone behavior are enforced and tested.
- [x] Reusable ticker and upcoming-event providers/components exist for later homepage integration without modifying homepage files.
- [x] No raw HTML or image upload is introduced.
- [x] Focused tests and the full required CI set pass on the exact implementation head.

## Ownership

```yaml
owned_paths:
  - app/Announcements/**
  - app/Events/**
  - database/migrations/*site_announcements*.php
  - database/migrations/*events*.php
  - resources/views/announcements/**
  - resources/views/events/**
  - resources/views/admin/announcements/**
  - resources/views/admin/events/**
  - resources/navigation/public/events.php
  - routes/modules/announcements.php
  - routes/modules/events.php
  - tests/Feature/Announcements/**
  - tests/Feature/Events/**
  - tests/Feature/PublicPortal/PublicPortalExtensionTest.php
  - docs/agents/tasks/active/OTERYN-20260724-announcements-events.md
modules:
  - Announcements
  - Events
  - AdminRBAC
  - Audit
  - PublicPortal
  - CMS
dependencies:
  - PR #146 public-web foundation merged
  - exact permissions reserved centrally
  - module-local route and navigation loading available
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T10:01:20+02:00
head: d9d70ca9cc05a47dde900db23e218b4057d77930
branch: feat/OTERYN-20260724-announcements-events
pr: 157
status: ready
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - admin-rbac
  - database
  - security
  - testing
owned_paths:
  - app/Announcements/**
  - app/Events/**
  - database/migrations/*site_announcements*.php
  - database/migrations/*events*.php
  - resources/views/announcements/**
  - resources/views/events/**
  - resources/views/admin/announcements/**
  - resources/views/admin/events/**
  - resources/navigation/public/events.php
  - routes/modules/announcements.php
  - routes/modules/events.php
  - tests/Feature/Announcements/**
  - tests/Feature/Events/**
  - tests/Feature/PublicPortal/PublicPortalExtensionTest.php
  - docs/agents/tasks/active/OTERYN-20260724-announcements-events.md
proven:
  - Announcements persist plain-text title and body, bounded severity, publication state, inclusive UTC start and exclusive UTC end boundaries, and tightly validated internal or HTTPS action links.
  - ActiveAnnouncementQuery returns only published records inside the explicit publication window and AnnouncementTickerProvider exposes AVAILABLE, EMPTY and UNAVAILABLE states.
  - Announcement administration is protected by authentication, confirmed MFA and the exact `portal.announcements.manage` permission; writes are transactional, audited with bounded metadata and protected by row locking plus lock-version conflicts.
  - Events persist explicit UTC start/end values, featured state, optional news relation and localized EN/PL translations with per-locale unique slugs.
  - Event content saves always return the record to draft, while publication transitions require the separate exact `events.publish` permission in addition to `events.manage`.
  - Public event queries classify scheduled records deterministically as active, upcoming or completed from UTC boundaries, preserve cancelled records, exclude drafts and expose only the requested locale.
  - Event detail links related news only when that news post is itself published at the read time.
  - All public announcement and event output is escaped plain text; no raw-HTML or image-upload surface was added.
  - Reusable AnnouncementTickerProvider and UpcomingEventProvider plus module-local Blade components are available for later homepage integration; homepage files were not changed.
  - Main was merged into the task branch at d9d70ca9cc05a47dde900db23e218b4057d77930, preserving the completed Support navigation entries and adding Events by module priority.
  - PR #157 is mergeable against current main and its implementation diff is limited to owned module paths, the public-navigation contract test and this task record.
derived:
  - Event schedules remain authoritative structured data and are never inferred from free-form news content.
  - Editing previously approved event content revokes public approval until an authorized publisher explicitly transitions it again.
  - Provider-level dependency failures remain distinguishable from legitimate empty states for later homepage integration.
unknown: []
conflicts:
  - path: tests/Feature/PublicPortal/PublicPortalExtensionTest.php
    resolution: merged current main Support expectations with the registered Events navigation item; no shared navigation implementation file was edited
first_failure:
  marker: full PHPUnit regression after static analysis passed
  evidence: PublicPortalExtensionTest expected the pre-Events header list; the contract expectation was updated to include all registered current-main items and Events
rejected_hypotheses:
  - PHPStan failure was not caused by domain query generics after explicit database scalar normalization; remaining failures were unsupported Faker calls and Carbon null handling.
  - The full-suite failure was not an announcements/events behavior failure; both focused module suites passed before the navigation contract mismatch was isolated.
changed_paths:
  - app/Announcements/**
  - app/Events/**
  - database/migrations/2026_07_24_211000_create_site_announcements_table.php
  - database/migrations/2026_07_24_211100_create_events_and_event_translations_tables.php
  - resources/navigation/public/events.php
  - resources/views/announcements/**
  - resources/views/events/**
  - resources/views/admin/announcements/**
  - resources/views/admin/events/**
  - routes/modules/announcements.php
  - routes/modules/events.php
  - tests/Feature/Announcements/AnnouncementsModuleTest.php
  - tests/Feature/Events/EventsModuleTest.php
  - tests/Feature/PublicPortal/PublicPortalExtensionTest.php
  - docs/agents/tasks/active/OTERYN-20260724-announcements-events.md
validation:
  - command: focused Announcements and Events PHPUnit suites
    result: PASS
    evidence: temporary diagnostic run 30149842397 completed successfully; diagnostic PR #172 was closed without merge
  - command: GitHub CI run 30150181078 on d9d70ca9cc05a47dde900db23e218b4057d77930
    result: PASS
    evidence: Composer validation, dependency audit, Pint, PHPStan level 10 and the full PHPUnit suite passed
  - command: Agent Governance run 30150181098
    result: PASS
    evidence: exact implementation head passed repository checkpoint and ownership validation
  - command: Platform DB Outage Validation run 30150181061
    result: PASS
    evidence: fail-closed mutation and recovery validation passed
  - command: Game Auth Ticket Concurrency run 30150181111
    result: PASS
    evidence: independent-process MariaDB concurrency proof passed
  - command: Phase 7 Production-Like Validation run 30150181063
    result: PASS
    evidence: production-like deployment, migration, privilege, recovery and regression validation passed
  - command: Build Synology Staging Images run 30150181064
    result: PASS
    evidence: staging image build passed on the exact implementation head
  - command: Acceptance E2E and Visual UX run 30150181132
    result: PASS
    evidence: exact-SHA smoke, browser portability, responsive, dependency resilience and keyboard accessibility profiles passed
blockers:
  - none
next_action: Verify the required checks on the final task-record-only head, then mark PR #157 ready for review.
```

## Notes

Trust boundaries affected: public publication filtering, privileged CMS-like mutations, exact permission authorization, MFA enforcement and bounded audit logging. Platform-owned schema only; no Canary/login-server compatibility change, secrets or production-only configuration is involved. Migrations are reversible and stale-write behavior is deterministic.
