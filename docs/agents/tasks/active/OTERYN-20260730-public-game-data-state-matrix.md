---
task_id: OTERYN-20260730-public-game-data-state-matrix
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
search_first:
  - Issue #350, parent #326 and open PRs touching Community Data acceptance or portal evidence ledgers
  - current Community Data workflow, deterministic Canary acceptance schema and public game-data pagination/error handling
optional_reads:
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/contracts/PUBLIC_COMMUNITY_DATA_CONTRACT.md
---

# OTERYN-20260730-public-game-data-state-matrix

## Goal

Deliver Issue #350 as one bounded, real-browser Community Data acceptance slice proving long externally sourced values, a multi-page result set, a genuine non-debug application `500`, deterministic restoration and post-failure recovery on Chromium desktop, tablet and mobile.

## Acceptance criteria

- [x] More than 50 deterministic active characters exercise the real MariaDB-backed highscore paginator and page 2.
- [x] A long character name and comment render without document-level horizontal overflow.
- [x] A genuine application `500` is induced without a mock endpoint and exposes no stack, path, SQL, database or credential material.
- [x] The temporarily unavailable view and every acceptance-only row are restored in guaranteed cleanup paths.
- [x] The highscore route succeeds after restoration.
- [x] Community Data evidence and the delivered-surface state declaration are updated truthfully.
- [x] Exact-runtime-head repository and focused zero-retry checks pass.
- [x] Parent #326 remains open for all other surfaces and state/data/media permutations.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-public-game-data-state-matrix.md
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/community-data-completeness.json
modules:
  - PublicGameData
  - Testing
  - AgentGovernance
dependencies:
  - Issue #350
  - parent Issue #326
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T09:34:00Z
head: 62a1777ab6e276fa2bed310d50dd11b7d98f44dd
branch: test/OTERYN-20260730-public-game-data-state-matrix
pr: 351
status: validating
context_routes:
  - agent-governance
  - testing
  - public-game-data
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-public-game-data-state-matrix.md
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/community-data-completeness.json
proven:
  - Community Data Acceptance run 30530686904 executed the real Laravel HTTP runtime against isolated MariaDB and Redis with retries fixed at zero on Chromium desktop, tablet and mobile.
  - The exact runtime head seeded 76 acceptance-only active characters, proved deterministic page-two pagination and removed those rows in a guaranteed cleanup path.
  - A 208-character externally sourced character name and a 249-character public comment rendered through the real highscore and character-detail paths without document-level horizontal overflow at all three viewports.
  - Temporarily removing the exact highscore Blade source after clearing compiled views produced a genuine HTTP 500 with APP_DEBUG=false and without exception, view-path, SQL, database-name or acceptance-credential disclosure.
  - Restoring the Blade source and clearing compiled views returned the same highscore route to HTTP 200 in the same zero-retry test.
  - The Community Data coverage fragment now records exact Issue #350 state IDs, assertions, projects, evidence marker, retries and nonclaims; the executable spec fails closed when that record drifts.
  - CI, Agent Governance, Portal Acceptance Contract, Acceptance E2E and Visual UX, Community Data Acceptance, Edge Security Emulation, Game Auth Ticket Concurrency, Platform DB Outage Validation and Phase 7 Production-Like Validation all passed at runtime head 62a1777ab6e276fa2bed310d50dd11b7d98f44dd.
  - Parent Issue #326 remains open; this slice does not establish all-screen or production completeness.
derived:
  - A bounded state-evidence extension can remain adjacent to the existing Community Data surface fragment without duplicating named routes or overlapping the viewport/browser dimension work in #347 and PRs #348/#349.
unknown:
  - Remaining state/data/error/media permutations for every other delivered screen under parent Issue #326.
conflicts: []
first_failure:
  marker: none
  evidence: no implementation failure occurred on the final runtime-affecting head
rejected_hypotheses:
  - Add an acceptance-only application route that calls abort(500), because that would expand the runtime route table solely for tests.
  - Treat the existing aggregate Community Data marker as exact proof for long-data, large-result and internal-error recovery states.
  - Add the public game-data stress marker to the deaths surface evidence, because that would falsely attribute an unrelated route-state proof.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-public-game-data-state-matrix.md
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/community-data-completeness.json
validation:
  - command: Community Data Acceptance run 30530686904
    result: PASS
    evidence: exact runtime head 62a1777ab6e276fa2bed310d50dd11b7d98f44dd; real Laravel, MariaDB, Redis, Chromium desktop/tablet/mobile and retries 0.
  - command: CI run 30530686531
    result: PASS
    evidence: exact runtime head 62a1777ab6e276fa2bed310d50dd11b7d98f44dd.
  - command: Portal Acceptance Contract run 30530685877
    result: PASS
    evidence: exact runtime head 62a1777ab6e276fa2bed310d50dd11b7d98f44dd; strict portal coverage closure passed.
  - command: Acceptance E2E and Visual UX run 30530685901
    result: PASS
    evidence: exact runtime head 62a1777ab6e276fa2bed310d50dd11b7d98f44dd.
  - command: Agent Governance run 30530685883
    result: PASS
    evidence: exact runtime head 62a1777ab6e276fa2bed310d50dd11b7d98f44dd.
  - command: Edge Security Emulation run 30530686723
    result: PASS
    evidence: exact runtime head 62a1777ab6e276fa2bed310d50dd11b7d98f44dd.
  - command: Game Auth Ticket Concurrency run 30530686688
    result: PASS
    evidence: exact runtime head 62a1777ab6e276fa2bed310d50dd11b7d98f44dd.
  - command: Platform DB Outage Validation run 30530685838
    result: PASS
    evidence: exact runtime head 62a1777ab6e276fa2bed310d50dd11b7d98f44dd.
  - command: Phase 7 Production-Like Validation run 30530685928
    result: PASS
    evidence: exact runtime head 62a1777ab6e276fa2bed310d50dd11b7d98f44dd; this is staging-like evidence, not production proof.
blockers:
  - none
next_action: Verify all repository-required checks on the final documentation head, mark PR #351 ready and squash-merge it without closing parent Issue #326.
```

## Boundaries

This task changes only isolated acceptance evidence and its machine-readable state record. It adds no runtime route, schema, production credential, Canary mutation, staging action or `PRODUCTION_PROVEN` claim.
