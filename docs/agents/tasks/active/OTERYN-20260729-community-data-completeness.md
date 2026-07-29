---
task_id: OTERYN-20260729-community-data-completeness
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
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
  - docs/agents/tasks/active/** for public game data, community, guild, highscores, deaths or profile ownership
  - open pull requests for Issue #280 or overlapping public-game-data routes/repositories/views
  - current Canary read model, schema fixtures, indexes, privacy controls and portal coverage fragments
optional_reads:
  - docs/architecture/adr/**public**
  - docs/contracts/**CANARY**
---

# OTERYN-20260729-community-data-completeness

## Goal

Close Issue #280 with bounded read-only community data: supported highscore categories and filters, privacy-aware rich character profiles, latest deaths and kill statistics, guild directory/search and explicitly scoped guild workflows, localized failure states, performance boundaries and zero-retry responsive browser evidence.

## Acceptance criteria

- [ ] Existing public-game-data routes, schema assumptions, privacy flags, guild ownership and query-performance boundaries are inventoried before implementation.
- [ ] Supported highscore metrics expose explicit categories, vocation/world/channel filters and bounded pagination without guessing unavailable metrics.
- [ ] Public character detail includes every applicable server-backed field with private-by-default account association and status policy.
- [ ] Latest deaths and relevant kill statistics use bounded read-only queries and explicit empty, stale, unavailable and not-found states.
- [ ] Guild directory/search is delivered with bounded pagination and resilient long-name/member presentation.
- [ ] Guild administration is adopted only where a safe owner/leadership contract exists; otherwise an explicit durable read-only policy is recorded.
- [ ] Transfer history is displayed only when an authoritative transfer source exists; otherwise it is explicitly not applicable for the current no-world-transfer boundary.
- [ ] Polls and public punishment information are either delivered or excluded through explicit product/privacy policy.
- [ ] No private account identifiers, hidden association/status data, moderator-only enforcement data or raw runtime internals are exposed.
- [ ] Canary remains read-only and all queries have deterministic limits, index expectations and dependency-failure behavior.
- [ ] English/Polish desktop, tablet and mobile UI covers success, empty, validation, unavailable, restored and not-found states.
- [ ] Unit, feature, real-MariaDB/query-contract and zero-retry browser evidence cover authorization/privacy, filters, pagination, resilience and responsive/accessibility behavior.
- [ ] Route and product-completeness ledgers are updated only after exact evidence exists.
- [ ] Every required exact-final-head workflow passes before merge.

## Ownership

```yaml
owned_paths:
  - app/PublicGameData/**
  - app/Http/Controllers/PublicGameData/**
  - app/Http/Requests/PublicGameData/**
  - routes/modules/public-portal.php
  - resources/views/game/**
  - resources/navigation/public/**
  - lang/en/game.php
  - lang/pl/game.php
  - public/css/**community**
  - tests/Unit/PublicGameData/**
  - tests/Feature/PublicGameData/**
  - tests/Integration/PublicGameData/**
  - scripts/acceptance/tests/*community*
  - scripts/acceptance/coverage/**community**
  - docs/architecture/adr/*community*
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/operations/PUBLIC_COMMUNITY_DATA.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260729-community-data-completeness.md
modules:
  - PublicGameData
  - Identity
  - Guilds
  - Testing
dependencies:
  - current read-only Canary database connection and schema contract
  - account-security privacy flags
  - current localized public portal and acceptance harness
blockers: []
cross_repository_tasks:
  - none; this task is read-only and does not modify Canary
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T09:10:00Z
head: 2d7c02fb60e52e93ee23e5b84de16cde3b862469
branch: feat/OTERYN-20260729-community-data-completeness
pr: none
status: investigating
context_routes:
  - agent-governance
  - architecture
  - canary-integration
  - database
  - auth-identity
  - web-cms
  - security
  - testing
owned_paths:
  - app/PublicGameData/**
  - routes/modules/public-portal.php
  - resources/views/game/**
  - lang/{en,pl}/game.php
  - tests/{Unit,Feature,Integration}/PublicGameData/**
  - scripts/acceptance/**community**
  - docs/architecture/adr/*community*
  - docs/operations/PUBLIC_COMMUNITY_DATA.md
  - docs/testing/{PRODUCT_COMPLETENESS_BENCHMARK.md,product-completeness-benchmark.json,PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md}
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
  - docs/agents/tasks/active/OTERYN-20260729-community-data-completeness.md
proven:
  - Issue #280 is open and requires broader highscores, rich profiles, deaths/kill statistics and guild directory/workflow policy.
  - Current delivered contract has level-only highscores, basic character detail, one-guild detail, online players and server status.
  - Canary writes are neither required nor authorized for this read-only task.
derived:
  - The safest complete slice extends the bounded read-only repository and public views while consuming existing Platform privacy flags server-side.
unknown:
  - Exact current Canary table/column/index availability for deaths, houses, achievements, guild ranks and supported highscore metrics requires source and schema inspection.
  - Whether guild administration, polls or public punishment presentation have an approved product/ownership policy is not yet established.
conflicts: []
first_failure:
  marker: incomplete-community-read-model
  evidence: Issue #280 and the benchmark classify highscore filters, rich profiles, deaths/statistics and guild directory/workflows as partial or missing.
rejected_hypotheses:
  - Third-party Tibia or wiki schemas can be treated as authoritative for Oteryn.
  - A working level-highscore route proves broader community-data completeness.
  - Optional polls or public punishment data may be exposed without an explicit product/privacy policy.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260729-community-data-completeness.md
validation:
  - command: repository and overlap inventory
    result: NOT_RUN
    evidence: Pending exact source, schema, task and PR inspection.
blockers: []
next_action: Inspect the current PublicGameData repository, route/controller/view contract, Canary schema fixtures and privacy flags, then define the exact read-only delivery and exclusion policy before implementation.
```

## Boundaries

This task owns public read models and presentation only. It does not mutate Canary, expose private Identity or moderator data, invent unavailable server facts, or claim production deployment.
