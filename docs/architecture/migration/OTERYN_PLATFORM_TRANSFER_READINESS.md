# Oteryn Platform Repository Transfer Readiness

Date: 2026-08-18
Transaction candidate: `OTERYN-PLATFORM-TRANSFER-20260818`
Source: `blakinio/Oteryn-Platform`
Target: `Oteryn/Oteryn-Platform`

## Decision

**Verdict: `PREPARED_NOT_READY` for physical transfer.**

The old Wave-1 destination blocker is resolved: the `Oteryn` organization exists and the connected GitHub integration has already proven organization access in the migration programme. The target coordinate `Oteryn/Oteryn-Platform` is not currently present in the organization search.

The transfer is still fail-closed because Platform has executable owner-scoped GHCR and self-hosted-runner coordinates. Those are not historical links and cannot be delegated to ordinary repository redirects.

## Fresh live baseline

### FACT / PROVEN

- Source repository: `blakinio/Oteryn-Platform`, public, not archived.
- Connected GitHub integration permission on source: `admin=true`, `push=true`.
- Default branch: `main`.
- Exact observed `main`: `132cc41d5c722911bdb4f3e30c200c5d8b47f1ec`.
- `main` protection is enabled.
- Required status contexts observed: `classify-changes`, `test`.
- Repository merge policy observed: squash enabled; merge-commit/rebase disabled; auto-merge enabled.
- Organization search did not find `Oteryn/Oteryn-Platform`; no same-name target collision is currently observed.
- Current code contains owner-scoped GHCR references in build/deploy configuration and repository-scoped Synology runner coordinates.
- No physical repository transfer, GHCR mutation, self-hosted runner re-registration, secret/environment mutation, deployment or production operation was performed by this readiness task.

### FACT / official GitHub transfer behavior used by this decision

Current GitHub documentation states that a repository transfer preserves repository contents and Git history and transfers Issues, pull requests, releases, wiki, stars/watchers and repository settings such as webhooks/services/secrets/deploy keys. GitHub also states that package behavior is registry-dependent and that granular-permission packages keep their account scope; a repository link can be removed when the repository moves to another account. Organization defaults apply after transfer.

This means repository transfer itself is not proof that `ghcr.io/blakinio/*` remains a valid publish/deploy authority for workflows executing from `Oteryn/Oteryn-Platform`.

## Executable coordinate findings

### P1 — GHCR namespace

`build-synology-staging-images.yml` currently publishes three Platform-owned images to the personal namespace:

```text
ghcr.io/blakinio/oteryn-platform
ghcr.io/blakinio/oteryn-game-gateway
ghcr.io/blakinio/oteryn-deploy-runner
```

The deployment workflow resolves Platform and Gateway release tags and immutable digests from the same `ghcr.io/blakinio/*` namespace. `deploy/synology/.env.example` and runner configuration also use that namespace.

**Classification:** `MUST_CHANGE_BEFORE_TRANSFER` for workflow/package ownership logic, followed by `MUST_VERIFY_AT_CUTOVER` for actual package publication/access.

Required hardening direction:

1. derive the package namespace from the repository owner or an explicit bounded repository variable instead of embedding `blakinio` in publish/deploy logic;
2. preserve exact SHA/digest release identity;
3. after transfer, prove the new organization repository can publish/read the intended organization-scoped packages with its own `GITHUB_TOKEN` permissions;
4. do not delete or repurpose old user-scoped packages until rollback/provenance requirements are closed.

### P1 — repository-level Synology runner

`deploy/synology/runner/compose.yml` pins:

```text
RUNNER_URL=https://github.com/blakinio/Oteryn-Platform
```

`deploy/synology/runner/entrypoint.sh` has the same old coordinate as its fallback registration URL. The runner container persists its registration in the `runner_config` volume.

**Classification:** `MUST_PREPARE_BEFORE_TRANSFER` and `MUST_VERIFY_AT_CUTOVER`.

Required hardening direction:

1. make the runner URL explicitly configurable in the Compose/environment contract while preserving current behavior before cutover;
2. record the target value `https://github.com/Oteryn/Oteryn-Platform` for the transfer transaction;
3. immediately after transfer verify whether the existing registration is still online and attached to the transferred repository;
4. if it is not, obtain a new one-time registration token and re-register exactly that runner; do not expose the token in Git or logs;
5. only resume staging jobs after the runner identity, repository binding and label `oteryn-staging` are observed at the target coordinate.

