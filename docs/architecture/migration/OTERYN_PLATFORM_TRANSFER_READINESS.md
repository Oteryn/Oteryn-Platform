# Oteryn Platform Repository Transfer Readiness

Date: 2026-08-18
Transaction candidate: `OTERYN-PLATFORM-TRANSFER-20260818`
Source: `blakinio/Oteryn-Platform`
Target: `Oteryn/Oteryn-Platform`

## Decision

**Verdict: `PREPARED_NOT_READY` for physical transfer.**

The repository-side pre-cutover owner-coordinate hardening is now complete for Platform-owned components. PR #1153 made Platform-owned GHCR, Synology runner, Character Bazaar and staging-preflight coordinates owner-neutral and passed all required and affected exact-head validation before squash merge `6a3b92cae0099b36d4b58048657fbfa8aea7b9bf`.

The transfer remains fail-closed because repository code cannot prove the live GitHub Package objects/permissions/repository links or the behavior of the currently registered repository-level Synology runner after owner transfer. The connected GitHub action surface also still exposes no repository-transfer operation.

## Current source baseline

### FACT / PROVEN

- Source repository: `blakinio/Oteryn-Platform`, repository ID `1305155726`, public and not archived.
- Connected GitHub integration has source admin/write access.
- Default branch: `main`.
- Exact observed post-hardening `main`: `6a3b92cae0099b36d4b58048657fbfa8aea7b9bf`.
- `main` protection remains enabled with required `classify-changes` and `test` contexts.
- Target `Oteryn/Oteryn-Platform` was absent at the last bounded target-collision observation; this must be refreshed immediately before any physical mutation.
- Platform owner-coordinate hardening PR #1153 exact final head `43f7649b32eefc50e8e8bdc669d44bf4e5de7338` passed all required and affected workflow lanes and squash-merged as `6a3b92cae0099b36d4b58048657fbfa8aea7b9bf`.
- The implementation source branch was deleted after merge.
- No physical repository transfer, package publication/relinking/deletion, runner re-registration, secret/environment mutation, staging/production operation or Game/server repository access occurred in the hardening task.

## Repository-side hardening result

### GHCR namespace — repository gate complete

Platform-owned package coordinates no longer embed the personal owner in transfer-sensitive runtime paths. The shared helper:

```text
deploy/synology/scripts/repository-ghcr-image.sh
```

resolves a validated lowercase owner from the current repository context and constructs:

```text
ghcr.io/<current-owner>/<package>
```

This is consumed by:

- Synology Platform/Gateway/deploy-runner image build validation;
- Synology Platform/Gateway deployment digest resolution;
- Character Bazaar Platform/Gateway image resolution;
- Synology production-target immutable-image preflight.

The current source owner therefore resolves to `ghcr.io/blakinio/...`; after a successful repository owner transfer the same code is designed to resolve `ghcr.io/oteryn/...`.

**Repository classification:** `PASS_PRE_CUTOVER_HARDENING`.

**Remaining live classification:** `MUST_VERIFY_AT_CUTOVER` because GitHub Package object ownership, granular permissions and repository linkage are not observable through the currently available integration.

### Synology repository-level runner — repository gate complete

The runner contract no longer hard-codes:

```text
https://github.com/blakinio/Oteryn-Platform
```

or a personal-owner deploy-runner image default.

- first registration requires an explicit exact `RUNNER_URL`;
- `RUNNER_IMAGE` is explicit and owner-scoped through environment configuration;
- an already registered persistent runner can restart from its `.runner` state without a source-coordinate fallback.

**Repository classification:** `PASS_PRE_CUTOVER_HARDENING`.

**Remaining live classification:** `MUST_VERIFY_AT_CUTOVER` because the existing repository-level runner's attachment/online behavior after GitHub owner transfer remains unobserved. If the transferred repository no longer sees that runner, obtain a new one-time registration token and re-register exactly that runner without exposing the token in Git or logs.

### Production-target preflight — repository gate complete

`deploy/synology/scripts/production-target-preflight.sh` now resolves the expected Platform/Gateway package repositories from the current repository owner and still requires immutable digest references plus matching OCI revision metadata.

The final PR head passed `Synology Production Target Preflight` run `32137944818`. No live/manual preflight was executed by this migration task.

### Dependent package paths — repository gate complete

Character Bazaar resolves Platform/Gateway images through the same owner-neutral package contract while preserving its exact pinned Canary dependency as external provenance.

That external dependency coordinate was deliberately not rewritten merely because Platform ownership is changing.

## Live-side-effect guard

Self-review found that repository-only hardening could otherwise have triggered package publication merely from merging workflow/helper changes to `main`.

The merged change preserves broad PR validation while narrowing automatic main-push side effects:

- deploy-runner package publication requires explicit `workflow_dispatch`;
- repository-only Synology workflow/helper/runner hardening does not match automatic package-publication push paths;
- Character Bazaar's push control remains additionally guarded by its existing explicit commit marker.

The squash merge message for PR #1153 did not contain the Character Bazaar trigger marker.

## Validation evidence

Exact final head: `43f7649b32eefc50e8e8bdc669d44bf4e5de7338`.

- Agent Governance `32137944890` — PASS.
- CI `32137944800` — PASS.
  - `classify-changes` job `95713558651` — PASS.
  - `runtime-tests` job `95713599239` — PASS.
  - `test` job `95714103528` — PASS.
