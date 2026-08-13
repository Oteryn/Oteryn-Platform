---
task_id: OTERYN-20260813-player-companion-session-analyzer-v1
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
search_first:
  - PlayerCompanion
  - SessionAnalysis
  - loot split
optional_reads: []
---

# OTERYN-20260813-player-companion-session-analyzer-v1

## Goal

Deliver the first complete PlayerCompanion vertical slice: an authenticated, owner-private Hunt Session Analyzer v1 that accepts a bounded session log, derives deterministic session/economy metrics, persists normalized analyses without retaining the raw log, supports private history/detail/delete flows, and exposes a responsive EN/PL web UI.

## Acceptance criteria

- [ ] Authenticated users can open the analyzer, submit a supported bounded session log, see deterministic metrics and save the normalized analysis.
- [ ] Raw submitted logs are treated as untrusted input and are not persisted or written to ordinary application logs.
- [ ] Saved analyses are owner-private and cross-owner access returns a safe denial/not-found result.
- [ ] History, detail and deletion journeys are implemented with CSRF-protected state changes.
- [ ] Invalid/unsupported/oversized logs fail with bounded validation errors and no partial persisted record.
- [ ] Parser/domain logic is reusable outside Blade and carries parser/formula version metadata.
- [ ] EN/PL UI copy, loading-independent empty/error/success states, responsive layout and accessible labels are present.
- [ ] Focused unit/feature tests cover parsing, ownership, validation, persistence and deletion.
- [ ] Real browser E2E and exact-final-head repository-required CI pass before merge.
- [ ] Full exact-head self-review passes, related PRs are terminal, task is archived and ownership released after merge.

## Ownership

```yaml
owned_paths:
  - app/PlayerCompanion/**
  - app/Http/Controllers/PlayerCompanion/**
  - app/Http/Requests/PlayerCompanion/**
  - database/migrations/2026_08_13_*player_companion_session_analyses*.php
  - resources/views/player-companion/**
  - routes/modules/player-companion.php
  - tests/Unit/PlayerCompanion/**
  - tests/Feature/PlayerCompanion/**
  - scripts/acceptance/tests/player-companion-session-analyzer.spec.mjs
  - lang/en/player_companion.php
  - lang/pl/player_companion.php
  - docs/agents/tasks/active/OTERYN-20260813-player-companion-session-analyzer-v1.md
  - docs/agents/tasks/archive/OTERYN-20260813-player-companion-session-analyzer-v1.md
modules:
  - PlayerCompanion.SessionAnalysis
dependencies:
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T20:18:00+02:00
head: 05b2642b277530e2a5a79bc302af41496a52e603
branch: feat/player-companion-session-analyzer-v1
pr: none
status: implementing
context_routes:
  - architecture
  - web-cms
  - database
  - testing
owned_paths:
  - app/PlayerCompanion/**
  - app/Http/Controllers/PlayerCompanion/**
  - app/Http/Requests/PlayerCompanion/**
  - database/migrations/2026_08_13_*player_companion_session_analyses*.php
  - resources/views/player-companion/**
  - routes/modules/player-companion.php
  - tests/Unit/PlayerCompanion/**
  - tests/Feature/PlayerCompanion/**
  - scripts/acceptance/tests/player-companion-session-analyzer.spec.mjs
  - lang/en/player_companion.php
  - lang/pl/player_companion.php
  - docs/agents/tasks/active/OTERYN-20260813-player-companion-session-analyzer-v1.md
  - docs/agents/tasks/archive/OTERYN-20260813-player-companion-session-analyzer-v1.md
proven:
  - main at task start is 638df04f616c93d80e33e1abf3f2cf0198163e7a.
  - PlayerCompanion SessionAnalysis is accepted architecture and P0 priority 1.
  - No active task currently owns PlayerCompanion paths.
  - No repository search result showed an existing PlayerCompanion SessionAnalysis implementation to reuse.
  - Initial implementation head 05b2642b277530e2a5a79bc302af41496a52e603 contains normalized persistence, parser, owner-private routes/controllers, EN/PL UI, unit tests and feature tests.
derived:
  - A first v1 can remain Platform-only and avoid Canary/Oteryn-v2 access because session text is user-supplied and analysis is advisory.
unknown:
  - Focused and repository CI results for the implementation head.
  - Exact existing acceptance-test helper pattern to reuse for final browser E2E.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Raw session logs must be persisted to provide history; normalized metrics are sufficient for v1 history.
changed_paths:
  - app/PlayerCompanion/Models/SessionAnalysis.php
  - app/PlayerCompanion/SessionAnalysis/SessionLogParser.php
  - app/Http/Controllers/PlayerCompanion/SessionAnalysisController.php
  - app/Http/Requests/PlayerCompanion/StoreSessionAnalysisRequest.php
  - database/migrations/2026_08_13_201500_create_player_companion_session_analyses_table.php
  - routes/modules/player-companion.php
  - resources/views/player-companion/session-analyses/index.blade.php
  - resources/views/player-companion/session-analyses/show.blade.php
  - lang/en/player_companion.php
  - lang/pl/player_companion.php
  - tests/Unit/PlayerCompanion/SessionLogParserTest.php
  - tests/Feature/PlayerCompanion/SessionAnalysisFeatureTest.php
  - docs/agents/tasks/active/OTERYN-20260813-player-companion-session-analyzer-v1.md
validation:
  - command: focused tests
    result: NOT_RUN
    evidence: remote validation not started yet
blockers:
  - none
next_action: open the draft PR, run focused/repository validation, then repair any exact failures before adding the real browser E2E.
```

## Notes

V1 intentionally does not execute game transfers, access Canary/Oteryn-v2, or infer authoritative game state. Loot split values are advisory deterministic calculations over submitted text.
