---
task_id: OTERYN-20260818-platform-transfer-owner-neutral-hardening
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
search_first:
  - ghcr.io/blakinio
  - https://github.com/blakinio/Oteryn-Platform
optional_reads: []
---

# OTERYN-20260818-platform-transfer-owner-neutral-hardening

## Goal

Remove Platform-owned personal-owner repository/GHCR assumptions from the pre-cutover build, deployment, Synology runner and staging-preflight paths while preserving current `blakinio/Oteryn-Platform` behavior before transfer and making the same code valid after transfer to `Oteryn/Oteryn-Platform`.

## Acceptance criteria

- [x] Platform-owned GHCR publish/deploy paths derive a lowercase namespace from the current repository owner rather than hard-code `blakinio`.
- [x] Character Bazaar staging control uses the same current repository-owner namespace for Platform/Gateway images while preserving the separately pinned Canary dependency.
- [x] Liquid20 publication derives its image namespace from the current repository owner while preserving the pinned external Freqtrade source coordinate.
- [x] Synology runner image and repository registration URL are explicit/configurable with no hard-coded source repository fallback; an already registered persistent runner remains restart-safe.
- [x] Production-target preflight validates Platform/Gateway immutable images against the current repository owner without hard-coded `blakinio`.
- [x] Focused CI tests fail if source-owner coordinates are reintroduced into transfer-sensitive Platform paths.
- [x] Latent ADR-registry validation is repaired to support the already-canonical cross-repository META successor with exact merge provenance rather than weakening the superseded-state gate.
- [ ] Exact-head required CI, affected workflow lanes and review hygiene pass before merge.
- [x] No package, runner, secret, staging/production, repository-transfer or Game/server mutation is performed.

## Ownership

