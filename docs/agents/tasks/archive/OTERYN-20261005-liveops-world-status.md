---
task_id: OTERYN-20261005-liveops-world-status
governing_issue: 1458
required_reads:
  - docs/architecture/LIVEOPS_ARCHITECTURE.md
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
search_first:
  - app/GameAuth/NativeRuntimeStatus
  - app/PublicPortal/Today
  - tests/Feature/GameAuth/NativeRuntimeStatus
  - tests/Feature/PublicPortal
optional_reads:
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/architecture/PUBLIC_PORTAL_TODAY_ARCHITECTURE.md
---

# OTERYN-20261005-liveops-world-status

## Goal

Issue #1458. Deliver the first executable LiveOps slice: public-safe native WorldStatus plus configured maintenance into the already delivered public Today composition.

The former selector blocker is resolved on protected main `46ed77d5799a8c0019e9fecf9178aafe13584115`: canonical native WorldId/ChannelId, scope-assignment ownership fencing, Game-authored runtime-status ingestion, freshness classification and a Platform read model are integrated. This task consumes those Platform-side boundaries only; it does not invent or mutate Game authority.

## Acceptance criteria

- [x] PublicPortal consumes an App\LiveOps query boundary rather than raw runtime tables.
- [x] Runtime public evidence omits GameNode identity, assignment/fencing generations, route revisions and private endpoints.
- [x] Configured maintenance/login policy remains distinct from observed runtime readiness.
- [x] Fresh ready/not-ready, mixed/degraded, stale, unavailable and invalid evidence remain distinct; none fabricates whole-world offline.
- [x] Canonical WorldId/ChannelId validation is preserved.
- [x] EN/PL Today presentation uses public-safe state labels and keeps partial evidence explicit.
- [x] Focused feature tests cover fresh, maintenance, stale/unavailable/invalid, mixed-channel degraded, recovery and public redaction.
- [x] Zero-retry browser acceptance covers the delivered public route on exact candidate.
- [x] Exact-head required CI is green.
- [x] Independent exact-head review has no open material finding.

## Ownership

