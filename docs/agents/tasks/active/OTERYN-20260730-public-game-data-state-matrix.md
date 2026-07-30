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

- [ ] More than 50 deterministic active characters exercise the real MariaDB-backed highscore paginator and page 2.
- [ ] A long character name and comment render without document-level horizontal overflow.
- [ ] A genuine application `500` is induced without a mock endpoint and exposes no stack, path, SQL, database or credential material.
- [ ] The temporarily unavailable view and every acceptance-only row are restored in guaranteed cleanup paths.
- [ ] The highscore route succeeds after restoration.
- [ ] Community Data evidence and the delivered-surface state declaration are updated truthfully.
- [ ] Exact-final-head repository and focused zero-retry checks pass.
- [ ] Parent #326 remains open for all other surfaces and state/data/media permutations.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-public-game-data-state-matrix.md
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - .github/workflows/community-data-acceptance.yml
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
updated_at: 2026-07-30T09:17:00Z
head: 8e613c00503c0874e69e2085c740f87f4a87e002
branch: test/OTERYN-20260730-public-game-data-state-matrix
pr: none
status: implementing
context_routes:
  - agent-governance
  - testing
  - public-game-data
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-public-game-data-state-matrix.md
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - .github/workflows/community-data-acceptance.yml
proven:
  - Current Community Data browser acceptance already runs the real Laravel HTTP runtime against isolated MariaDB and Redis on Chromium desktop, tablet and mobile with retries fixed at zero.
  - The current public game-data evidence covers normal, empty, localization and dependency 503/restoration paths, but not long values, a result set beyond one page or a genuine application 500/recovery path.
  - Active PRs #348 and #349 own separate viewport/browser evidence-ledger paths and do not own the Community Data test or workflow files claimed here.
derived:
  - Temporarily renaming the exact highscore Blade view inside the isolated runner can produce a genuine framework 500 without adding a test-only production route or mock backend.
unknown:
  - Whether the current production error renderer and responsive shell already satisfy the new leakage and overflow assertions without remediation.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Add an acceptance-only application route that calls abort(500), because that would expand the runtime route table solely for tests.
  - Treat the existing aggregate Community Data marker as exact proof for long-data, large-result and internal-error recovery states.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-public-game-data-state-matrix.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation not yet committed
blockers:
  - none
next_action: Extend the existing Community Data browser spec with deterministic stress fixtures, safe 500 induction, guaranteed cleanup and recovery assertions.
```
