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
  - PR #368 and trusted-main staging control evidence
  - current main, open PRs, active tasks and Marketplace path ownership
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
  - PR #270 delivered the complete product slice and PR #274 archived it.
  - the last durable Synology staging release predated Character Bazaar.
  - PR #368 delivered the missing fail-closed staging package, guarded transfer, escrow, scheduler, reconciliation and rollback controls.
  - the remaining work is exact live-run observation and durable evidence, not product implementation.
```

## Execution shape

```yaml
policy_version: 2
project_lane: oteryn-platform-bazaar
task_kind: implementation
decomposition_decision: single
context_pressure: medium
context_growth: stable
estimate_confidence: high
focused_tests_during_work: true
component_gate_after_milestone: true
heavy_validation_runs_expected: 1
max_full_attempts_per_session: 2
separate_validator_session: preferred
```

## Acceptance criteria

- [x] Runtime defaults are fail-closed outside isolated testing and acceptance.
- [x] One private scheduler uses the exact Platform image and publishes no port.
- [x] The transfer principal is dedicated and effective grants are verified exactly.
- [x] The staging-only random transfer credential remains mode 0600 on the protected runner state volume and never enters Git or artifacts.
- [x] The reviewed non-login escrow identity is created or validated and must remain unbound.
- [x] Enablement runs migrations, privilege verification, bounded reconciliation, scheduler start, proxy refresh and route verification.
- [x] Rollback rejects non-terminal auctions and disables Marketplace before image rollback.
- [x] PR #368 exact-head repository, Marketplace, browser, deployment and production-like gates passed.
- [x] PR #368 merged to trusted main with the authorized `[character-bazaar-staging]` marker.
- [ ] A completed trusted-main control run is reported with exact run identity and sanitized artifact metadata.
- [ ] The task records a directly observed `STAGING_PROVEN` result or the exact first failure and recovery action.
- [x] Canary repository writes, payments, production mutation and `PRODUCTION_PROVEN` remain excluded.

## Ownership

```yaml
owned_paths:
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/character-bazaar-staging-report.yml
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
  - PR #368 merge 4f1ddf3816da04bd8f2aa18471ff35aebe853356
  - existing trusted-main Synology deploy and rollback scripts
blockers:
  - none
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
updated_at: 2026-07-31T10:18:00Z
head: 3b0b77005020387e3c68b8fd56ae166860137f63
branch: fix/OTERYN-20260731-character-bazaar-evidence-report
pr: 370
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
  - .github/workflows/character-bazaar-staging-report.yml
  - .github/workflows/character-bazaar-staging-validation.yml
  - config/marketplace.php
  - database/provisioning/canary-character-transfer.sql.template
  - deploy/synology/.env.example
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/marketplace-staging.sh
  - docs/operations/MARKETPLACE_STAGING_ENABLEMENT.md
  - docs/agents/tasks/active/OTERYN-20260731-character-bazaar-staging.md
proven:
  - PR #270 delivered the complete Character Bazaar product and exact-head isolated evidence.
  - PR #368 exact head 7964e70d804bb17ad0e6d361c95bb6ae775743be passed CI, Agent Governance, Portal Acceptance, Acceptance E2E and Visual UX, Phase 7, Platform DB Outage, Edge Security, Game Auth Concurrency, Build Synology Images and Character Bazaar Staging Validation.
  - PR #368 merged as trusted-main SHA 4f1ddf3816da04bd8f2aa18471ff35aebe853356 with the authorized staging marker.
  - the guarded control is idempotent, serialized with standard Synology deployments and retains the transfer credential only on the runner state volume.
  - the current connector does not enumerate push-triggered workflow runs through fetch_commit_workflow_runs or commit statuses.
  - PR #370 adds a workflow_run reporter that posts only sanitized conclusion, SHA, run and artifact metadata to PR #368.
  - PR #370 changes a watched fail-closed configuration comment so its marked merge performs one idempotent trusted-main retrigger.
derived:
  - reporting through the PR conversation is the smallest durable way to observe the already-authorized staging control with the available connector.
  - no new product task or Canary write task is required.
unknown:
  - conclusion and run ID of the first PR #368 push-triggered staging control.
  - live staging state until a reporter-backed control run completes.
conflicts:
  - open PR #335 owns deploy/synology/compose.yml; this task continues to avoid that path.
first_failure:
  marker: staging-control-run-not-observable
  evidence: connector fetch_commit_workflow_runs returned no push-triggered run for merge SHA 4f1ddf3816da04bd8f2aa18471ff35aebe853356 and commit status was empty
rejected_hypotheses:
  - absence of connector-visible run proves no staging execution: the connector explicitly filters commit workflow runs to pull-request events.
  - repository CI establishes STAGING_PROVEN: isolated checks cannot prove live Synology state.
  - a second product implementation task is needed: only observability and exact live evidence remain.
changed_paths:
  - .github/workflows/character-bazaar-staging-report.yml
  - config/marketplace.php
  - docs/agents/tasks/active/OTERYN-20260731-character-bazaar-staging.md
validation:
  - command: PR #368 exact-head workflow matrix
    result: PASS
    evidence: runs 30622363468, 30622363469, 30622363524, 30622363460, 30622363549, 30622363483, 30622363637, 30622363531, 30622363495 and 30622363465
  - command: PR #370 exact-head workflow matrix
    result: NOT_RUN
    evidence: awaiting GitHub Actions on current head
blockers:
  - none
next_action: Validate PR #370, merge it with marker [character-bazaar-staging], then inspect the reporter comment, control job and sanitized artifact for exact live staging evidence.
```

## Notes

The live workflow is authorized only for Synology staging. It may not mutate production, change production secrets, write to `blakinio/canary`, introduce payments or establish `PRODUCTION_PROVEN`.
