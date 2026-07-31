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
  - docs/architecture/adr/0016-character-bazaar-wallet-and-escrow.md
  - docs/contracts/CHARACTER_TRANSFER_CONTRACT.md
  - docs/operations/MARKETPLACE_OPERATIONS.md
  - docs/operations/MARKETPLACE_STAGING_ENABLEMENT.md
search_first:
  - PR #270 and Issue #269
  - PR #274 archive lifecycle
  - PR #368 staging package
  - control run 30623491990 and artifact 8790910943
optional_reads:
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
---

# OTERYN-20260731-character-bazaar-staging

## Goal

Enable and prove the already-delivered Character Bazaar on Synology staging without reimplementing product behavior, weakening least privilege, changing Canary source, introducing payments or making a production claim.

## Final classification

```yaml
classification: staging_enablement
result: STAGING_PROVEN
production_environment_proven: false
```

PR #270 supplied the complete product implementation. This task closed only the proven staging gap: fail-closed configuration, dedicated transfer and escrow controls, one scheduler, recovery-aware rollback and direct exact-release evidence.

## Acceptance criteria

- [x] Runtime defaults are fail-closed outside isolated PHPUnit and acceptance environments.
- [x] The staging overlay supplies one private scheduler using the exact Platform image and no published port.
- [x] The dedicated character-transfer principal is provisioned through the guarded staging control and exact effective grants are verified.
- [x] The random staging-only transfer credential stays only in the protected runner state directory and never enters Git or evidence artifacts.
- [x] The reviewed non-login escrow account is created or validated by immutable marker and verified unbound from Platform Identity.
- [x] Enablement runs migrations, bounded reconciliation, exact-image recreation, scheduler start, proxy refresh and live Bazaar route verification.
- [x] Rollback rejects non-terminal auctions and disables Marketplace before standard image rollback.
- [x] Existing EN/PL public, account and administrator UI plus desktop/tablet/mobile and accessibility evidence remains green.
- [x] PR #368 exact-head repository, browser, database, deployment and production-like checks passed.
- [x] Trusted-main control run `30623491990` completed successfully on exact source `d23293baed9641a7542f8bc1d33d19c13f8f5b5c`.
- [x] Artifact `8790910943` proves Marketplace enabled, exactly one scheduler, verified transfer privileges and reviewed unbound non-login escrow identity.
- [x] No Canary repository write, payment provider, production mutation or `PRODUCTION_PROVEN` claim occurred.

## Final ownership

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
  - docs/agents/tasks/archive/OTERYN-20260731-character-bazaar-staging.md
modules:
  - Marketplace
  - Wallet
  - CanaryIntegration
  - Synology staging deployment
dependencies:
  - PR #270 merge 0f19656e0875d0a10b22002ac0e096deb20e94d8
  - PR #368 merge 4f1ddf3816da04bd8f2aa18471ff35aebe853356
  - PR #370 merge d23293baed9641a7542f8bc1d33d19c13f8f5b5c
  - existing trusted-main Synology deploy and rollback scripts
blockers:
  - none
cross_repository_tasks:
  - blakinio/canary remained source-read-only; staging database mutation used only the approved operation-specific contract