```yaml
owned_paths:
  - app/LiveOps/WorldStatus/**
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimePublicEvidence.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusReadModel.php
  - app/PublicPortal/Today/TodayPageQuery.php
  - app/PublicPortal/Today/TodayCardState.php
  - resources/views/public/today/index.blade.php
  - lang/en/today.php
  - lang/pl/today.php
  - tests/Feature/LiveOps/**
  - scripts/acceptance/coverage/surfaces/public-today.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/homepage-template-selector.json
  - scripts/acceptance/seed-homepage-navigation-seo.php
  - scripts/acceptance/set-liveops-state.php
  - scripts/acceptance/tests/homepage-navigation-seo.spec.mjs
  - config/game-auth.php
  - docs/architecture/MODULE_CATALOG.md
  - docs/agents/tasks/active/OTERYN-20261005-liveops-world-status.md
modules:
  - LiveOps
  - PublicPortal/Today
dependencies:
  - Issue #1458
  - accepted LIVEOPS_ARCHITECTURE
  - integrated native runtime-status producer consumer boundary on Platform main
blockers: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T19:29:00Z
status: completed
phase: terminal
branch: feat/liveops-world-status-1458
head: a41b2210e3c433176f3843be5758d22def63386d
pr: 1459
context_routes:
  - api
  - testing
  - security
owned_paths:
  - app/LiveOps/WorldStatus/**
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimePublicEvidence.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusReadModel.php
  - app/PublicPortal/Today/TodayPageQuery.php
  - app/PublicPortal/Today/TodayCardState.php
  - resources/views/public/today/index.blade.php
  - lang/en/today.php
  - lang/pl/today.php
  - tests/Feature/LiveOps/**
  - scripts/acceptance/coverage/surfaces/public-today.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/homepage-template-selector.json
  - scripts/acceptance/seed-homepage-navigation-seo.php
  - scripts/acceptance/set-liveops-state.php
  - scripts/acceptance/tests/homepage-navigation-seo.spec.mjs
  - config/game-auth.php
  - docs/architecture/MODULE_CATALOG.md
  - docs/agents/tasks/active/OTERYN-20261005-liveops-world-status.md
proven:
  - Protected main 46ed77d5799a8c0019e9fecf9178aafe13584115 contains native runtime-status ingestion/read-model and canonical topology required by the accepted LiveOps first-slice gate.
  - Public Today is already delivered and explicitly carries LiveOps as unavailable until an App\LiveOps provider exists.
derived:
  - A bounded Platform-only WorldStatus projection can now be implemented without Game mutation or new Game semantics.
unknown:
  - Production runtime-status activation/configuration; this task makes no production claim.
conflicts: []
first_failure:
  marker: phase7_exact_sha_regression_fixture_duplicate_assignment
  evidence: superseded exact-head run 37362795927 / job 111941882180 failed PublicWorldStatusQueryTest because runtime() inserted the same native_scope_assignments (world_id, channel_id) twice; repaired with updateOrInsert on functional head cfef690a56982dc8dceddef1416e52b3086cc688
rejected_hypotheses:
  - Treating stale or unavailable runtime evidence as offline.
  - Reading native runtime tables directly from PublicPortal.
changed_paths:
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimePublicEvidence.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusReadModel.php
  - app/LiveOps/WorldStatus/PublicWorldStatus.php
  - app/LiveOps/WorldStatus/PublicWorldStatusQuery.php
  - app/PublicPortal/Today/TodayCardState.php
  - app/PublicPortal/Today/TodayPageQuery.php
  - resources/views/public/today/index.blade.php
  - lang/en/today.php
  - lang/pl/today.php
  - tests/Feature/LiveOps/PublicWorldStatusQueryTest.php
  - scripts/acceptance/coverage/surfaces/public-today.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/homepage-template-selector.json
  - scripts/acceptance/seed-homepage-navigation-seo.php
  - scripts/acceptance/set-liveops-state.php
  - scripts/acceptance/tests/homepage-navigation-seo.spec.mjs
  - config/game-auth.php
  - docs/architecture/MODULE_CATALOG.md
  - docs/agents/tasks/active/OTERYN-20261005-liveops-world-status.md
validation:
  - command: php -l on new PHP implementation/test files
    result: PASS
    evidence: local syntax validation before first branch commit
  - command: independent exact-diff review after remediation
    result: PASS
    evidence: review found acceptance fixture leakage risk and swallowed LiveOps query exceptions; functional head 1c6681360940426e248e596746b4745e556049ce creates/cleans the fixture per test and reports query failures before rendering truthful unavailable state
  - command: superseded Phase 7 failure triage and fixture remediation
    result: PASS
    evidence: run 37362795927 / job 111941882180 exposed one duplicate-assignment fixture failure; functional head cfef690a56982dc8dceddef1416e52b3086cc688 makes the assignment fixture idempotent
  - command: Acceptance E2E and Visual UX on exact head e7c414276714f521f6953edc9cd59138ae592bb2
    result: PASS
    evidence: zero-retry browser acceptance completed successfully and exercised LiveOps ready, stale, unavailable, configured maintenance and recovery states
  - command: CI checkpoint validation on exact head e7c414276714f521f6953edc9cd59138ae592bb2
    result: FAIL
    evidence: classifier reached checkpoint validation and rejected unsupported task-result enum values PASS_AFTER_REMEDIATION and FAIL_REPAIRED; this checkpoint-only repair replaces them with contract-valid PASS semantics while preserving the remediation evidence
blockers: []
next_action: none; PR #1459 merged after all exact-head required checks passed, and this task is archived.
```


## Terminal closeout

- PR #1459 merged from exact head `a41b2210e3c433176f3843be5758d22def63386d` on 2026-10-06.
- Exact-head CI, Phase 7, Acceptance E2E and Visual UX, Portal Acceptance Contract, Agent Governance, CodeQL, Edge Security, DB Outage, Game Auth Ticket Concurrency, Synology staging image build and native protocol checks passed.
- Governing Issue #1458 is closed.
- No production runtime-status activation/configuration is claimed by this archive.


## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR #1459 is terminal and the same-repository implementation branch has no durable post-merge purpose.
source_branch_evidence: PR #1459 merged from exact head a41b2210e3c433176f3843be5758d22def63386d and live branch lookup confirms feat/liveops-world-status-1458 is absent.
```
