# Oteryn Platform Repository Transfer Readiness

Date: 2026-08-18
Transaction: `OTERYN-PLATFORM-TRANSFER-20260818`
Source: `blakinio/Oteryn-Platform`
Intended target: `Oteryn/Oteryn-Platform`
Migration backup: `Oteryn/Oteryn-Platform-Migration-Backup-20260818`

## Decision

**Verdict: `NO_GO` for physical transfer. Canonical transaction state: `PREPARED`.**

The previous target-coordinate collision is now **resolved**. Repository ID `1338405017` survived the owner/admin rename and now exists at `Oteryn/Oteryn-Platform-Migration-Backup-20260818`; the live Oteryn organization repository inventory contains no current repository named `Oteryn-Platform`.

The transfer still cannot become `READY_TO_EXECUTE` because the connected GitHub tool surface exposes no repository-transfer mutation, live granular-permission GHCR package linkage is not observable through that surface, existing repository-level Synology runner behavior after transfer remains unproven, and rollback feasibility must be proven against the exact live cutover state.

No repository transfer, package mutation, runner mutation, staging operation or production operation is performed by this report.

## Current live repository identities

### Source — PROVEN

- repository: `blakinio/Oteryn-Platform`;
- repository ID: `1305155726`;
- visibility: public;
- archived: false;
- default branch: `main`;
- current observed `main`: `ac39722ab348c71748e915395787195d2ea20ebb`;
- connected GitHub integration: admin/write access proven;
- `main` protection: enabled;
- required checks: `classify-changes`, `test`.

Comparison from the last pre-rename preflight base `77fa480a3f4e847dac98f76e05b6acd27cca4a57` to current `main` proves only these migration documentation/governance paths changed:

- `docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md`;
- `docs/agents/tasks/archive/OTERYN-20260818-repository-migration-live-reconciliation.md`;
- `docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md`.

Therefore repository-side runtime/package/runner hardening evidence is not invalidated by code drift after the last preflight. The exact cutover evidence lease is still not current because live package/runner/rollback gates remain unresolved.

### Intended target coordinate — PROVEN FREE IN CURRENT ORG INVENTORY

The intended canonical target is:

`Oteryn/Oteryn-Platform`

The current live Oteryn organization repository inventory contains:

- `Oteryn/Oteryn`;
- `Oteryn/Oteryn-Game`;
- `Oteryn/Oteryn-Atlas`;
- `Oteryn/Oteryn-Platform-Migration-Backup-20260818`.

No current organization repository is named `Oteryn-Platform`. The old URL may redirect to the renamed backup; that redirect is not treated as target occupancy.

### Migration backup — PROVEN

- repository: `Oteryn/Oteryn-Platform-Migration-Backup-20260818`;
- repository ID: `1338405017`;
- visibility: public;
- archived: false;
- default branch: `main`;
- classification: migration bootstrap/backup evidence only, not canonical Platform.

Bootstrap PR #1 is closed without merge. The historical one-off seed workflow remains evidence that mirror clone, `git fsck` and full Git bundle verification passed while branch-ref publication failed with `head_push_rc=1` because workflow-bearing refs could not be updated by the Actions `GITHUB_TOKEN`.

Preserving repository ID `1338405017` under the backup coordinate satisfies the target-collision cleanup without pretending that backup repository is the Platform product.

## Repository-side hardening evidence retained

PR #1153 / merge `6a3b92cae0099b36d4b58048657fbfa8aea7b9bf` established owner-neutral repository code for transfer-sensitive Platform surfaces, including:

- owner-neutral GHCR image coordinate construction;
- owner-neutral repository-level runner registration configuration;
- owner-neutral production-target preflight expectations;
- Character Bazaar Platform/Gateway package resolution;
- automatic live-side-effect guards for repository-only hardening changes.

PR #1156 / merge `88e1661bbd13ddb36b064d411d54702075f64852` then removed Liquid20/Freqtrade operational workflows and Synology control assets from Platform scope. Current `liquid20` search results are historical/governance/test references rather than active Platform runtime/control ownership.

