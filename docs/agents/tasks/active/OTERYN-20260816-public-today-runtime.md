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
  - PR #1114
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
  - app/Localization/LocalizedPublicRouteRegistrar.php
  - app/PublicPortal/Seo/PublicSitemapQuery.php
  - routes/modules/public-portal.php
  - resources/views/public/today/**
  - resources/navigation/public/core.php
  - lang/en/today.php
  - lang/pl/today.php
  - lang/pl.json
  - tests/Feature/PublicPortal/PublicTodayTest.php
  - scripts/acceptance/tests/homepage-navigation-seo.spec.mjs
  - scripts/acceptance/seed-homepage-navigation-seo.php
  - scripts/acceptance/coverage/surfaces/public-today.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/homepage-template-selector.json
  - docs/testing/portal-content-scale-surfaces/public-today.json
  - docs/testing/portal-media-state-surfaces/public-today.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json
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
updated_at: 2026-08-16T17:55:00Z
head: 851b0577a2a166a2bedf703253e897c4ed1d839f
branch: feat/issue-1113-public-today
pr: 1114
status: validating
context_routes:
  - OTERYN_PORTAL_COMPLETION entry 6 public_today
  - PUBLIC_PORTAL_TODAY_ARCHITECTURE first implementation handoff
owned_paths:
  - app/PublicPortal/Today/**
  - app/Http/Controllers/PublicPortal/PublicTodayController.php
  - app/Localization/LocalizedPublicRouteRegistrar.php
  - app/PublicPortal/Seo/PublicSitemapQuery.php
  - routes/modules/public-portal.php
  - resources/views/public/today/**
  - resources/navigation/public/core.php
  - lang/en/today.php
  - lang/pl/today.php
  - lang/pl.json
  - tests/Feature/PublicPortal/PublicTodayTest.php
  - scripts/acceptance/tests/homepage-navigation-seo.spec.mjs
  - scripts/acceptance/seed-homepage-navigation-seo.php
  - scripts/acceptance/coverage/surfaces/public-today.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/homepage-template-selector.json
  - docs/testing/portal-content-scale-surfaces/public-today.json
  - docs/testing/portal-media-state-surfaces/public-today.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json
  - docs/agents/tasks/active/OTERYN-20260816-public-today-runtime.md
proven:
  - protected implementation base main is 286efb1625d510c9d2cc344cb51a2438b31ebe48
  - Issue #1113 and draft PR #1114 durably own the selected bounded slice
  - material implementation head before checkpoint is 851b0577a2a166a2bedf703253e897c4ed1d839f
  - Announcements Upcoming Events and CMS public query providers are consumed through source-owned boundaries
  - no App LiveOps runtime provider exists on the selected base
  - LiveOps is rendered as unavailable with runtime-evidence absent and no fabricated state
  - healthy empty and provider unavailable are separate typed states
  - Today card priority is deterministic and versioned at schema version 1
  - Today response is explicitly no-store and no derived Today cache or index is introduced
  - route localization navigation sitemap route-view coverage content-scale media-applicability and viewport/browser evidence contracts are wired
  - browser acceptance uses the existing zero-retry responsive portability accessibility matrix and acceptance-only bounded failure injection
  - acceptance failure injection header is ignored outside APP_ENV acceptance
  - no external repository access protected operation deployment signer payment or owner-funded AI operation is used
  - open PR #338 remains unrelated and on its external compatibility hold
derived:
  - public_today remains the first safe READY selector entry for this invocation
  - lack of LiveOps authority makes the normal public Today page partial rather than blocking publication or fabricating empty state
unknown:
  - exact-head workflow outcomes for the checkpointed implementation
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - LiveOps must be implemented before Today; focused architecture explicitly allows unavailable/not-yet-provided LiveOps
  - a Today result cache is required for the first slice; architecture explicitly permits no-cache and the implementation uses no-store
changed_paths:
  - app/PublicPortal/Today/TodayCardState.php
  - app/PublicPortal/Today/TodayPageState.php
  - app/PublicPortal/Today/TodayItem.php
  - app/PublicPortal/Today/TodayCard.php
  - app/PublicPortal/Today/TodayPageViewModel.php
  - app/PublicPortal/Today/TodayPageQuery.php
  - app/Http/Controllers/PublicPortal/PublicTodayController.php
  - app/Localization/LocalizedPublicRouteRegistrar.php
  - app/PublicPortal/Seo/PublicSitemapQuery.php
  - routes/modules/public-portal.php
  - resources/views/public/today/index.blade.php
  - resources/navigation/public/core.php
  - lang/en/today.php
  - lang/pl/today.php
  - lang/pl.json
  - tests/Feature/PublicPortal/PublicTodayTest.php
  - scripts/acceptance/tests/homepage-navigation-seo.spec.mjs
  - scripts/acceptance/seed-homepage-navigation-seo.php
  - scripts/acceptance/coverage/surfaces/public-today.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/homepage-template-selector.json
  - docs/testing/portal-content-scale-surfaces/public-today.json
  - docs/testing/portal-media-state-surfaces/public-today.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json
  - docs/agents/tasks/active/OTERYN-20260816-public-today-runtime.md
validation:
  - command: selector and ownership preflight
    result: PASS
    evidence: Issue #1113 PR #1114 plus live main architecture and task ownership state
  - command: whole implementation construction review
    result: PASS
    evidence: source-owned providers explicit LiveOps absence no-store localized route and existing acceptance matrices wired at material head 851b0577a2a166a2bedf703253e897c4ed1d839f
blockers:
  - none
next_action: run exact-head repository governance CI feature integration and zero-retry browser validation; repair every concrete failure before merge
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: implementation is active and exact-head validation is pending
source_branch_evidence: pending
```

## Notes

Runtime/browser E2E is applicable. No production activation, protected-environment operation, external/server-repository access or owner-funded AI operation is authorized.