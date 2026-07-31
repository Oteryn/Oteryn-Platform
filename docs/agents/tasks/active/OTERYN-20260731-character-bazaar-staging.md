---
task_id: OTERYN-20260731-character-bazaar-staging
policy_version: 2
project_lane: oteryn-platform-bazaar
task_kind: implementation
decomposition_decision: single
classification: staging_enablement
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/adr/0016-character-bazaar-wallet-and-escrow.md
  - docs/contracts/CHARACTER_TRANSFER_CONTRACT.md
  - docs/operations/MARKETPLACE_OPERATIONS.md
search_first:
  - PR #270 and Issue #269
  - PR #274 archive lifecycle
  - current main, open PRs, active tasks and Marketplace path ownership
  - current Synology staging deployment package and exact deployed evidence
optional_reads:
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
---

# OTERYN-20260731-character-bazaar-staging

## Goal

Enable and prove the already-delivered Character Bazaar on the existing Synology staging architecture without reimplementing product behavior, weakening least privilege, changing Canary source, introducing payments or making any production claim.

## Preflight classification

```yaml
classification: staging_enablement
rationale:
  - PR #270 already delivered the complete Character Bazaar backend, Wallet ledger, reachable EN/PL public/account/admin UI, recovery state machine and exact-head isolated validation.
  - PR #274 archived that implementation after exact-head validation.
  - the latest durable staging deployment predates PR #270.
  - current Synology Compose and deployment automation do not supply the Character Bazaar feature flag, escrow account identifier, dedicated character-transfer connection or scheduler process required by the Marketplace runbook.
```

## Execution shape

```yaml
policy_version: 2
project_lane: oteryn-platform-bazaar
task_kind: implementation
decomposition_decision: single
context_pressure: medium
context_growth: growing
estimate_confidence: high
focused_tests_during_work: true
component_gate_after_milestone: true
heavy_validation_runs_expected: 1
max_full_attempts_per_session: 2
separate_validator_session: preferred
```

## Acceptance criteria

- [ ] Synology staging configuration fails closed unless Character Bazaar enablement, escrow identity and the dedicated transfer credential are complete and valid.
- [ ] The dedicated `canary_character_transfer` principal is provisioned only through the reviewed staging deployment boundary and passes effective-grant verification.
- [ ] A dedicated non-login escrow account is resolved and validated without binding it to a Platform Identity or exposing a usable credential.
- [ ] Platform web and scheduler processes receive the same Marketplace/transfer configuration, while no public service gains a new network bind.
- [ ] `marketplace:reconcile` runs every minute through one persistent scheduler process and retains `withoutOverlapping` behavior.
- [ ] Deployment runs Platform migrations, transfer privilege verification, a bounded reconciliation pass and Marketplace-specific health checks before reporting success.
- [ ] Rollback preserves Marketplace data and does not permit rollback to an image that cannot understand persisted Marketplace states.
- [ ] Existing Character Bazaar focused feature, MariaDB privilege/concurrency and zero-retry EN/PL desktop/tablet/mobile browser evidence remains green on the exact final head.
- [ ] Exact-head repository CI and affected deployment/image gates pass.
- [ ] Any live staging deployment, new staging secret value or staging data mutation remains separately approval-gated and is not represented as complete until directly observed.
- [ ] No production deployment, production secret/data mutation, payment provider, Canary repository write or `PRODUCTION_PROVEN` claim occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/.env.example
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/README.md
  - database/provisioning/canary-character-transfer.sql.template
  - docs/operations/MARKETPLACE_OPERATIONS.md
  - docs/agents/tasks/active/OTERYN-20260731-character-bazaar-staging.md
modules:
  - Marketplace
  - Wallet
  - CanaryIntegration
  - Synology staging deployment
dependencies:
  - PR #270 merge 0f19656e0875d0a10b22002ac0e096deb20e94d8
  - ADR 0016
  - CHARACTER_TRANSFER_CONTRACT.md
  - existing trusted-main Synology deployment workflow
