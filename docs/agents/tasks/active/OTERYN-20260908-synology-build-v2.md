---
task_id: OTERYN-20260908-synology-build-v2
governing_issue: 1328
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - Platform Issue #1328
  - Platform Issue #1313
  - Platform PR #1331
optional_reads: []
---

# OTERYN-20260908-synology-build-v2

## Goal

Governing GitHub Issue: #1328 — replace the fixed Synology staging image build path with component-aware, proportional build behavior while preserving protected-main deployment identity, immutable per-component provenance, rollback/recovery safety and the guarded Synology staging deploy boundary.

## Acceptance criteria

- [ ] Pull requests build only runtime/deploy-runner images whose real inputs changed; control-plane changes fail closed to all relevant image builds.
- [ ] Protected-main changes rebuild only changed runtime components; unchanged components are reused only by proven source SHA + immutable digest.
- [ ] Overall protected-main release identity is persisted separately from Platform/Gateway component source identity.
- [ ] Candidate/current/last-good/rollback/recovery state persists and validates per-component source SHA + immutable digest.
- [ ] Platform, Gateway and deploy-runner Docker contexts are bounded to truthful image inputs.
- [ ] Existing Synology migration/schema/rollback safety remains fail closed.
- [ ] Required exact-head CI including `platform-gate` passes and integration follows protected-main policy.
- [ ] Representative deployment-package-only and one-runtime-component protected-main behavior is proven live on Synology staging.
- [ ] Performance evidence demonstrates proportional image-job reduction.

## Ownership

```yaml
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/platform.Dockerfile.dockerignore
  - deploy/synology/docker/gateway.Dockerfile.dockerignore
  - deploy/synology/runner/Dockerfile.dockerignore
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/scripts/upgrade-legacy-release-state.sh
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
modules:
  - Synology staging image build, component provenance, release state and rollback/recovery validation
dependencies:
  - Platform Issue #1313 completed with successful Synology staging deploy run 34167821638
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T06:25:00Z
head: dc51ad8196946aa7f11de4d0898882f9f45675f0
branch: ci/synology-build-v2-1328
pr: 1331
status: implementing
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/platform.Dockerfile.dockerignore
  - deploy/synology/docker/gateway.Dockerfile.dockerignore
  - deploy/synology/runner/Dockerfile.dockerignore
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/scripts/upgrade-legacy-release-state.sh
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
proven:
  - protected main is 519f610b9e02641283cc4322c80b74a9462edb11 and remains protected with required platform-gate
  - Issue 1313 is closed completed and Deploy Synology Staging run 34167821638 succeeded for runtime ada4d5376817d1c8f439dacba88673b367a69668
  - canonical #1328 work is PR 1331 from ci/synology-build-v2-1328; stale parallel branch was inspected but not merged
  - canonical branch was reconciled with current protected main using ordinary merge commit 83fee24941d583dab16cdd642027b9b82ac4121b without force push
  - classifier now separates truthful Platform, Gateway, deploy-runner, deployment-package and control-plane inputs and never schedules deploy-runner publication on ordinary protected-main pushes
  - component Docker contexts are bounded using Dockerfile-specific ignore files and Platform Dockerfile no longer depends on repository-wide COPY
  - build workflow allocates image jobs only when has_image_builds=true and records newly built runtime provenance from the current-run immutable build digest
  - deploy workflow resolves unchanged Platform/Gateway only from validated persisted current-release state, verifies accepted protected-main lineage and validates OCI revision against each component source SHA
  - legacy migration now upgrades current, last-good and candidate managed release-state files only when legacy immutable images prove the persisted release SHA; partial provenance fails closed
derived:
  - deployment-package-only protected-main releases can reconcile staging with zero runtime image builds after persisted component provenance is validated on the Synology runner
  - a one-component protected-main release can combine the current-run immutable digest for the changed component with the persisted proven digest/source SHA for the unchanged component
unknown:
  - exact-head CI result for the current implementation because deterministic contracts are still being updated to the new state format
  - live post-merge representative deployment-package-only and one-runtime-component evidence
conflicts: []
first_failure:
  marker: stale-contract-fixtures
  evidence: previous exact-head runs failed because rollback/classifier tests still constructed or asserted the pre-component-provenance release-state format; shell syntax itself passed
rejected_hypotheses:
  - mutable source/main tags are acceptable reuse provenance
  - metadata-only derivative images are necessary for unchanged components
  - every deploy/synology/runner change is a deploy-runner image input
  - deployment-package-only protected-main changes need synthetic runtime images
  - deploy-runner-only protected-main changes require runtime deployment
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/platform.Dockerfile.dockerignore
  - deploy/synology/docker/gateway.Dockerfile.dockerignore
  - deploy/synology/runner/Dockerfile.dockerignore
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/scripts/upgrade-legacy-release-state.sh
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
validation:
  - command: previous PR 1331 exact-head workflow set at 771221b28fd37ba5e0d8b95b55b1660b99b84258
    result: FAIL
    evidence: first failures isolated to stale provenance contract fixtures/assertions and stale active-task checkpoint; control-plane image builds succeeded
  - command: Deploy Synology Staging run 34167821638
    result: PASS
    evidence: prerequisite #1313 staging deployment completed successfully for ada4d5376817d1c8f439dacba88673b367a69668
blockers: []
next_action: update deterministic classifier/provenance/rollback/recovery tests for the explicit component state model, then requalify the exact branch head and repair only evidence-backed failures
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task remains active until protected-main live verification completes
source_branch_evidence: PR #1331 and Issue #1328 remain open
```

## Notes

The guarded staging deployment/migration boundary is preserved. Production deployment remains excluded. Pull requests never deploy Synology staging, and ordinary protected-main pushes never publish the privileged deploy-runner.
