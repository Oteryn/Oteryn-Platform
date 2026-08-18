# Oteryn Platform Repository Transfer Readiness

Date: 2026-08-18
Transaction: `OTERYN-PLATFORM-TRANSFER-20260818`
Source: `blakinio/Oteryn-Platform`
Intended target: `Oteryn/Oteryn-Platform`
Observed bootstrap target: `Oteryn/Oteryn-platform`

## Decision

**Verdict: `NO_GO` for physical transfer. Canonical transaction state: `PREPARED`.**

The repository-side owner-neutral hardening remains valid evidence, but the live transfer state changed materially after that work. The intended target coordinate is now occupied by a bootstrap-only repository, the Platform source `main` advanced after the previous transfer-readiness checkpoint, and the connected GitHub action surface still exposes no repository-transfer operation.

This report therefore does not claim `READY_TO_EXECUTE` or `CUTOVER_READY` and does not substitute a history copy/force-push for the accepted repository-transfer transaction.

## Current live repository identities

### Source — PROVEN

- repository: `blakinio/Oteryn-Platform`;
- repository ID: `1305155726`;
- visibility: public;
- archived: false;
- default branch: `main`;
- current observed `main`: `77fa480a3f4e847dac98f76e05b6acd27cca4a57`;
- connected GitHub integration: admin/write access proven.

The previous owner-neutral hardening merge remains:

`6a3b92cae0099b36d4b58048657fbfa8aea7b9bf`

The source has advanced since that merge. The current source head therefore invalidates any evidence lease that assumed `6a3b92...` or the later seed baseline `c567da9...` was still the cutover head.

### Intended target coordinate — CONFLICT

The intended canonical target is:

`Oteryn/Oteryn-Platform`

GitHub currently resolves that same case-insensitive repository coordinate to the existing organization repository displayed as:

`Oteryn/Oteryn-platform`

Observed target facts:

- repository ID: `1338405017`;
- visibility: public;
- archived: false;
- default branch: `main`;
- current observed `main`: `db381488697eee315bdf5840ab0d4f8807f7bfb0`;
- current content classification: migration bootstrap/backup scaffolding only;
- repository-local Platform governance/runtime source is absent because canonical Platform implementation still lives at the source repository.

This is an intended migration bootstrap repository, not evidence that the Platform product itself was migrated. Its existence still blocks a native same-name owner transfer of source repository ID `1305155726` until the target coordinate is deliberately freed.

## Bootstrap history-seed evidence

Target PR #1 is an open Draft bootstrap PR whose own contract says not to merge it into canonical imported history.

The one-off workflow:

`.github/workflows/one-off-platform-history-seed.yml`

ran against source baseline:

`c567da9d9ae444110262774f8febf5a5abab2a90`

Workflow run `32140478830`, job `95721679043`, proved:

- mirror clone: PASS;
- `git fsck --full`: PASS;
- full Git bundle creation and verification: PASS;
- backup artifact upload: PASS;
- backup artifact ID: `9325658054`;
- tag push return code: `0` with no tags to migrate;
- head push return code: `1`.

The head push failed because the Actions `GITHUB_TOKEN` was not permitted to create/update workflow-bearing refs. GitHub rejected the source branches with the workflow-permission error. Therefore the successful workflow conclusion is **backup evidence only**, not proof that Platform refs/history were installed as canonical target refs.

The source has since advanced to `77fa480a3f4e847dac98f76e05b6acd27cca4a57`, so rerunning the stale seed unchanged would correctly fail its exact-source-main guard before any push attempt.

## Repository-side hardening evidence retained

PR #1153 / merge `6a3b92cae0099b36d4b58048657fbfa8aea7b9bf` established owner-neutral repository code for the transfer-sensitive Platform surfaces, including:

- owner-neutral GHCR image coordinate construction;
- owner-neutral repository-level runner registration configuration;
- owner-neutral production-target preflight expectations;
- Character Bazaar Platform/Gateway package resolution;
- automatic live-side-effect guards for repository-only hardening changes.

That repository-side preparation remains useful, but it does **not** prove the live package objects, live runner attachment, target rulesets, or the physical transfer operation.

## Current material gates

### 1. Target-coordinate collision — FAIL

`Oteryn/Oteryn-platform` repository ID `1338405017` occupies the intended target coordinate.

The canonical transfer transaction requires the existing source repository ID `1305155726` to become `Oteryn/Oteryn-Platform`; it must not be replaced by a copied repository with a different identity.

Before the transfer can become executable, the bootstrap-only target must be moved out of the intended coordinate through a separately authorized owner/admin repository rename or another explicitly accepted target-resolution operation. Preserve the migration backup evidence before any destructive cleanup.

### 2. Physical repository-transfer operation — FAIL

The connected GitHub action surface exposes repository reads/writes, refs, PRs, Issues and workflow operations, but no repository owner-transfer mutation.

Tool availability is a canonical gate. Lack of a transfer action cannot be reinterpreted as permission to perform a different operation such as force-pushing a mirror into the bootstrap target.

### 3. Live GHCR package ownership/linkage — UNKNOWN

Repository code is owner-neutral, but current live GitHub Package objects, granular permissions, linked repositories and target-organization publication/read behavior are not observable through the available connector.

