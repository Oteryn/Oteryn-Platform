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

- [x] Existing public-game-data routes, schema assumptions, privacy flags, guild ownership and query-performance boundaries are inventoried before implementation.
- [x] Supported highscore metrics expose explicit categories, vocation and truthful global-scope filtering with bounded pagination without guessing unavailable metrics.
- [x] Public character detail includes every applicable server-backed field with private-by-default account association and status policy.
- [x] Latest deaths and relevant kill statistics use bounded read-only queries and explicit empty, unavailable, restored and not-found states.
- [x] Guild directory/search is delivered with bounded pagination and resilient long-name/member presentation.
- [x] Guild administration is adopted only where a safe owner/leadership contract exists; the current explicit durable read-only policy records that no such contract exists.
- [x] Transfer history is displayed only when an authoritative transfer source exists; it is explicitly not applicable for the current no-world-transfer boundary.
- [x] Polls and public punishment information are excluded through explicit product/privacy policy.
- [x] No private account identifiers, hidden association/status data, moderator-only enforcement data or raw runtime internals are exposed.
- [x] Canary remains read-only and all queries have deterministic limits, index expectations and dependency-failure behavior.
- [x] English/Polish desktop, tablet and mobile UI covers success, empty, validation, unavailable, restored and not-found states.
- [x] Unit, feature, real-MariaDB/query-contract and zero-retry browser evidence cover privacy, filters, pagination, resilience and responsive/accessibility behavior.
- [x] Route and product-completeness ledgers are updated only after exact evidence exists.
- [x] Every required exact-final-head workflow passes before merge.

## Ownership

