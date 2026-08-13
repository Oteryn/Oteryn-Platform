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
  - resources/views/identity/account/overview.blade.php
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
pr: 1028
status: validating
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
  - resources/views/identity/account/overview.blade.php
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
  - No active task at claim time owned PlayerCompanion paths.
  - No repository search result showed an existing PlayerCompanion SessionAnalysis implementation to reuse.
  - PR 1028 is the sole delivery path for this task.
  - Implementation includes normalized persistence, parser/formula versioning, private owner-scoped routes, raw-log non-retention, EN/PL UI, Account Center discoverability, bounded validation, focused tests and a zero-retry browser journey.
  - Parser hardening rejects duplicate participants, more than 20 participants, decimal/ambiguous metrics and partial participant aggregates presented as complete totals.
derived:
  - V1 remains Platform-only and avoids Canary/Oteryn-v2 access because session text is user-supplied and analysis is advisory.
unknown:
  - Exact-head CI and browser E2E result for the latest implementation generation.
conflicts: []
first_failure:
  marker: checkpoint-validation
  evidence: The first workflow generation rejected an invalid checkpoint validation result before runtime tests.
rejected_hypotheses:
  - Raw session logs must be persisted to provide history; normalized metrics are sufficient for v1 history.
  - A route reachable only by manually entering its URL is sufficient product discoverability; the Account Center now links to the analyzer.
changed_paths:
  - app/PlayerCompanion/Models/SessionAnalysis.php
  - app/PlayerCompanion/SessionAnalysis/SessionLogParser.php
  - app/Http/Controllers/PlayerCompanion/SessionAnalysisController.php
  - app/Http/Requests/PlayerCompanion/StoreSessionAnalysisRequest.php
  - database/migrations/2026_08_13_201500_create_player_companion_session_analyses_table.php
  - routes/modules/player-companion.php
  - resources/views/player-companion/session-analyses/index.blade.php
  - resources/views/player-companion/session-analyses/show.blade.php
  - resources/views/identity/account/overview.blade.php
  - lang/en/player_companion.php
  - lang/pl/player_companion.php
  - tests/Unit/PlayerCompanion/SessionLogParserTest.php
  - tests/Feature/PlayerCompanion/SessionAnalysisFeatureTest.php
  - scripts/acceptance/tests/player-companion-session-analyzer.spec.mjs
  - docs/agents/tasks/active/OTERYN-20260813-player-companion-session-analyzer-v1.md
validation:
  - command: repository-selected PR workflows
    result: NOT_RUN
    evidence: The first generation stopped at checkpoint validation; a corrected generation is required.
blockers:
  - none
next_action: validate the corrected exact head, repair only proven failures, then perform full-diff self-review and merge only when every required gate is green.
```

## Notes

V1 intentionally does not execute game transfers, access Canary/Oteryn-v2, or infer authoritative game state. Loot split values are advisory deterministic calculations over submitted text.