The refreshed machine-readable inventory is:

`docs/architecture/migration/oteryn-platform-transfer-inventory.json`

## Current GitHub transfer semantics

Current official GitHub documentation confirms:

- source administrator access is required;
- transferring to an organization requires permission to create a repository in that organization;
- the target account must not already have a repository with the same name;
- the repository transfer preserves repository Git history and transfers Issues, PRs, releases, projects and settings subject to documented exceptions;
- the REST `Transfer a repository` endpoint supports a GitHub App **user access token** with repository `Administration: write`;
- granular-permission packages such as Container registry packages remain scoped to their account, and a repository link is removed when the linked repository transfers to another owner; associated Actions access may therefore change.

The connected ChatGPT GitHub action surface exposes no `transfer repository` mutation. An installation-scoped Actions `GITHUB_TOKEN` is not treated as a substitute for the required user-authorized transfer surface.

## Current material gates

### 1. Target-coordinate collision — PASS

Repository ID `1338405017` is now at the non-colliding backup coordinate and the Oteryn organization inventory contains no current `Oteryn-Platform` repository.

### 2. Source identity/protection — PASS

Repository ID `1305155726`, `main=ac39722ab348c71748e915395787195d2ea20ebb`, public/unarchived/admin access and required `classify-changes` + `test` protection are verified.

### 3. Repository-side owner-neutral hardening — PASS

The runtime-sensitive post-hardening delta is either the accepted Liquid20 scope removal or documentation/governance only. No new owner-sensitive package/runner code was introduced after hardening.

### 4. Physical repository-transfer operation — FAIL / TOOL BLOCKED

The connected GitHub action surface has no repository owner-transfer mutation. Tool availability is a canonical gate and cannot be bypassed with a mirror force-push or a different repository identity.

### 5. Live GHCR package ownership/linkage — UNKNOWN

Repository code is owner-neutral, but current live GitHub Package objects, granular permissions, repository links and target-organization publication/read behavior are not observable through the available connector.

GitHub documents that for granular-permission packages a repository link is removed when the repository transfers to another owner. Package linkage and Actions access therefore require an explicit post-transfer plan/verification rather than an assumption.

### 6. Existing repository-level self-hosted runner — UNKNOWN

Repository-side runner configuration is owner-neutral, but whether the currently registered repository-level Synology runner remains attached and online after owner transfer is not observable through the available connector. The protected staging runner must be verified or deliberately re-registered after transfer before protected staging use.

### 7. Target organization protection/rulesets — MUST REVALIDATE AFTER TRANSFER

The resulting `main` protection/ruleset state cannot be inferred. The transferred repository must be read back immediately after mutation and required checks must still be enforced.

### 8. Rollback feasibility — NOT PROVEN

A transfer-back rollback is plausible but must not be declared `PROVEN` until the exact pre-cutover owner/name availability, permissions and transfer capability are verified. A GitHub redirect is not rollback.

## Canonical transaction

