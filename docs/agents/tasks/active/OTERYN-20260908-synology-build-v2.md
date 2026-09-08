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
  - Platform PR #1335
  - Platform PR #1336
optional_reads: []
---

# OTERYN-20260908-synology-build-v2

## Goal

Governing GitHub Issue: #1328 — make Synology staging image builds component-aware and proportional while preserving protected-main identity, immutable per-component provenance, rollback/recovery safety and guarded automatic staging deployment.

## Acceptance criteria

- [ ] Pull requests build only images whose real build inputs changed; control-plane changes fail closed.
- [ ] Protected-main changes rebuild only changed runtime components and reuse unchanged components only by proven source SHA + immutable digest.
- [ ] Overall release identity remains separate from Platform/Gateway component identity.
- [ ] Candidate/current/last-good/rollback/recovery state validates explicit per-component provenance.
- [ ] Platform, Gateway and deploy-runner Docker contexts are bounded to truthful inputs.
- [ ] Deployment-package inputs are separated from runtime-image inputs and docs-only changes do not start the Synology image/deploy workflow.
- [ ] Existing migration/schema/rollback safety remains fail closed.
- [ ] Exact-head CI including `platform-gate` and Merge Queue passes.
- [ ] Deployment-package-only and one-runtime-component protected-main behavior is proven live on Synology staging.
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
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_gateway_build_inputs.py
  - tests/ci/test_synology_deployment_package_routing.py
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
modules:
  - Synology staging image selection, deployment package, component provenance, release state and rollback/recovery validation
dependencies:
  - Platform Issue #1313 completed; Deploy Synology Staging run 34167821638 succeeded
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T07:40:00Z
head: 28a1f33c6bb1ce6ab753de7d7f4246be3b11b4c3
branch: ci/1328-synology-truthful-deploy-package
pr: 1336
status: validating
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_deployment_package_routing.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
proven:
  - PR 1331 integrated through Merge Queue as protected main 48565e8c3a31c1d0349a89e7c7b38118fd4cebba with component-aware build/provenance architecture
  - protected-main run 34195930772 exposed a provenance fan-in local-initialization defect after successful Platform/Gateway publication
  - PR 1335 repaired the fan-in and bounded Gateway inputs to go.mod plus cmd/** plus internal/**; its exact-head and Merge Queue qualification passed
  - PR 1335 integrated through Merge Queue as protected main cbe2d4ee618019f1884c3f076133469386198f95; main remains protected with required platform-gate
  - protected-main Build Synology Staging Images run 34197410049 succeeded at cbe2d4ee618019f1884c3f076133469386198f95
  - automatic Deploy Synology Staging run 34197488519 succeeded for the same protected-main release
  - Gateway BuildKit context was reduced from 136.13 kB before truthful bounding to 12.53 kB on PR 1335 qualification
  - remaining broad deploy/synology/** trigger/classifier fallback incorrectly treated Synology documentation as deployment package/runtime reconciliation input
  - PR 1336 replaces that fallback with explicit runtime, operator-only and validation-only input sets and removes the broad deploy/synology/** workflow trigger
  - docs under deploy/synology, including README.md and PUBLIC_ENDPOINTS.md, are excluded from the PR 1336 classifier and workflow trigger model
  - PR 1336 control-plane qualification at e40489363a9b868f8a8d7936027f8248337a871f successfully built and smoke-checked Platform, Gateway and deploy-runner; PR deploy was skipped as required
  - the only Synology validation failure at e40489363a9b868f8a8d7936027f8248337a871f was a stale test that explicitly required the removed deploy/synology/** trigger
  - tests/ci/test_synology_auto_staging_deploy.py now requires representative truthful runtime paths and rejects the broad glob and Synology documentation paths
derived:
  - a docs-only change under deploy/synology can no longer enter the Synology image/deploy workflow once PR 1336 is integrated because neither the top-level paths nor classifier include those documentation paths
  - runtime deployment-package-only protected-main changes remain release-relevant while allocating zero image jobs, enabling exact immutable Platform/Gateway reuse through persisted current-release provenance
  - operator-only and validation-only Synology changes can still be validated without forcing base staging reconciliation
unknown:
  - exact-head CI result after the stale trigger-test and checkpoint-schema repairs
  - protected-main behavior after PR 1336 integration
  - representative deployment-package-only zero-image-build live proof
  - representative one-runtime-component one-image-build live proof with immutable reuse
conflicts: []
first_failure:
  marker: stale-broad-synology-trigger-contract
  evidence: Build Synology Staging Images run 34199930921 failed only because test_synology_auto_staging_deploy.py asserted deploy/synology/** remained in the push block; classifier and all three PR component builds passed
rejected_hypotheses:
  - docs under deploy/synology are deployment package inputs
  - production-target preflight changes require staging reconciliation
  - marketplace operator-control script changes inherently require base staging reconciliation
  - the new component classifier or PR image builds failed at e40489363a9b868f8a8d7936027f8248337a871f
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_deployment_package_routing.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
validation:
  - command: Build Synology Staging Images run 34197410049 at protected main cbe2d4ee618019f1884c3f076133469386198f95
    result: PASS
    evidence: protected-main image build/fan-in path completed successfully after PR 1335
  - command: Deploy Synology Staging run 34197488519
    result: PASS
    evidence: automatic Synology staging deployment completed successfully for cbe2d4ee618019f1884c3f076133469386198f95
  - command: Build Synology Staging Images run 34199930921 at e40489363a9b868f8a8d7936027f8248337a871f
    result: FAIL
    evidence: classifier and Platform/Gateway/deploy-runner PR builds passed; only stale broad-trigger contract failed before Compose validation
blockers: []
next_action: qualify the repaired PR 1336 exact head, integrate through required platform-gate and Merge Queue, then execute representative deployment-package-only and one-component live proofs
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task remains active until live proportional protected-main behavior is proven and Issue #1328 is terminally completed
source_branch_evidence: PR #1336 continues the same governing Issue #1328 task after merged PRs #1331 and #1335
```

## Notes

Production deployment remains excluded. PRs never deploy Synology staging. Ordinary protected-main pushes never publish the privileged deploy-runner. Component reuse remains immutable and provenance-validated.