```

## Security handoff

```yaml
trust_boundary: Platform web and scheduler to Canary through a dedicated character-transfer principal and non-login escrow account
identity_authorization_invariant: browser-supplied account or player identifiers never establish seller, bidder, winner or administrator authority
canary_compatibility: no Canary schema or source change; accounts, players and cluster_sessions contract remains pinned
rollback_required: true
secret_or_production_configuration: one staging-only random transfer password is retained mode 0600 on the dedicated runner state volume; production remains excluded
```

## Final checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-31T11:06:00Z
head: d23293baed9641a7542f8bc1d33d19c13f8f5b5c
branch: docs/OTERYN-20260731-character-bazaar-staging-closeout
pr: 375
status: ready
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
  - docs/agents/tasks/archive/OTERYN-20260731-character-bazaar-staging.md
proven:
  - PR #270 merged the complete Character Bazaar product implementation as 0f19656e0875d0a10b22002ac0e096deb20e94d8.
  - PR #368 exact head 7964e70d804bb17ad0e6d361c95bb6ae775743be passed all ten authoritative repository, browser, database, security, deployment and production-like workflow families.
  - PR #368 merged the fail-closed staging package as 4f1ddf3816da04bd8f2aa18471ff35aebe853356.
  - PR #370 merged the exact-image idempotent staging retrigger as d23293baed9641a7542f8bc1d33d19c13f8f5b5c after its full exact-head matrix passed.
  - Character Bazaar Staging Control run 30623491990 and job 91136549015 completed successfully on oteryn-synology-staging.
  - Exact Platform and Gateway tag sha-d23293baed9641a7542f8bc1d33d19c13f8f5b5c was deployed while preserving the approved immutable Canary image.
  - Standard staging health, bindings, Gateway readiness, MFA boundary and World Registry checks passed before Marketplace enablement.
  - Character-transfer effective grants passed the approved column-level SELECT plus players.account_id UPDATE verifier.
  - Pre-enable reconciliation processed zero auctions with zero recovery-required rows.
  - Exactly one Marketplace scheduler was running and live Bazaar verification passed.
  - Artifact 8790910943, digest sha256:b3fc7a50f3431d9e1c8746d1df1e57ed7d70f8e484e35ceb4cbf12a3943b98fe, records STAGING_PROVEN, marketplace_enabled true, scheduler_running_count 1, verified transfer privileges, reviewed unbound non-login escrow identity and production_environment_proven false.
  - The ephemeral deployment environment was removed and GHCR logout completed after evidence upload.
derived:
  - Character Bazaar is implemented and directly proven enabled on the documented Synology staging boundary.
  - The smallest real gap is closed; no further implementation, hardening or staging-enablement task remains.
unknown:
  - Real production behavior until Issue #91 receives separate explicit authorization and direct production evidence.
conflicts: []
first_failure:
  marker: none
  evidence: final trusted-main staging control completed successfully; earlier connector observability limitations were resolved by temporary read-only inspection closed without merge
rejected_hypotheses:
  - Character Bazaar required reimplementation: PR #270 and current code proved the product slice already existed.
  - Repository CI established staging activation: only live control run 30623491990 and artifact 8790910943 established STAGING_PROVEN.
  - A broad Canary credential or source change was needed: the dedicated column-level principal and existing schema contract were sufficient.
  - Production activation was implied by staging authorization: production remained excluded throughout.
changed_paths:
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/character-bazaar-staging-validation.yml
  - config/marketplace.php
  - database/provisioning/canary-character-transfer.sql.template
  - deploy/synology/.env.example
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/marketplace-staging.sh
  - docs/operations/MARKETPLACE_STAGING_ENABLEMENT.md
  - docs/agents/tasks/archive/OTERYN-20260731-character-bazaar-staging.md
validation:
  - command: PR #368 exact-head workflow matrix on 7964e70d804bb17ad0e6d361c95bb6ae775743be
    result: PASS
    evidence: runs 30622363468, 30622363469, 30622363524, 30622363460, 30622363549, 30622363483, 30622363637, 30622363531, 30622363495 and 30622363465
  - command: PR #370 exact-head workflow matrix on 37b4fc594b86fda7d2e974a99253484a0e49dcbd
    result: PASS
    evidence: all ten triggered workflow families completed successfully before merge
  - command: Character Bazaar Staging Control deploy-enable
    result: PASS
    evidence: run 30623491990, job 91136549015, exact source d23293baed9641a7542f8bc1d33d19c13f8f5b5c
  - command: sanitized live staging evidence inspection
    result: PASS
    evidence: artifact 8790910943, 504 bytes, digest sha256:b3fc7a50f3431d9e1c8746d1df1e57ed7d70f8e484e35ceb4cbf12a3943b98fe
blockers:
  - none
next_action: Use Character Bazaar Staging Control for future staging verify/deploy/rollback operations and keep all production activation or proof isolated to separately authorized Issue #91.
```

## Boundary

`STAGING_PROVEN` applies only to the exact Synology staging control run and documented Character Bazaar boundary. It does not establish production deployment, public production exposure, real-money commerce or `PRODUCTION_PROVEN`.
