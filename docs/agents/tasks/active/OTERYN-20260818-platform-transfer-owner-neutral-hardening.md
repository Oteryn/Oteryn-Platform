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

Remove Platform-owned personal-owner repository/GHCR assumptions from pre-cutover build, deployment, Synology runner and staging-preflight paths while preserving current behavior before transfer and making the same repository code valid after transfer to `Oteryn/Oteryn-Platform`.

## Acceptance criteria

- [x] Platform-owned GHCR publish/deploy paths derive a lowercase namespace from the current repository owner rather than hard-code `blakinio`.
- [x] Character Bazaar uses the current repository-owner namespace for Platform/Gateway images while preserving the separately pinned Canary dependency.
- [x] Liquid20 uses the current repository-owner namespace for its package while preserving the pinned external Freqtrade source coordinate.
- [x] Synology runner image and repository registration URL are explicit/configurable with no hard-coded source repository fallback; an already registered persistent runner remains restart-safe.
- [x] Production-target preflight validates Platform/Gateway immutable images against the current repository owner.
- [x] Focused tests fail if old Platform-owner coordinates are reintroduced.
- [x] The latent ADR-registry validation gap supports the already-canonical cross-repository META successor only with exact merge provenance.
- [x] PR validation remains broad while a repository-only hardening merge cannot automatically publish the privileged deploy-runner image or bootstrap Liquid20 on Synology.
- [x] Full exact-head diff/self-review found no open material implementation finding before final CI.
- [ ] Exact-head required CI and affected workflow lanes pass and review hygiene remains clean before merge.
- [x] No package, runner, secret, staging/production, repository-transfer or Game/server mutation was performed by this task.

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
updated_at: 2026-08-18T12:39:00Z
head: ba4fb432f562198109d3551a95a91133b3b350e3
branch: ci/platform-transfer-owner-neutral-hardening
pr: 1153
status: ready
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
  - Platform base main at task start was ed3a79a0ffe3b0b72e6faec941bab050f513a6d2 with required classify-changes and test protection
  - no active task or open PR owned the implementation paths before task claim
  - Draft PR 1153 owns branch ci/platform-transfer-owner-neutral-hardening
  - repository-ghcr-image.sh lowercases and validates the current GitHub repository owner before constructing Platform-owned GHCR coordinates
  - Synology build deploy Character Bazaar Liquid20 and production-target preflight consume owner-neutral Platform package coordinates
  - the pinned Canary digest and pinned external blakinio/freqtrade source remain intentionally unchanged provenance/dependency coordinates
  - runner first registration requires explicit RUNNER_URL while persistent registered runners can restart without a source-coordinate fallback
  - first implementation head ba4b2ff23211fb0e59384edc73215cff0c383545 passed Agent Governance Synology Production Target Preflight Character Bazaar Liquid20 Synology rollback and Build Synology Staging Images
  - first-head main CI run 32136239455 failed only in AdrRegistryValidationTest because the validator supported only local supersession targets while ADR 0041 already records a cross-repository META successor
  - ADR 0041 blob 4d9d2b3d33242fdb199d5c402a256f2088229764 is identical on main and the failing head and already records Oteryn/Oteryn ADR 0001 plus exact successor merge a2672baac544ada81c526e92f0517903865a9ad0
  - ADR validator now preserves local-target existence checks and accepts exactly one bounded cross-repository successor only with exactly one lowercase 40-hex successor merge; dedicated positive and negative fixtures were added
  - self-review found that the original main-push triggers could cause package publication and Liquid20 Synology bootstrap merely from merging repository-only hardening
  - build main-push paths are now limited to product/image source inputs; deploy-runner publication requires explicit workflow_dispatch while PR validation still builds it
  - Liquid20 pull requests still validate workflow/helper changes but main-push bootstrap is limited to deploy/liquid20 source changes
  - Character Bazaar main-push control remains additionally guarded by the existing explicit commit marker
  - no package publication runner registration secret environment staging production repository transfer or Game/server operation was performed
derived:
  - the repository change preserves current blakinio package resolution before transfer and will resolve lowercase oteryn package coordinates after owner transfer
  - live package permissions links and existing runner attachment remain separate cutover evidence and are not implied by this repository hardening
unknown:
  - live GHCR package objects permissions and repository links remain unobserved until the physical cutover transaction
  - existing repository-level Synology runner behavior immediately after physical owner transfer remains unknown until observed
conflicts: []
first_failure:
  marker: adr_registry_cross_repository_successor_not_supported
  evidence: CI run 32136239455 runtime-tests job 95708164496 artifact 9324137637 contained one JUnit failure in AdrRegistryValidationTest for ADR 0041; branch and main ADR blobs were identical
rejected_hypotheses:
  - hardening only the primary build and deploy workflows is sufficient
  - preserving a hard-coded old RUNNER_URL fallback is necessary for persistent runner restart
  - pinned Canary or Freqtrade dependency coordinates should be rewritten with the Platform owner
  - changing ADR 0041 back to Accepted or weakening supersession validation is an acceptable CI repair
  - merging repository-only hardening may safely trigger package publication or Synology bootstrap without separate live-operation authority
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
  - command: live-state path ownership and PR preflight
    result: PASS
    evidence: no conflicting owner/PR found before implementation
  - command: first-head affected workflow validation
    result: PASS
    evidence: ba4b2ff23211fb0e59384edc73215cff0c383545 passed Agent Governance Synology Production Target Preflight Character Bazaar Liquid20 Synology rollback and Build Synology Staging Images
  - command: first-head main CI
    result: FAIL
    evidence: run 32136239455 isolated the latent ADR registry cross-repository successor contract gap
  - command: exact full PR diff self-review after ADR and no-live-side-effect repairs
    result: PASS
    evidence: 15 declared paths; no unrelated product runtime data secret deployment or Game/server change; validator output contract restored after self-review caught an accidental formatting drift
  - command: final exact-head required and affected GitHub Actions
    result: NOT_RUN
    evidence: final ready-state checkpoint commit will trigger the exact-head generation
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: repository CI package-coordinate runner and validation contracts only; no user-facing runtime or live environment operation is executed
blockers: []
next_action: Observe one final exact-head required/affected workflow generation, then mark PR 1153 Ready and squash-merge only if all required gates and review hygiene remain clean.
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head_reviewed: ba4fb432f562198109d3551a95a91133b3b350e3
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings:
    - original main-push side-effect risk repaired before readiness
    - accidental ADR validator output formatting drift repaired before readiness
  evidence:
    - PR 1153 changed-file list and per-file patches
    - first-head workflow results and JUnit failure artifact
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: PR 1153 is ready for final exact-head validation and merge gate
source_branch_evidence: pending
```