This remains a material cutover unknown unless the repository owner supplies an exact bounded risk acceptance permitted by the canonical programme or a live administrative surface proves the required package state.

### 4. Existing repository-level self-hosted runner — UNKNOWN

Repository-side runner configuration is owner-neutral, but whether the currently registered repository-level Synology runner remains attached and online after a GitHub owner transfer is unproven.

The transfer must not be used to trigger protected staging work until the resulting runner state is verified or the runner is deliberately re-registered through an authorized non-secret-bearing process.

### 5. Target organization protection/rulesets — MUST REVALIDATE

The resulting `main` protection/ruleset state cannot be inferred from the bootstrap target or from the source's previous protection state. Required checks and protection must be read back after the actual transfer.

## Canonical transaction

```yaml
migration_transaction:
  transaction_id: OTERYN-PLATFORM-TRANSFER-20260818
  mutation: transfer
  state: PREPARED
  public_status: NO_GO
  source_coordinate: blakinio/Oteryn-Platform
  target_coordinate: Oteryn/Oteryn-Platform
  source_head: 77fa480a3f4e847dac98f76e05b6acd27cca4a57
  source_repository_id: 1305155726
  pre_state_snapshot:
    source_repository: blakinio/Oteryn-Platform
    source_repository_id: 1305155726
    source_main: 77fa480a3f4e847dac98f76e05b6acd27cca4a57
    observed_target_repository: Oteryn/Oteryn-platform
    observed_target_repository_id: 1338405017
    observed_target_main: db381488697eee315bdf5840ab0d4f8807f7bfb0
    target_content_classification: migration_bootstrap_only
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
  target_governance_verified: false
  source_state_verified: true
  evidence_lease_current: false
  active_pr_task_impact_verified: true
  coordinate_inventory_complete_for_cutover: false
  executable_callers_resolved: false
  ci_impact_resolved: false
  package_impact_resolved_or_owner_risk_acceptance_proven: false
  provenance_strategy_verified: true
  target_collision: true
  ownership_conflict: false
  material_unknowns:
    - live GHCR package object ownership permissions and repository links
    - repository-level Synology runner behavior immediately after owner transfer
    - target organization branch/ruleset result after owner transfer
  cutover_lock:
    owner: none
    acquired_at: none
    invalidated_by: target collision source-head drift package evidence runner evidence or repository-state drift
  replay_guard:
    mutation_fingerprint: transfer_repository:1305155726:Oteryn/Oteryn-Platform
    reissue_forbidden_until_state_proven_not_applied: true
    resume_detection: read repository ID 1305155726 owner and intended target coordinate before any transfer request
  point_of_no_return:
    reached_when: GitHub reports repository ID 1305155726 owned by Oteryn at the intended coordinate
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
    operation: transfer repository ID 1305155726 back to blakinio if GitHub permits and the rollback coordinate remains recoverable
    trigger: material post-transfer identity history CI package or runner verification failure
    decision_owner: repository owner
    execution_window: must be proven immediately before READY_TO_EXECUTE
    verification: exact repository ID owner main head protection packages runners and redirect state
  post_mutation_validation:
    - exact repository ID owner coordinate visibility default branch and main head
    - branches tags Issues PRs releases and history provenance
    - GitHub App admin/write access protection rulesets and required checks
    - repository secrets and environments existence without reading secret values
    - GitHub-hosted Actions GHCR publication/read/linkage and self-hosted runner state
```

## Why copy/force-push is not the cutover

A mirror copy into repository ID `1338405017` would create a different canonical repository identity from the accepted transfer of repository ID `1305155726`. It would also leave Issues, PR identity, repository settings, packages, runners and other repository-attached state on a different object unless separately reconstructed.

The successful Game history migration used a history-preserving copy under Game-specific accepted governance. That precedent does not silently change the Platform transaction from `transfer` to `copy`.

Changing the Platform operation class requires a new accepted transaction with its own dependency, provenance, CI/package/runner and rollback proof. This report does not make that architecture change.

## Resulting preflight sequence after the first blocker is resolved

After the bootstrap target no longer occupies the intended coordinate:

1. refresh source repository ID, exact `main`, visibility, archived state, admin access, open PR/task ownership and required checks;
2. prove target coordinate absence and organization ownership/access;
3. refresh executable repository-coordinate callers and CI/release/package inventory against the new source head;
4. resolve live GHCR package/linkage evidence or obtain an exact allowed owner risk acceptance;
5. capture the current self-hosted runner identity/label/online state without reading or exposing registration secrets;
6. prove a currently authorized surface can execute the actual repository transfer;
7. prove rollback feasibility;
8. only then transition the canonical transaction to `READY_TO_EXECUTE` if the current authorized tool can perform the mutation, or to public `CUTOVER_READY` only if exactly one owner-only physical operation remains;
9. perform at most the single transfer and immediately enter post-mutation verification.

## Current next action

Owner/admin must first free the intended `Oteryn/Oteryn-Platform` coordinate by an owner-approved rename of the bootstrap-only `Oteryn/Oteryn-platform` repository while preserving its migration backup evidence. After that action, rerun this preflight from live state; do not reuse cached source/target SHAs as authority.
