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
  - deploy/synology/docker/gateway.Dockerfile
  - deploy/synology/docker/gateway.Dockerfile.dockerignore
  - deploy/synology/runner/Dockerfile.dockerignore
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/scripts/upgrade-legacy-release-state.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_gateway_build_inputs.py
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
updated_at: 2026-09-08T06:48:00Z
head: 9c626b038edc736ff51f06064fbb961ac2b3c1b1
branch: ci/1328-synology-postmerge-fixes
pr: null
status: implementing
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/docker/gateway.Dockerfile
  - deploy/synology/docker/gateway.Dockerfile.dockerignore
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_gateway_build_inputs.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
proven:
  - PR 1331 integrated through Merge Queue as protected main 48565e8c3a31c1d0349a89e7c7b38118fd4cebba; main remains protected with required platform-gate
  - PR 1331 exact-head and Merge Queue qualification passed before integration
  - protected-main Build Synology Staging Images run 34195930772 built and published Platform and Gateway successfully and uploaded exact immutable provenance artifacts
  - protected-main Build Synology Staging Images run 34195930772 failed only in the deploy fan-in before workflow dispatch, so no new Synology staging deployment was claimed
  - first failure is reproducible from job 101963679870: read_component declared name and file in one local statement under set -u, causing name to be expanded before initialization
  - the merged Synology Gateway Dockerfile still copied the entire services/game-gateway tree, causing README.md and the unrelated service Dockerfile to participate in Synology Gateway image invalidation
  - follow-up branch splits the fan-in locals and narrows Gateway Dockerfile, Docker context, classifier and workflow path triggers to go.mod plus cmd/** plus internal/**
  - regression contract tests/ci/test_synology_gateway_build_inputs.py proves truthful Gateway routing/context and the fan-in local initialization rule
derived:
  - the post-merge fan-in repair is sufficient to let the already proven build provenance artifacts reach the guarded Deploy Synology Staging workflow, subject to fresh exact-head and protected-main verification
unknown:
  - exact-head CI result for this follow-up branch
  - successful protected-main staging dispatch/deployment under the repaired fan-in
  - representative deployment-package-only zero-image-build live proof
  - representative one-runtime-component one-image-build live proof
conflicts: []
first_failure:
  marker: synology-main-provenance-fan-in-local-order
  evidence: Build Synology Staging Images run 34195930772 job 101963679870 failed with `name: unbound variable` in read_component before dispatch
rejected_hypotheses:
  - protected-main image build/publish itself failed
  - immutable provenance artifacts were missing
  - a staging deployment succeeded for 48565e8c3a31c1d0349a89e7c7b38118fd4cebba
  - Gateway README.md is a real Synology Gateway image input
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/docker/gateway.Dockerfile
  - deploy/synology/docker/gateway.Dockerfile.dockerignore
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_gateway_build_inputs.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
validation:
  - command: Build Synology Staging Images run 34195930772 at protected main 48565e8c3a31c1d0349a89e7c7b38118fd4cebba
    result: FAIL
    evidence: classify, deployment-package validation, Platform build/publish and Gateway build/publish all passed; job 101963679870 failed in provenance fan-in before dispatch
  - command: Synology Rollback Contract run 34195930773 at protected main 48565e8c3a31c1d0349a89e7c7b38118fd4cebba
    result: PASS
    evidence: rollback/recovery contracts remained green after the first integration
  - command: Deploy Synology Staging run 34167821638
    result: PASS
    evidence: pre-1328 prerequisite staging deployment remained the latest previously proven healthy staging baseline
blockers: []
next_action: open the post-merge repair PR, qualify exact head including platform-gate, integrate through Merge Queue, then verify the protected-main build fan-in and live Synology staging deployment before moving to representative zero-build and one-component cases
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task remains active until live proportional protected-main behavior is proven and Issue #1328 is terminally completed
source_branch_evidence: follow-up branch ci/1328-synology-postmerge-fixes continues the same governing Issue #1328 task ownership after merged PR #1331
```

## Notes

The guarded staging deployment/migration boundary remains unchanged. Production deployment remains excluded. Pull requests never deploy Synology staging, and ordinary protected-main pushes never publish the privileged deploy-runner.
