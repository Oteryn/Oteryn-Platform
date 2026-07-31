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
  - docs/operations/MARKETPLACE_STAGING_ENABLEMENT.md
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

Enable and prove the already-delivered Character Bazaar on Synology staging without reimplementing product behavior, weakening least privilege, changing Canary source, introducing payments or making a production claim.

## Classification

```yaml
classification: staging_enablement
rationale:
  - PR #270 already delivered backend, Wallet ledger, EN/PL public/account/admin UI, transfer saga, recovery and exact-head isolated validation.
  - PR #274 archived that completed implementation.
  - the last durable Synology staging release predates PR #270.
  - the ordinary staging package lacked explicit Marketplace configuration, transfer principal, escrow identity and scheduler.
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

- [x] Normal runtime defaults are fail-closed outside isolated testing and acceptance.
- [x] A separate Compose overlay provides one private scheduler with no published port.
- [x] The exact least-privilege transfer template and effective verifier remain authoritative.
- [x] The staging control writes a prepared disabled secret state atomically before mutation and retains it only on the runner state volume.
- [x] The reviewed non-login escrow identity is created or validated by name and immutable creation marker and must remain unbound.
- [x] Enablement runs migrations, privilege verification, bounded reconciliation, Platform recreation, scheduler start, proxy refresh and route verification.
- [x] Rollback control drains/rejects non-terminal work before disabling routes and invoking image rollback.
- [x] EN/PL product behavior and responsive browser acceptance remain covered by the existing Marketplace suite.
- [ ] Exact-head repository and staging-package CI passes after the authorized control workflow changes.
- [ ] Trusted-main marker-gated staging control run succeeds and uploads sanitized evidence.
- [ ] The task records the exact deployed SHA, workflow/run identity and true `STAGING_PROVEN` status.
- [x] Canary repository, payments, production deployment, production secrets/data and `PRODUCTION_PROVEN` remain excluded.

## Ownership

```yaml
owned_paths:
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/character-bazaar-staging-validation.yml
  - config/marketplace.php
  - database/provisioning/canary-character-transfer.sql.template
  - deploy/synology/.env.example
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/marketplace-staging.sh
  - docs/operations/MARKETPLACE_STAGING_ENABLEMENT.md
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
  - existing trusted-main Synology deploy and rollback scripts
blockers:
  - none after explicit user authorization for staging-only secret and data mutation
cross_repository_tasks:
  - blakinio/canary remains read-only; no write is authorized
```

## Security handoff

```yaml
trust_boundary: Platform web and scheduler to Canary through a dedicated character-transfer principal and non-login escrow account
identity_authorization_invariant: browser-supplied account or player identifiers never establish seller, bidder, winner or administrator authority
canary_compatibility: no Canary schema or source change; accounts, players and cluster_sessions contract remains pinned
rollback_required: true
secret_or_production_configuration: one staging-only random transfer password is retained mode 0600 on the dedicated runner state volume; production is excluded
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-31T09:54:00Z
head: 7831e13e31da00ed562ea54d4cb2bccb075adbbd
branch: feat/OTERYN-20260731-character-bazaar-staging
pr: 368
status: validating
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
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/character-bazaar-staging-validation.yml
  - config/marketplace.php
  - database/provisioning/canary-character-transfer.sql.template
  - deploy/synology/.env.example
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/marketplace-staging.sh
  - docs/operations/MARKETPLACE_STAGING_ENABLEMENT.md
  - docs/agents/tasks/active/OTERYN-20260731-character-bazaar-staging.md
proven:
  - PR #270 delivered the complete Character Bazaar product slice and exact-head isolated evidence.
  - current main retains real routes, migrations, Wallet ledger, transfer adapter, recovery, EN/PL UI and Marketplace browser tests.
  - the last durable staging release predates Character Bazaar.
  - the package now defaults normal runtime to disabled, provides a private scheduler overlay and validates it in a dedicated workflow.
  - the authorized control workflow uses the existing staging Environment and deployment concurrency boundary.
  - the transfer password is generated on the staging runner, persisted before mutation in a mode-0600 allowlisted state file and never uploaded.
  - enablement validates or creates the reviewed escrow sink, applies exact grants, migrates, reconciles, starts the scheduler and verifies the live route.
  - rollback control rejects non-terminal auctions and disables Marketplace before invoking image rollback.
  - the user explicitly authorized staging-only transfer-secret creation and escrow/grant/data mutation on 2026-07-31.
derived:
  - the smallest real gap remains staging enablement, not implementation or production activation.
  - one task, branch and PR remain sufficient; no independent Canary write task exists.
unknown:
  - exact trusted-main merge SHA and control workflow run ID.
  - live staging outcome until the marker-gated control run completes.
conflicts:
  - open PR #335 owns deploy/synology/compose.yml; this task uses compose.marketplace.yml and does not modify that path.
first_failure:
  marker: live-staging-not-yet-executed
  evidence: repository package is implemented, but trusted-main control run has not yet executed
rejected_hypotheses:
  - Character Bazaar requires reimplementation: PR #270 and current code prove the product slice exists.
  - CI proves staging activation: repository and isolated acceptance evidence do not prove a live Synology runtime.
  - a GitHub Environment transfer secret is required: the approved design retains the random staging-only value on the existing protected runner state volume and keeps it out of GitHub logs/artifacts.
changed_paths:
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/character-bazaar-staging-validation.yml
  - config/marketplace.php
  - database/provisioning/canary-character-transfer.sql.template
  - deploy/synology/.env.example
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/marketplace-staging.sh
  - docs/operations/MARKETPLACE_STAGING_ENABLEMENT.md
  - docs/agents/tasks/active/OTERYN-20260731-character-bazaar-staging.md
validation:
  - command: exact-head workflows on 35215a08e054aac46efd1f51b06664debae6316c
    result: PASS
    evidence: CI, Agent Governance, Portal Acceptance, Acceptance E2E, Phase 7, Platform DB Outage, Edge Security, Game Auth Concurrency, Build Synology Images and Character Bazaar Staging Validation all passed
  - command: exact-head workflows after control-workflow authorization changes
    result: NOT_RUN
    evidence: awaiting GitHub Actions for the current PR head
blockers:
  - none
next_action: Observe exact-head PR checks, merge PR #368 with marker [character-bazaar-staging], then inspect the trusted-main staging control run and sanitized evidence.
```

## Notes

The live control workflow is authorized only for Synology staging. It may not mutate production, change production secrets, write to `blakinio/canary`, introduce payments or establish `PRODUCTION_PROVEN`.
