---
task_id: OTERYN-20260816-public-today-runtime
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/architecture/PUBLIC_PORTAL_TODAY_ARCHITECTURE.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
search_first:
  - Issue #1113
  - PR #1061
optional_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
---

# OTERYN-20260816-public-today-runtime

## Goal

Deliver Issue #1113 as the first complete `PUBLIC_GUEST` Today vertical slice selected by `OTERYN_PORTAL_COMPLETION` entry 6, using only Platform-owned public query/provider boundaries and explicitly representing absent authoritative LiveOps as unavailable/not-yet-provided.

## Acceptance criteria

- [ ] PublicPortal consumes bounded source-owned public application/query boundaries only.
- [ ] Representation is strictly public guest and does not consult owner-private PlayerCompanion state.
- [ ] Announcements, Events and CMS/news are real providers on the exact base.
- [ ] Missing authoritative LiveOps is explicit unavailable/not-yet-provided with no fabricated current/stale/recovery evidence.
- [ ] Healthy empty differs from provider unavailable; one provider outage produces truthful partial composition.
- [ ] Deterministic priority/card order is covered.
- [ ] Initial Today response is explicitly no-store/no-cache.
- [ ] Public route, canonical/SEO, navigation/sitemap, EN/PL, accessibility and responsive behavior are integrated.
- [ ] Real zero-retry browser E2E covers success, empty, partial dependency failure and absent-LiveOps behavior.
- [ ] Exact-head CI, self-review, review hygiene, merge, Issue closure, archive and source-branch closeout are terminal.

## Ownership

```yaml
owned_paths:
  - app/PublicPortal/Today/**
  - app/Http/Controllers/PublicPortal/PublicTodayController.php
  - resources/views/public/today/**
  - routes/modules/public-portal.php
  - app/PublicPortal/Seo/PublicSitemapQuery.php
  - resources/views/components/public-navigation.blade.php
  - resources/lang/en/**
  - resources/lang/pl/**
  - tests/Feature/PublicPortal/**
  - tests/Unit/PublicPortal/**
  - tests/Browser/**
  - scripts/acceptance/**
  - docs/testing/**
  - docs/agents/tasks/active/OTERYN-20260816-public-today-runtime.md
modules:
  - PublicPortal
dependencies:
  - Issue #1113
  - docs/architecture/PUBLIC_PORTAL_TODAY_ARCHITECTURE.md
  - PR #1061 merged prerequisite
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T17:43:30Z
head: 286efb1625d510c9d2cc344cb51a2438b31ebe48
branch: feat/issue-1113-public-today
pr: none
status: implementing
context_routes:
  - OTERYN_PORTAL_COMPLETION entry 6 public_today
  - PUBLIC_PORTAL_TODAY_ARCHITECTURE first implementation handoff
owned_paths:
  - app/PublicPortal/Today/**
  - app/Http/Controllers/PublicPortal/PublicTodayController.php
  - resources/views/public/today/**
  - routes/modules/public-portal.php
  - app/PublicPortal/Seo/PublicSitemapQuery.php
  - tests/Feature/PublicPortal/**
  - tests/Unit/PublicPortal/**
  - scripts/acceptance/**
  - docs/testing/**
  - docs/agents/tasks/active/OTERYN-20260816-public-today-runtime.md
proven:
  - protected base main is 286efb1625d510c9d2cc344cb51a2438b31ebe48
  - Issue #1113 owns the selected bounded slice
  - Announcements Upcoming Events and CMS public query providers exist on the base
  - no App LiveOps runtime provider exists on the base
  - PR #1061 merged the Announcements Events reverse-edge prerequisite
  - architecture permits public Today with explicit unavailable LiveOps
  - no external repository access is authorized or required
  - initial no-cache policy avoids derived representation authority
  - open PR #338 is unrelated and remains external compatibility hold
  - current active long-lived tasks own public-domain and native-auth production verification only
derived:
  - selector entries before public_today have no safe READY Platform implementation candidate on this base
  - public_today is the first READY candidate
unknown:
  - exact existing acceptance ledger files that must classify the new Today route
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - LiveOps must be implemented before Today; focused architecture explicitly allows unavailable/not-yet-provided LiveOps
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260816-public-today-runtime.md
validation:
  - command: selector and ownership preflight
    result: PASS
    evidence: Issue #1113 plus live main architecture and task ownership state
blockers:
  - none
next_action: implement the complete public Today slice and required acceptance classification on this branch
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: implementation is active
source_branch_evidence: pending
```

## Notes

Runtime/browser E2E is applicable. No production activation, protected-environment operation, external/server-repository access or owner-funded AI operation is authorized.