blockers:
  - none for repository implementation and isolated validation
  - live staging execution requires separate approval before adding/changing a secret value or mutating staging data
cross_repository_tasks:
  - blakinio/canary remains read-only; no write is authorized
```

## Security handoff

```yaml
trust_boundary: Platform and its scheduler to Canary through the operation-specific character-transfer principal and dedicated escrow account
identity_authorization_invariant: browser-supplied account or player identifiers never establish seller, bidder, winner or administrator authority
canary_compatibility: no Canary schema or source change; current accounts, players and cluster_sessions contract remains pinned
rollback_required: true
secret_or_production_configuration: one staging-only transfer credential is required but no value may be committed, logged or changed without separate approval; production is excluded
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-31T08:11:00Z
head: b3706e53dce0e3222cdfabbb6d3a26abae03dcbb
branch: feat/OTERYN-20260731-character-bazaar-staging
pr: none
status: implementing
context_routes:
  - agent-governance
  - architecture
  - accounts-characters
  - canary-integration
  - database
  - web-cms
  - admin-rbac
  - testing
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/.env.example
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/README.md
  - database/provisioning/canary-character-transfer.sql.template
  - docs/operations/MARKETPLACE_OPERATIONS.md
  - docs/agents/tasks/active/OTERYN-20260731-character-bazaar-staging.md
proven:
  - PR #270 merged the complete Character Bazaar implementation and exact-head isolated acceptance as 0f19656e0875d0a10b22002ac0e096deb20e94d8.
  - PR #274 archived the completed implementation and removed its temporary acceptance dispatcher.
  - current main retains Marketplace routes, UI, migrations, Wallet ledger, transfer adapter, EN/PL strings, recovery runbook and focused/browser tests.
  - current production-default Marketplace configuration is fail-closed, and routes are absent when MARKETPLACE_ENABLED is false.
  - the latest durable Synology staging deployment identity 583cae5f430998b2bbdf5e60b59d93f09ec6f4c8 predates PR #270.
  - current deploy/synology/compose.yml lacks Marketplace enablement, escrow, transfer-connection and scheduler configuration.
  - current deploy.sh provisions and verifies read-only, provisioning and character-create principals but not the character-transfer principal.
  - open PR #335 touches deploy/synology/compose.yml for boot recovery; this task avoids owning that path by using a separate Compose overlay.
derived:
  - the smallest real gap is staging enablement rather than product reimplementation, generic hardening or production activation.
  - one bounded branch and PR can add the missing staging package without overlapping PR #335's compose.yml ownership.
unknown:
  - whether the current staging Environment already contains an unused character-transfer credential.
  - whether a dedicated unbound escrow account already exists in the current staging Canary database.
conflicts: []
first_failure:
  marker: synology-marketplace-prerequisites-absent
  evidence: current compose.yml, deploy.sh and deploy workflow omit all Character Bazaar transfer and scheduler prerequisites required by MARKETPLACE_OPERATIONS.md
rejected_hypotheses:
  - Character Bazaar requires reimplementation: current main code and PR #270 evidence prove the product slice already exists.
  - isolated CI proves staging activation: durable project state explicitly says later Marketplace evidence was not deployed by the last verified staging refresh.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260731-character-bazaar-staging.md
validation:
  - command: live GitHub preflight over main, PRs, tasks, code, migrations, routes, UI, tests, deployment package and durable evidence
    result: PASS
    evidence: main b3706e53dce0e3222cdfabbb6d3a26abae03dcbb; PRs #270/#274/#335/#363; project/task/deployment sources
  - command: repository and component tests
    result: NOT_RUN
    evidence: implementation has not yet been committed
blockers:
  - none for repository implementation
next_action: Implement the fail-closed Synology Marketplace Compose overlay, deployment provisioning/verification and scheduler health boundary without modifying compose.yml.
```

## Notes

This task must stop at a human decision before changing any staging secret value or mutating staging data. Repository and isolated staging-like evidence cannot establish live staging or production correctness.