```yaml
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/liquid20-synology-control.yml
  - .github/workflows/synology-production-target-preflight.yml
  - deploy/synology/.env.example
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/scripts/repository-ghcr-image.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - tools/validation/adr_registry.py
  - tools/validation/test_adr_registry.py
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/agents/tasks/active/OTERYN-20260818-platform-transfer-owner-neutral-hardening.md
modules:
  - repository-migration
  - ci-release-provenance
  - synology-staging-control
  - architecture-registry-validation
dependencies:
  - OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
  - OTERYN-20260818-platform-transfer-readiness
blockers:
  - none for repository hardening
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-18T12:29:00Z
head: ba4b2ff23211fb0e59384edc73215cff0c383545
branch: ci/platform-transfer-owner-neutral-hardening
pr: 1153
status: implementing
context_routes:
  - agent-governance
  - testing
  - ci-repair
  - architecture
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/liquid20-synology-control.yml
  - .github/workflows/synology-production-target-preflight.yml
  - deploy/synology/.env.example
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/scripts/repository-ghcr-image.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - tools/validation/adr_registry.py
  - tools/validation/test_adr_registry.py
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/agents/tasks/active/OTERYN-20260818-platform-transfer-owner-neutral-hardening.md
proven:
  - current Platform base main is ed3a79a0ffe3b0b72e6faec941bab050f513a6d2 and is protected with required classify-changes and test checks
  - active durable tasks own no path in this hardening set; public-domain repair owns only deploy/synology/scripts/health-check.sh among Synology runtime paths and native-auth production verification owns only its task record
  - no open PR was found matching Synology staging GHCR transfer runner hardening intent before task claim
  - Draft PR 1153 owns branch ci/platform-transfer-owner-neutral-hardening
  - primary Synology build and deploy workflows resolve Platform-owned package coordinates through repository-ghcr-image.sh
  - Character Bazaar resolves Platform and Gateway images through the helper while preserving the exact pinned Canary digest and without inspecting the Game/server repository
  - Liquid20 resolves its package namespace through the helper while preserving the pinned external blakinio/freqtrade source coordinate
  - runner Compose and entrypoint no longer contain a source-repository URL fallback; first registration requires an explicit RUNNER_URL while an existing persistent .runner configuration remains restart-safe
  - production-target preflight resolves expected Platform and Gateway repositories through the same helper and validates immutable digest references against those current-owner coordinates
  - first implementation head ba4b2ff23211fb0e59384edc73215cff0c383545 passed Agent Governance, Synology Production Target Preflight, Character Bazaar validation, Liquid20 validation, Synology rollback validation and Build Synology Staging Images
  - main CI run 32136239455 failed only in AdrRegistryValidationTest because ADR 0041 is already Superseded with a cross-repository META successor while the registry validator accepted only local '- Superseded by:' paths
  - ADR 0041 bytes are identical on main and ba4b2ff23211fb0e59384edc73215cff0c383545 at blob 4d9d2b3d33242fdb199d5c402a256f2088229764; the failing ADR was not changed by this task
  - current ADR 0041 already records exact cross-repository successor Oteryn/Oteryn ADR 0001 and exact successor merge a2672baac544ada81c526e92f0517903865a9ad0
  - validator repair preserves fail-closed local successor existence checks and additionally permits exactly one bounded cross-repository '- Successor:' ADR only when exactly one 40-hex '- Successor merge:' is present
  - no package runner secret staging production repository-transfer or Game/server mutation has occurred
  - container/local clone execution is unavailable because this environment cannot resolve github.com; GitHub Actions provides exact-head executable validation
derived:
  - repository code preserves present blakinio behavior through GITHUB_REPOSITORY_OWNER while automatically resolving the lowercase Oteryn owner after a future transfer
  - the first main-CI failure is a latent base validation-contract gap exposed because this runtime-affecting CI lane ran full PHPUnit whereas recent documentation-only migration closeouts skipped runtime-tests
  - live package permissions repository links and runner attachment remain separate cutover evidence and are not proven by repository hardening
unknown:
  - live GHCR package objects permissions and repository links remain unobserved and are intentionally deferred to cutover verification
  - existing repository-level Synology runner binding behavior after physical transfer remains unknown and must be observed after transfer
conflicts: []
first_failure:
  marker: adr_registry_cross_repository_successor_not_supported
  evidence: CI run 32136239455 runtime-tests job 95708164496 artifact 9324137637 reports only Tests Unit Architecture AdrRegistryValidationTest failing on ADR 0041 Superseded successor metadata; main and branch ADR blobs are identical
rejected_hypotheses:
  - hardening only build-synology-staging-images.yml and deploy-synology-staging.yml is sufficient
  - preserving a hard-coded old RUNNER_URL fallback is necessary for an already registered persistent runner
  - the historical pinned Canary package coordinate or external Freqtrade source coordinate should be rewritten merely because Platform owner changes
  - current owner-neutral implementation changed ADR 0041 or caused the ADR lifecycle state itself
  - weakening ADR 0041 back to Accepted is an acceptable way to obtain green CI
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/liquid20-synology-control.yml
  - .github/workflows/synology-production-target-preflight.yml
  - deploy/synology/.env.example
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/scripts/repository-ghcr-image.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - tools/validation/adr_registry.py
  - tools/validation/test_adr_registry.py
  - docs/agents/tasks/active/OTERYN-20260818-platform-transfer-owner-neutral-hardening.md
validation:
  - command: exact GitHub live-state and path ownership preflight
    result: PASS
    evidence: current main active tasks open PR search and executable owner-coordinate inventory refreshed before implementation
  - command: first-head affected workflow validation on ba4b2ff23211fb0e59384edc73215cff0c383545
    result: PASS
    evidence: Agent Governance Synology Production Target Preflight Character Bazaar Liquid20 Synology rollback and Build Synology Staging Images all completed successfully
  - command: first-head main CI on ba4b2ff23211fb0e59384edc73215cff0c383545
    result: FAIL
    evidence: run 32136239455 runtime-tests job 95708164496; sole JUnit failure is latent ADR registry cross-repository successor support
  - command: focused ADR registry repair fixtures
    result: NOT_RUN
    evidence: repair candidate will run through repository validation after push
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: repository build deployment runner and validation contracts only; this task performs no staging or production operation and does not alter a user-facing application path
blockers: []
next_action: Push the bounded ADR registry validation repair with the owner-neutral implementation, then rerun exact-head required and affected workflow validation without weakening any gate.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: PR 1153 implementation is repairing an evidence-backed exact-head CI failure
source_branch_evidence: pending
```