- Character Bazaar Staging Validation `32137944744` — PASS.
- Synology Rollback Contract `32137944930` — PASS.
- Synology Production Target Preflight `32137944818` — PASS.
- Build Synology Staging Images `32137944703` — PASS.
- Edge Security Emulation `32137944714` — PASS.
- Platform DB Outage Validation `32137944713` — PASS.
- Phase 7 Production-Like Validation `32137944771` — PASS.
- Game Auth Ticket Concurrency `32137944724` — PASS.
- reviews: `0`; inline review threads: `0`; PR comments: `0` at merge gate.

The first implementation head exposed a pre-existing ADR-registry contract mismatch because Platform ADR 0041 already used the canonical cross-repository META successor form. The validator was repaired fail-closed to accept one bounded cross-repository successor only with one exact lowercase 40-hex successor merge; ADR 0041 itself was not changed by that repair.

## Remaining pre-cutover evidence

The following remain material and prevent `READY_TO_EXECUTE` / `CUTOVER_READY`:

1. **Live GHCR Packages:** exact Platform-owned package objects, current account scope, granular permissions, linked repository state and target organization publication/read behavior are `UNKNOWN` to the available connector.
2. **Existing self-hosted runner:** whether the current repository-level `oteryn-staging` runner remains attached and online immediately after owner transfer is `UNKNOWN` until observed.
3. **Target repository state:** collision, organization defaults, rulesets/protection and GitHub App access must be refreshed immediately before and after the physical mutation.
4. **Physical transfer capability:** the connected GitHub action surface currently exposes no repository-transfer mutation.

These are cutover evidence/operation gaps, not reasons to copy the repository into a fresh competing target or weaken CI/provenance.

## Physical cutover sequence when remaining evidence is available

Before mutation:

1. refresh exact source repository ID, `main` SHA, admin access and required checks;
2. refresh target-coordinate collision and organization access;
3. verify no active ownership/PR conflict with the transfer;
4. obtain live package ownership/linkage evidence and establish the exact old/new package retention/publication plan without deleting rollback provenance;
5. capture current repository-level runner identity/label/online state without exposing credentials;
6. prove an authorized GitHub surface can perform the actual repository transfer;
7. persist the canonical transaction as `READY_TO_EXECUTE` only when every other required gate is proven.

Immediately after the single transfer mutation:

1. verify repository ID and exact `main` commit continuity at `Oteryn/Oteryn-Platform`;
2. verify Issues/PRs/releases and GitHub App admin/write access;
3. verify `main` rulesets/protection and required `classify-changes`/`test` gates;
4. verify repository secrets/environments exist without reading secret values;
5. verify ordinary GitHub-hosted Actions;
6. verify or deliberately re-register the repository-level Synology runner before any protected staging job;
7. verify target-owner GHCR publication/read/linkage with immutable digest identities;
8. only under separate staging authority run the smallest required protected staging validation.

## Rollback and provenance discipline

- Do not create a new repository at `blakinio/Oteryn-Platform` while redirect/rollback compatibility is required.
- Do not delete or repurpose old user-scoped package objects until target package behavior is proven and the rollback window is closed.
- Historical ADR/task/PR/Issue links remain truthful provenance and are not globally rewritten.
- If target CI/protection/package/runner verification fails after transfer, stop protected deployments and follow the canonical migration transaction rollback decision instead of weakening checks.
- Repository merge authority is not production or staging mutation authority.

## Migration transaction

```yaml
migration_transaction:
  transaction_id: OTERYN-PLATFORM-TRANSFER-20260818
  mutation: transfer
  state: PREPARED_NOT_READY
  public_status: PREPARED_NOT_READY
  source_coordinate: blakinio/Oteryn-Platform
  target_coordinate: Oteryn/Oteryn-Platform
  source_repository_id: 1305155726
  source_main_sha: 6a3b92cae0099b36d4b58048657fbfa8aea7b9bf
  target_collision: MUST_REFRESH_BEFORE_MUTATION
  source_admin_access: true
  source_branch_protection:
    enabled: true
    required_checks:
      - classify-changes
      - test
  gates:
    destination_identity: PASS_LAST_OBSERVATION_REFRESH_REQUIRED
    target_name_available: PASS_LAST_OBSERVATION_REFRESH_REQUIRED
    source_identity_and_admin: PASS
    repository_history_preservation_model: PASS
    executable_coordinate_inventory: PASS_BOUNDED_PLATFORM_SCOPE
    owner_neutral_repository_hardening: PASS
    owner_neutral_hardening_merge: 6a3b92cae0099b36d4b58048657fbfa8aea7b9bf
    ghcr_repository_code_cutover: PASS
    ghcr_live_package_cutover: UNKNOWN_REQUIRES_LIVE_EVIDENCE
    self_hosted_runner_repository_preparation: PASS
    self_hosted_runner_live_cutover: UNKNOWN_REQUIRES_POST_TRANSFER_EVIDENCE
    branch_protection_post_transfer: MUST_REVALIDATE
    physical_transfer_tool_available: false
  material_unknowns:
    - live GHCR package objects permissions and repository links
    - repository-level Synology runner behavior immediately after owner transfer
    - target organization ruleset/protection result until transferred repository observation
  physical_mutation_performed: false
  next_action: resolve live GHCR package ownership/linkage and runner cutover evidence and obtain an authorized transfer-capable GitHub surface before attempting physical transfer
```

## Physical transfer operation

The current connected GitHub action set has no repository-transfer operation. This report therefore does not simulate transfer by copying files, creating a competing repository, force-pushing history or weakening provenance.
