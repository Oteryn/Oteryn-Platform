# Oteryn Platform Repository Transfer — Post-Transfer Verification

Date: 2026-08-20
Transaction: `OTERYN-PLATFORM-TRANSFER-20260818`
Original source: `blakinio/Oteryn-Platform`
Canonical target: `Oteryn/Oteryn-Platform`
Migration backup: `Oteryn/Oteryn-Platform-Migration-Backup-20260818`

## Decision

**Provider transfer verdict: `POST_TRANSFER_VERIFIED`. Physical owner transfer is complete.**

Repository ID `1305155726` now resolves to `Oteryn/Oteryn-Platform`. The exact pre-cutover main `42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b` survived the owner transfer, historical PR identity is preserved, the repository remains admin-accessible, and `main` remains protected with required `classify-changes` and `test` contexts.

This report does **not** declare the whole ecosystem migration `MIGRATION_COMPLETE=YES`. Remaining governance/coordinate reconciliation, temporary migration-backup disposition and META closeout are independently gated.

No protected staging or production operation was executed to obtain this evidence.

## Current live repository

- repository: `Oteryn/Oteryn-Platform`
- repository ID: `1305155726`
- owner: `Oteryn`
- visibility: public
- default branch: `main`
- current observed main: `256f27ba97f4b103320c186211583ea7c13dcf33`
- exact transferred main: `42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b`
- connector access: admin/write proven
- main protection: enabled
- required checks: `classify-changes`, `test`
- historical PR continuity: PR #1161 readable at the target coordinate

The current main is allowed to advance after transfer. Transfer continuity is anchored to the exact transferred main, not to a requirement that main remain frozen forever.

## Bounded GHCR cutover evidence

Canonical bounded verification run: `32309057579`.

Each image was built from exact transferred main `42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b`, published under `ghcr.io/oteryn/*` with tag `verify-transfer-42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b`, inspected by immutable digest and pulled back successfully.

| Image | Digest | Result |
| --- | --- | --- |
| `ghcr.io/oteryn/oteryn-game-gateway` | `sha256:323fd66336b3de62f82fda69c4c299c78444dfe93481d26420bbc65b1c9b90f7` | PASS |
| `ghcr.io/oteryn/oteryn-platform` | `sha256:1d1e8f367a2006d117224577cc678a60aee4ed08aae304a49f78b4e7097f07c2` | PASS |
| `ghcr.io/oteryn/oteryn-deploy-runner` | `sha256:1eb10741adf42262834825e8e2c50dec4edf8e0f2791935727e75a798f83b520` | PASS |

The images carry target-repository source metadata and exact transferred-main revision metadata.

## Self-hosted runner cutover evidence

Run `32309057579`, job `96248074731`, executed a no-side-effect repository job on runner `oteryn-synology-staging` and completed successfully.

The runner is intentionally registered with `--no-default-labels` and routed only by the custom `oteryn-staging` label. Generic `self-hosted` exposure is intentionally absent because the host has Docker-socket capability. The verification job performed no checkout, deployment, Compose action, package mutation, protected staging operation or production operation.

A separate hosted registry-readback diagnostic in the same run failed with HTTP `403` because the PR `GITHUB_TOKEN` cannot enumerate repository runners. That diagnostic is not evidence of runner failure: actual GitHub scheduling and execution on the exact repository-scoped runner succeeded and is the stronger operational proof.

## Material gates

### Repository identity and transfer continuity — PASS

Repository ID, owner, coordinate, transferred main continuity, history/PR continuity and admin access are proven.

### Branch protection — PASS

`main` remains protected with required `classify-changes` and `test` contexts.

### Target GHCR publication/readback — PASS

All three Platform-owned image names passed bounded target-namespace publish, digest inspection and pull-back verification.

### Repository self-hosted runner attachment — PASS

The exact custom-label runner accepted and completed a no-side-effect repository job after transfer.

### Package repository-link metadata — UNKNOWN / NON-BLOCKING FOR PROVIDER CUTOVER

The current token cannot read all package-link metadata. Operational target publication/readback is proven. Package-link metadata must remain `UNKNOWN`; it must not be fabricated as PASS.

### GitHub App installation metadata — UNKNOWN / NON-BLOCKING FOR PROVIDER CUTOVER

Connector admin/write access is proven. A separate user-token installation metadata view is not available through the current surface and remains `UNKNOWN`.

### Rollback transfer-back feasibility — RECOVERY CONTINGENCY, NOT EXECUTION GATE

The transfer has already succeeded. Transfer-back capability remains a recovery contingency and must not be retroactively represented as a missing pre-transfer execution gate.

### Provider governance/coordinate reconciliation — PENDING INDEPENDENT WORK

The physical transfer is proven, but current repository governance still contains pre-transfer coordinate material being reconciled by separate gated work. This prevents ecosystem-level `MIGRATION_COMPLETE=YES` until the current authority surface is clean and verified.

### Migration backup terminal disposition — PENDING

Repository `Oteryn/Oteryn-Platform-Migration-Backup-20260818` remains migration evidence/rollback material. Archive/delete disposition is a separate terminal action after recovery/evidence obligations are explicitly resolved.

## Canonical transaction state

```yaml
migration_transaction:
  transaction_id: OTERYN-PLATFORM-TRANSFER-20260818
  mutation: transfer
  state: POST_TRANSFER_VERIFIED
  provider_transfer_complete: true
  ecosystem_migration_complete: false
  source_coordinate: blakinio/Oteryn-Platform
  target_coordinate: Oteryn/Oteryn-Platform
  repository_id: 1305155726
  transferred_main: 42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b
  current_main_observed: 256f27ba97f4b103320c186211583ea7c13dcf33
  identity_continuity: PASS
  history_pr_continuity: PASS
  connector_admin_write_access: PASS
  branch_protection_required_checks: PASS
  bounded_ghcr_publish_readback: PASS
  self_hosted_runner_attachment: PASS
  protected_staging_or_production_operation_performed: false
  material_unknowns:
    - package repository-link metadata unavailable through current token
    - GitHub App user-token installation metadata unavailable through current connector surface
  remaining_ecosystem_gates:
    - provider governance and stale-coordinate reconciliation
    - cross-repository META reconciliation
    - temporary migration-backup terminal disposition
```

## Evidence

- provider PR: #1164
- bounded verification run: `32309057579`
- transferred repository identity: ID `1305155726`, `Oteryn/Oteryn-Platform`
- transferred main: `42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b`
- current protected main observed during closeout: `256f27ba97f4b103320c186211583ea7c13dcf33`

## Next action

Remove the temporary verification workflow, finish exact provider programme/inventory reconciliation, pass final clean-head repository gates, merge provider closeout, then reconcile META only to the level proven. Do not mark the whole ecosystem migration complete until the remaining governance and temporary-backup gates are terminal.