```yaml
owned_paths:
  - app/PublicGameData/**
  - app/Http/Controllers/PublicGameData/**
  - app/Http/Requests/PublicGameData/**
  - app/CanaryIntegration/CanaryDatabasePrivilegeVerifier.php
  - database/provisioning/canary-readonly.sql.template
  - routes/modules/public-game-statistics.php
  - resources/views/game/**
  - resources/navigation/public/**
  - lang/en/community.php
  - lang/pl/community.php
  - public/css/community.css
  - tests/Unit/CanaryIntegration/CanaryDatabasePrivilegeVerifierTest.php
  - tests/Feature/PublicGameData/**
  - tests/Feature/PublicPortal/PublicPortalExtensionTest.php
  - scripts/acceptance/tests/*community*
  - scripts/acceptance/coverage/**community**
  - docs/architecture/adr/*community*
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/PUBLIC_COMMUNITY_DATA_CONTRACT.md
  - docs/operations/PUBLIC_COMMUNITY_DATA.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/archive/OTERYN-20260729-community-data-completeness.md
modules:
  - PublicGameData
  - Identity
  - Integration
  - Testing
dependencies:
  - current read-only Canary database connection and verified schema contract
  - account-security privacy flags and ready Identity-to-Canary binding
  - current localized public portal and acceptance harness
blockers: []
cross_repository_tasks:
  - none; this task is read-only and did not modify Canary
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T15:31:00Z
head: 45efd2a8f0162df22313e141e973c6a8c3ffb5d1
branch: feat/OTERYN-20260729-community-data-completeness
pr: 298
status: ready
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
  - app/Http/Controllers/PublicGameData/**
  - app/CanaryIntegration/CanaryDatabasePrivilegeVerifier.php
  - database/provisioning/canary-readonly.sql.template
  - routes/modules/public-game-statistics.php
  - resources/views/game/**
  - resources/navigation/public/**
  - lang/{en,pl}/community.php
  - public/css/community.css
  - tests/Feature/{PublicGameData,PublicPortal}/**
  - tests/Unit/CanaryIntegration/CanaryDatabasePrivilegeVerifierTest.php
  - scripts/acceptance/**community**
  - docs/architecture/adr/0018-read-only-community-data-boundary.md
  - docs/contracts/PUBLIC_COMMUNITY_DATA_CONTRACT.md
  - docs/operations/PUBLIC_COMMUNITY_DATA.md
  - docs/testing/{PRODUCT_COMPLETENESS_BENCHMARK.md,product-completeness-benchmark.json,PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md}
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
  - docs/agents/tasks/archive/OTERYN-20260729-community-data-completeness.md
proven:
  - Authoritative Canary source inspection confirmed global players, approved highscore fields, comments, boss points, guild membership/ranks, houses, player_deaths and runtime leases; no authoritative per-channel character ownership, selectable-achievement source or world-transfer history exists.
  - Highscore sort columns are selected only from a fixed server-side allowlist and category/vocation inputs are bounded.
  - Public profile assembly resolves the ready Identity-to-Canary binding server-side and applies private-by-default account-association and status flags before disclosure.
  - Public output excludes Platform Identity email/IDs, Canary account IDs, raw death participant payloads, house coordinates, runtime lease internals and moderator-only enforcement data.
  - Guild directory/search/detail, latest deaths and kill statistics use deterministic pagination, limits and ordering; guild administration remains outside Platform without an approved Canary mutation contract.
  - The dedicated Canary principal remains direct-table SELECT only and explicitly requires houses and player_deaths; schema-wide and write grants fail verification.
  - ADR 0018, the public community data contract and operations runbook record transfer, achievement, polls and public-enforcement policy boundaries.
  - Product and route ledgers were reconciled and their validators passed before merge.
  - Exact final head 45efd2a8f0162df22313e141e973c6a8c3ffb5d1 passed all 11 required workflows: CI 30465570268, Agent Governance 30465570432, Portal Acceptance Contract 30465570230, Acceptance E2E and Visual UX 30465570219, Community Data Acceptance 30465570663, Phase 7 Production-Like Validation 30465570562, Platform DB Outage Validation 30465570777, Edge Security Emulation 30465570280, Game Auth Ticket Concurrency 30465570748, Synology Production Target Preflight 30465570542 and Build Synology Staging Images 30465570799.
  - Community Data Acceptance run 30465570663 passed real MariaDB provisioning, exact direct-table read grants, focused PHP regressions and zero-retry Chromium desktop/tablet/mobile including privacy, dependency failure/recovery and horizontal-overflow checks.
  - Phase 7 run 30465570562 passed synthetic production-like schema provisioning, least-privilege and fail-closed database verification, Redis/SMTP/configuration checks, full regressions, restore and deployment lifecycle evidence.
  - Portal Acceptance run 30465570230 passed the strict machine ledgers and complete zero-retry account lifecycle.
  - PR #298 was squash-merged as 7533b12b1e1c6d266c6bf5a8800e584fad23a01e and Issue #280 closed as completed on 2026-07-29.
derived:
  - Issue #280 is complete for its approved read-only boundary.
  - Remaining required product completeness work includes Issue #277; Issue #278 remains mandatory before commerce activation and Issue #281 owns further server-backed knowledge expansion.
unknown:
  - Actual production MariaDB grants, indexes, latency, deployed code identity and recovery behavior remain unverified.
conflicts: []
first_failure:
  marker: none
  evidence: Every required exact-final-head workflow passed before PR #298 merged.
rejected_hypotheses:
  - Third-party Tibia or wiki schemas can be treated as authoritative for Oteryn.
  - Characters may be filtered by channel when the authoritative player schema stores no channel ownership.
  - A public profile may disclose related characters or status without Platform privacy flags.
  - Guild administration may reuse the generic read-only Canary principal.
  - Missing achievements, transfer history, polls or public enforcement may be guessed or silently represented as delivered.
changed_paths:
  - app/{PublicGameData,Http/Controllers/PublicGameData,CanaryIntegration}/**
  - database/provisioning/canary-readonly.sql.template
  - routes/modules/public-game-statistics.php
  - resources/{views/game,navigation/public}/**
  - lang/{en,pl}/community.php
  - public/css/community.css
  - tests/{Feature/PublicGameData,Feature/PublicPortal,Unit/CanaryIntegration}/**
  - scripts/acceptance/**community**
  - scripts/acceptance/tests/player-journey-acceptance.spec.mjs
  - scripts/acceptance/bootstrap-production-like.sh
  - .github/workflows/{community-data-acceptance,phase7-production-like-validation}.yml
  - docs/{architecture,contracts,operations,testing}/**community**
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
validation:
  - command: node scripts/acceptance/coverage/validate-product-completeness.mjs
    result: PASS
    evidence: Portal Acceptance run 30465570230 validated the updated 43-capability machine ledger at exact final head 45efd2a8f0162df22313e141e973c6a8c3ffb5d1.
  - command: node scripts/acceptance/coverage/validate-portal-coverage.mjs --manifest-only
    result: PASS
    evidence: Portal Acceptance run 30465570230 validated fragment shape, stable evidence markers and unique route ownership at exact final head 45efd2a8f0162df22313e141e973c6a8c3ffb5d1.
  - command: python tools/agents/test_checkpoint.py
    result: PASS
    evidence: Agent Governance run 30465570432 passed at exact final head 45efd2a8f0162df22313e141e973c6a8c3ffb5d1.
  - command: Required exact-final-head workflow suite
    result: PASS
    evidence: All 11 required workflow runs listed under proven completed successfully before merge.
blockers: []
next_action: Continue the benchmark backlog with Issue #277 under a new active task and separate pull request; no further action remains for Issue #280.
```

## Boundaries

This task delivered public read models and presentation only. It did not mutate Canary, expose private Identity or moderator data, invent unavailable server facts, or claim production deployment.
