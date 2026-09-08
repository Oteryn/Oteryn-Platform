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
  - deploy/synology/tests/test_fresh_baseline_contract.py
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
updated_at: 2026-09-08T06:36:00Z
head: fb9e7ea5c945a56c9e4487a2904a36de74b6057b
branch: ci/synology-build-v2-1328
pr: 1331
status: validating
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
  - deploy/synology/tests/test_fresh_baseline_contract.py
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
  - classifier separates truthful Platform, Gateway, deploy-runner, deployment-package and control-plane inputs and never schedules deploy-runner publication on ordinary protected-main pushes
  - component Docker contexts are bounded using Dockerfile-specific ignore files and Platform Dockerfile no longer depends on repository-wide COPY
  - build workflow allocates image jobs only when has_image_builds=true and records newly built runtime provenance from the current-run immutable build digest
  - deploy workflow resolves unchanged Platform/Gateway only from validated persisted current-release state, verifies accepted protected-main lineage and validates OCI revision against each component source SHA
  - legacy migration upgrades current, last-good and candidate managed release-state files only when legacy immutable images prove the persisted release SHA; partial provenance fails closed
  - exact-head fb9e7ea5c945a56c9e4487a2904a36de74b6057b passed Build Synology Staging Images run 34195160260, Synology Rollback Contract run 34195160199, Synology Production Target Preflight run 34195160284, Agent Governance run 34195160289, CodeQL run 34195160162, Edge Security Emulation run 34195160239 and Character Bazaar Staging Validation run 34195160165
  - Build Synology Staging Images run 34195160260 successfully built and runtime-smoked Platform, Gateway and deploy-runner because the PR changes the control-plane classifier/workflow and therefore correctly exercised the fail-closed all-component path
derived:
  - deployment-package-only protected-main releases can reconcile staging with zero runtime image builds after persisted component provenance is validated on the Synology runner
  - a one-component protected-main release can combine the current-run immutable digest for the changed component with the persisted proven digest/source SHA for the unchanged component
unknown:
  - remaining exact-head broad CI/runtime validation completion for the latest checkpoint commit
  - live post-merge representative deployment-package-only and one-runtime-component evidence
conflicts: []
first_failure:
  marker: none-current
  evidence: stale rollback/recovery/fresh-baseline provenance fixtures were updated; the focused rollback run and Synology build/contract validation are now green at fb9e7ea5c945a56c9e4487a2904a36de74b6057b
rejected_hypotheses:
  - mutable source/main tags are acceptable reuse provenance
  - metadata-only derivative images are necessary for unchanged components
  - every deploy/synology/runner change is a deploy-runner image input
  - deployment-package-only protected-main changes need synthetic runtime images
  - deploy-runner-only protected-main changes require runtime deployment
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/.env.example
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
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
validation:
  - command: Build Synology Staging Images run 34195160260 at exact head fb9e7ea5c945a56c9e4487a2904a36de74b6057b
    result: PASS
    evidence: classifier, deployment package validation, all three fail-closed control-plane image builds and PR runtime smoke checks passed; staging dispatch was correctly skipped for pull_request
  - command: Synology Rollback Contract run 34195160199 at exact head fb9e7ea5c945a56c9e4487a2904a36de74b6057b
    result: PASS
    evidence: explicit component provenance rollback/recovery and fresh-baseline contracts passed
  - command: Synology Production Target Preflight run 34195160284 at exact head fb9e7ea5c945a56c9e4487a2904a36de74b6057b
    result: PASS
    evidence: production-target preflight contract remains green; no production deployment was performed
  - command: Agent Governance run 34195160289 at exact head fb9e7ea5c945a56c9e4487a2904a36de74b6057b
    result: PASS
    evidence: active task/governance contracts passed
  - command: Deploy Synology Staging run 34167821638
    result: PASS
    evidence: prerequisite #1313 staging deployment completed successfully for ada4d5376817d1c8f439dacba88673b367a69668
blockers: []
next_action: finish broad exact-head qualification, repair only evidence-backed failures, then move PR 1331 through required review/checks and Merge Queue before live protected-main selective-build verification
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task remains active until protected-main live verification completes
source_branch_evidence: PR #1331 and Issue #1328 remain open
```

## Notes

The guarded staging deployment/migration boundary is preserved. Production deployment remains excluded. Pull requests never deploy Synology staging, and ordinary protected-main pushes never publish the privileged deploy-runner.