```yaml
migration_transaction:
  transaction_id: OTERYN-PLATFORM-TRANSFER-20260818
  mutation: transfer
  state: PREPARED
  public_status: NO_GO
  source_coordinate: blakinio/Oteryn-Platform
  target_coordinate: Oteryn/Oteryn-Platform
  source_head: ac39722ab348c71748e915395787195d2ea20ebb
  source_repository_id: 1305155726
  pre_state_snapshot:
    source_repository: blakinio/Oteryn-Platform
    source_repository_id: 1305155726
    source_main: ac39722ab348c71748e915395787195d2ea20ebb
    source_visibility: public
    source_archived: false
    source_main_protected: true
    source_required_checks:
      - classify-changes
      - test
    target_coordinate_currently_present_in_org_inventory: false
    migration_backup_repository: Oteryn/Oteryn-Platform-Migration-Backup-20260818
    migration_backup_repository_id: 1338405017
  expected_post_state:
    repository: Oteryn/Oteryn-Platform
    repository_id: 1305155726
    owner: Oteryn
    main_head: exact_source_head_at_cutover
    history_provenance: preserved
    connector_admin_write_access: PASS_REQUIRED
    branch_protection_and_required_checks: PASS_REQUIRED
    package_and_runner_cutover: PASS_REQUIRED
  authority_verified: true
  target_identity_or_absence_verified: true
  target_governance_verified: NOT_APPLICABLE_FOR_ABSENT_TARGET
  source_state_verified: true
  evidence_lease_current: false
  active_pr_task_impact_verified: true
  coordinate_inventory_complete_for_cutover: true
  executable_callers_resolved: true
  ci_impact_resolved: true
  package_impact_resolved_or_owner_risk_acceptance_proven: false
  provenance_strategy_verified: true
  target_collision: false
  ownership_conflict: false
  material_unknowns:
    - live GHCR package object ownership permissions and repository links
    - repository-level Synology runner behavior immediately after owner transfer
    - target organization branch/ruleset result after owner transfer
  cutover_lock:
    owner: none
    acquired_at: none
    invalidated_by: source-head drift target-name occupancy package evidence runner evidence rollback evidence or transfer-surface change
  replay_guard:
    mutation_fingerprint: transfer_repository:1305155726:Oteryn/Oteryn-Platform
    reissue_forbidden_until_state_proven_not_applied: true
    resume_detection: read repository ID 1305155726 owner and exact target coordinate before any transfer request
  point_of_no_return:
    reached_when: GitHub reports repository ID 1305155726 owned by Oteryn at Oteryn/Oteryn-Platform
    consequences: immediate resulting-state verification is mandatory before any later migration or protected runtime action
  residual_risk_acceptance:
    status: none
    accepted_by: none
    accepted_at: none
    exact_scope: none
    expiry_or_recheck: none
    evidence: none
  rollback:
    feasibility: NOT_PROVEN
    operation: transfer repository ID 1305155726 back to blakinio if GitHub permits and the source coordinate remains recoverable
    trigger: material post-transfer identity history CI package or runner verification failure
    decision_owner: repository owner
    execution_window: must be proven immediately before READY_TO_EXECUTE or owner-only CUTOVER_READY
    verification: exact repository ID owner main head protection packages runners and redirect state
  post_mutation_validation:
    - exact repository ID owner coordinate visibility default branch and main head
    - branches tags Issues PRs releases projects and history provenance
    - GitHub App admin/write access protection rulesets and required checks
    - repository secrets and environments existence without reading secret values
    - GitHub-hosted Actions GHCR publication/read/linkage and self-hosted runner state
```

## Why copy/force-push is not the cutover

A mirror copy into repository ID `1338405017` would create a different canonical repository identity from the accepted transfer of repository ID `1305155726`. It would also leave Issues, PR identity, repository settings, packages, runners and other repository-attached state on a different object unless separately reconstructed.

The Game history migration precedent does not silently change the Platform transaction from `transfer` to `copy`.

## Remaining preflight sequence

1. obtain live GHCR package ownership/linkage evidence or an exact bounded owner risk acceptance allowed by the canonical programme;
2. capture current self-hosted runner identity/label/online state through an authorized administrative surface or establish the exact re-registration plan;
3. prove rollback feasibility for the exact current owner/name state;
4. prove an authorized **GitHub App user access token or owner UI** surface can execute the single repository transfer;
5. refresh source head and target-name absence immediately before mutation;
6. if every non-execution gate is proven and exactly one owner-only transfer remains, public status may become `CUTOVER_READY`; if the current authorized tool can execute the transfer, transition internally to `READY_TO_EXECUTE` and continue;
7. after the one transfer mutation, immediately verify repository ID `1305155726` and all required resulting state before any protected runtime operation.

## Current next action

Resolve live GHCR package linkage plus self-hosted runner cutover evidence and prove rollback feasibility. The target-coordinate rename step is complete and must not be requested again.
