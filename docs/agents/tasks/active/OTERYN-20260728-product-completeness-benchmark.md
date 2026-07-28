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

- [x] Inventory every current public, authenticated, administrator, error, empty, validation, authorization and dependency-failure surface from actual code and acceptance evidence.
- [x] Verify every account/security capability listed in Issue #268 against repository implementation and runtime-oriented evidence.
- [x] Verify every character/public-profile capability listed in Issue #268 against repository implementation and runtime-oriented evidence.
- [x] Verify every commerce/entitlement capability listed in Issue #268, including explicit deferred or missing security-critical lifecycle stages.
- [x] Verify support, moderation, enforcement and notification capabilities.
- [x] Verify public/community data capabilities, including Character Bazaar after PR #270.
- [x] Classify benchmark knowledge/tooling capabilities as required, planned, optional/differentiator or not applicable with rationale.
- [x] Classify delivery status as implemented, partial, missing, untested or not applicable; every implemented or partial claim has repository/runtime evidence.
- [x] Create or link a focused issue for every required partial or missing capability.
- [x] Distinguish contract-tested, repository-proven, staging-like proven, product-complete and production-proven claims.
- [x] Commit no personal/account data or unredacted user-supplied screenshots.
- [ ] Required exact-final-head checks pass before merge.

## Ownership

```yaml
owned_paths:
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260728-product-completeness-benchmark.md
  - scripts/acceptance/coverage/validate-product-completeness.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
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
  - exact-final-head checks must complete successfully
cross_repository_tasks:
  - blakinio/canary and all reference portals remain read-only evidence sources
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-28T13:20:00Z
head: 6f3afaa06ba91590833fc7cd018323ab78c361fc
branch: audit/OTERYN-20260728-product-completeness-benchmark
pr: 275
status: validating
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
  - scripts/acceptance/coverage/validate-product-completeness.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
proven:
  - The current delivered-surface contract classifies twenty-one core and Character Bazaar surface groups with explicit role, state, viewport, browser and evidence dimensions.
  - The benchmark ledger classifies forty-three Issue #268 capabilities as three implemented, eleven partial and twenty-nine missing.
  - Relevance classification contains twenty-three required, thirteen planned and seven optional or differentiator capabilities.
  - Every implemented or partial capability cites an existing Oteryn repository or browser-evidence file and stable marker.
  - Every required partial or missing capability links to a focused backlog issue, with account security in #276, characters in #277, support in #279 and community data in #280.
  - Planned commercial lifecycle is tracked by #278 and server-backed knowledge catalogues by #281.
  - The machine validator fails for omitted required capability IDs, unsupported classifications, missing evidence files or markers and required open gaps without focused issues.
  - Portal Acceptance Contract strict coverage job 90286132396 passed the route ledger and product benchmark validator on head 6f3afaa06ba91590833fc7cd018323ab78c361fc.
derived:
  - Oteryn is complete against its delivered route contract but is not benchmark product-complete while required #276, #277, #279 and #280 gaps remain.
  - Commerce can remain planned for the current non-commercial boundary, but #278 is mandatory before commercial activation.
unknown:
  - Real production behavior until Issue #91 is separately authorized and directly executed against the exact deployed release.
conflicts: []
first_failure:
  marker: completeness-contract-gap
  evidence: a green route contract did not enumerate absent product capabilities, so the audit introduced a separate machine-enforced benchmark ledger rather than weakening the delivered-surface contract
rejected_hypotheses:
  - Treat all competitor features as mandatory; rejected because Issue #268 requires explicit relevance classification and rationale.
  - Treat missing capabilities as neutral exclusions; rejected for required product functions.
  - Reuse reference screenshots as repository evidence; rejected because they may contain personal/account information.
  - Mark Character Bazaar wallet as commerce complete; rejected because it has no customer purchase, provider, entitlement, refund or chargeback lifecycle.
changed_paths:
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260728-product-completeness-benchmark.md
  - scripts/acceptance/coverage/validate-product-completeness.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
validation:
  - command: repository and PR overlap search
    result: PASS
    evidence: no open PR matched Issue #268 or the product-completeness benchmark scope before task creation
  - command: Portal Acceptance Contract strict coverage job 90286132396
    result: PASS
    evidence: exact-head delivered-route and forty-three-capability benchmark ledgers validated successfully; diagnostic artifact 8689473473 was uploaded
  - command: remaining exact-final-head workflows
    result: NOT_RUN
    evidence: the task checkpoint update creates the final candidate head and requires a fresh complete workflow result set
blockers:
  - exact-final-head checks must pass before PR readiness and merge
next_action: Wait for the exact-final-head workflow set, inspect any failure at its first failing step, then mark PR #275 ready and merge only when all required checks pass.
```

## Boundaries

This task is an audit and backlog-closure task. It does not implement missing payment, account-deletion, character-mutation, support, moderation or production-go-live behavior. External portals are read-only references, and current product claims require Oteryn repository/runtime evidence.