### P1 — production-target preflight

`deploy/synology/scripts/production-target-preflight.sh` currently accepts Platform/Gateway immutable image references only when they begin with `ghcr.io/blakinio/...`. The corresponding CI test intentionally asserts that old namespace.

**Classification:** `MUST_CHANGE_BEFORE_TRANSFER` together with GHCR namespace hardening. Otherwise a correctly transferred staging runtime using organization-scoped images would fail its own preflight.

### P2 — documentation/defaults

Operational READMEs, examples and historical evidence contain old URLs and GHCR coordinates.

- executable defaults used for new setup belong to pre-cutover/cutover changes;
- ordinary current-state documentation becomes `MUST_CHANGE_AFTER_TRANSFER` unless required by the executable path;
- archived tasks, old evidence, historical ADRs, past PR/issue links and immutable provenance remain `HISTORICAL_PROVENANCE_DO_NOT_REWRITE`.

## CI and branch protection transaction

Before transfer:

1. freeze a final source `main` SHA;
2. require current `classify-changes` and `test` checks on that exact head;
3. verify no migration-owned PR or branch collision;
4. merge the GHCR/runner owner-neutral hardening before Tier-2 transfer;
5. capture source protection/check names and merge policy as an evidence lease.

Immediately after transfer:

1. verify repository ID continuity and exact `main` commit continuity;
2. verify open PRs/Issues/releases remain attached to the transferred repository;
3. verify `main` protection/rulesets and required `classify-changes`/`test` gates still apply;
4. verify GitHub App/admin write access at `Oteryn/Oteryn-Platform`;
5. verify secrets/environments exist without reading or reproducing secret values;
6. verify Actions can run on GitHub-hosted runners;
7. verify the repository-level Synology runner binding before any protected staging job;
8. publish/resolve new owner-scoped GHCR images and verify immutable digests;
9. run the smallest safe staging validation required by the deployment contract only under separate staging authority.

Branch/ruleset mutation is not performed by this task because the connected GitHub tool surface does not expose that administrative action. Absence of that tool is not represented as successful protection configuration.

## Rollback and redirect discipline

GitHub provides redirects for ordinary transferred repository web/Git coordinates, but redirect availability must not be generalized to Packages, runner registration or executable cross-repository workflow coordinates.

Rollback rules:

- do not create a new repository at `blakinio/Oteryn-Platform` while transfer redirects/rollback compatibility are required;
- preserve old package objects and historical provenance until the target package path is proven and rollback window closes;
- if target CI/protection/runner/package verification fails after transfer, stop deployments and follow the canonical migration transaction rollback decision rather than weakening checks;
- production deployment is not part of repository-transfer rollback authority.

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
  source_main_sha: 132cc41d5c722911bdb4f3e30c200c5d8b47f1ec
  target_collision: false
  source_admin_access: true
  source_branch_protection:
    enabled: true
    required_checks:
      - classify-changes
      - test
  gates:
    destination_identity: PASS
    target_name_available: PASS
    source_identity_and_admin: PASS
    repository_history_preservation_model: PASS
    executable_coordinate_inventory: PASS_BOUNDED_PLATFORM_SCOPE
    ghcr_namespace_cutover: BLOCKED_PRE_HARDENING
    self_hosted_runner_cutover: BLOCKED_PRE_HARDENING_AND_LIVE_REVALIDATION
    branch_protection_post_transfer: MUST_REVALIDATE
    physical_transfer_tool_available: false
  material_unknowns:
    - live GHCR package objects/permissions/repository links
    - repository-level Synology runner behavior immediately after owner transfer
    - target organization ruleset/protection result until the transferred repository is observed
  physical_mutation_performed: false
  next_action: implement and merge owner-neutral GHCR/package and Synology runner coordinate hardening before attempting the transfer
```

## Physical transfer operation

The current connected GitHub action set has no repository-transfer operation. Therefore this task does not simulate transfer by copying files, creating a competing repository, force-pushing history, or weakening provenance.

When all pre-transfer gates are green, the supported physical operation must be performed by an authorized GitHub owner surface that actually exposes repository transfer. After that single mutation, validation must resume from the exact resulting repository state before any further migration transaction.
