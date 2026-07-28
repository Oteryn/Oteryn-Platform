---
task_id: OTERYN-20260728-product-completeness-benchmark
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
search_first:
  - docs/agents/tasks/active/** for product-completeness, portal-audit, account, character, commerce, support, wiki or public-data ownership
  - open pull requests for Issue #268 or overlapping audit/report paths
  - routes, controllers, views, migrations, policies, tests and acceptance manifests for every claimed capability
optional_reads:
  - docs/architecture/ROADMAP.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
---

# OTERYN-20260728-product-completeness-benchmark

## Goal

Audit the actual Oteryn Platform product against Tibia/RubinOT and related OTS knowledge-portal benchmarks without treating the current route or acceptance contract as the definition of completeness. Produce a durable evidence-linked route/state inventory, capability matrix and explicit product-gap backlog.

## Acceptance criteria

- [ ] Inventory every current public, authenticated, administrator, error, empty, validation, authorization and dependency-failure surface from actual code and acceptance evidence.
- [ ] Verify every account/security capability listed in Issue #268 against repository implementation and runtime-oriented evidence.
- [ ] Verify every character/public-profile capability listed in Issue #268 against repository implementation and runtime-oriented evidence.
- [ ] Verify every commerce/entitlement capability listed in Issue #268, including explicit deferred or missing security-critical lifecycle stages.
- [ ] Verify support, moderation, enforcement and notification capabilities.
- [ ] Verify public/community data capabilities, including Character Bazaar after PR #270.
- [ ] Classify benchmark knowledge/tooling capabilities as required, planned, optional/differentiator or not applicable with rationale.
- [ ] Classify delivery status as implemented, partial, missing, untested or not applicable; every implemented claim has repository/runtime evidence.
- [ ] Create or link a focused issue for every required partial or missing capability.
- [ ] Distinguish contract-tested, repository-proven, staging-like proven, product-complete and production-proven claims.
- [ ] Commit no personal/account data or unredacted user-supplied screenshots.
- [ ] Required documentation/governance checks pass on the exact final head before merge.

## Ownership

```yaml
owned_paths:
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260728-product-completeness-benchmark.md
  - scripts/acceptance/coverage/**
modules:
  - Product Audit
  - Identity
  - Accounts
  - Characters
  - Marketplace
  - Public Game Data
  - Wiki
  - Admin
  - Support
  - Commerce
dependencies:
  - Issue #268 benchmark scope
  - current main route/controller/view/test state
  - current acceptance coverage manifest and exact-head evidence
  - external reference portals used only as product-capability evidence
blockers:
  - none
cross_repository_tasks:
  - blakinio/canary and all reference portals remain read-only evidence sources
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-28T12:20:00Z
head: 0a9a00014f55d7b2146d4ab151cd2a1b7c5bcd3d
branch: audit/OTERYN-20260728-product-completeness-benchmark
pr: none
status: investigating
context_routes:
  - agent-governance
  - architecture
  - auth-identity
  - accounts-characters
  - public-game-data
  - canary-integration
  - admin-rbac
  - payments
  - web-cms
  - testing
owned_paths:
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260728-product-completeness-benchmark.md
  - scripts/acceptance/coverage/**
proven:
  - Issue #268 explicitly requires an external benchmark audit and forbids treating the current Oteryn acceptance contract as the sole definition of completeness.
  - PR #270 merged Character Bazaar and its exact-head browser, database, security and portal-contract evidence before this audit began.
  - The current durable project state is stale relative to later portal, wiki and marketplace deliveries and must not be reused as complete evidence without current source verification.
derived:
  - A machine-readable capability ledger should accompany the human report so omissions and unsupported implemented claims can be validated deterministically.
unknown:
  - Complete current route/state inventory and the exact evidence level for each capability.
  - Which external benchmark capabilities are launch-required versus planned or optional for Oteryn.
conflicts: []
first_failure:
  marker: completeness-contract-gap
  evidence: the existing acceptance contract can be green while benchmark-required product capabilities remain absent or undeclared
rejected_hypotheses:
  - Treat all competitor features as mandatory; rejected because Issue #268 requires explicit relevance classification and rationale.
  - Treat missing capabilities as neutral exclusions; rejected for required product functions.
  - Reuse reference screenshots as repository evidence; rejected because they may contain personal/account information.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260728-product-completeness-benchmark.md
validation:
  - command: repository and PR overlap search
    result: PASS
    evidence: no open PR matched Issue #268 or the product-completeness benchmark scope before task creation
blockers:
  - none
next_action: Inventory current routes, delivered-surface manifests and existing audit evidence, then establish the machine-readable benchmark schema before classifying capabilities.
```

## Boundaries

This task is an audit and backlog-closure task. It does not implement missing payment, account-deletion, character-mutation or production-go-live behavior. External portals are read-only references, and any current product claims require Oteryn repository/runtime evidence.
