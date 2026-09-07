---
task_id: OTERYN-20260908-synology-build-v2
governing_issue: 1328
required_reads: []
search_first:
  - Platform Issue #1328
  - Platform Issue #1313
optional_reads: []
---

# OTERYN-20260908-synology-build-v2

## Goal

Governing GitHub Issue: #1328 — replace the fixed Synology staging image build path with component-aware, proportional build/reuse behavior while preserving exact protected-main deployment identity, rollback/recovery safety and the existing guarded Synology staging deploy boundary.

## Acceptance criteria

- [ ] Pull requests build only runtime/deploy-runner images whose real inputs changed; control-plane changes fail closed to all relevant image builds.
- [ ] Protected-main runtime/deployment changes publish exact current-release image identities without fully rebuilding unchanged runtime components.
- [ ] Unchanged component reuse is proven from an immutable source image and carries explicit component source provenance.
- [ ] Platform image construction no longer uses repository-wide `COPY . .` invalidation.
- [ ] Existing Synology deployment, rollback/recovery and exact release identity contracts remain fail closed.
- [ ] Required exact-head CI including `platform-gate` passes and integration follows protected-main policy.
- [ ] The resulting protected `main` is deployed to Synology staging and live health/deployment evidence is verified.

## Ownership

```yaml
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/reuse-release.Dockerfile
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
modules:
  - Synology staging image build and release packaging
dependencies:
  - Platform Issue #1313 live Synology staging deployment proof before integration
blockers:
  - none for implementation; #1313 must be reconciled before integration
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T23:00:35Z
head: f87e7574dfe666db498ac88916416cf9f20ea934
branch: ci/synology-build-v2-1328
pr: 1331
status: validating
context_routes: []
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/reuse-release.Dockerfile
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
proven:
  - protected main advanced to ada4d5376817d1c8f439dacba88673b367a69668 through PR 1329 and is an explicit merge parent of this branch
  - PR 1331 is mergeable against current main with a seven-file functional diff plus this active packet
  - Issue 1328 is open and directly governs component-aware proportional Synology builds
  - classifier separates Platform, Gateway, deploy-runner, runtime-deployment and control-plane changes
  - runner-only main changes do not trigger runtime release or automatic staging deployment
  - Platform image no longer uses repository-wide COPY and retains the exact Wiki runtime provenance files used by WikiExpectedContentValidator
  - immutable reuse keeps org.opencontainers.image.revision bound to the current release SHA while recording io.oteryn.component.source-revision separately
  - Build Synology Staging Images run 34168524305 passed classifier and full Synology deployment-contract validation on f87e7574dfe666db498ac88916416cf9f20ea934
derived:
  - the first v2 main rollout is intentionally fail-closed to full Platform and Gateway builds because the control-plane itself changes, seeding future immutable component-source reuse
unknown:
  - whether Issue 1313 live staging proof is complete enough for integration
  - final image-build and repository-wide exact-head CI conclusions for the latest candidate
conflicts: []
first_failure:
  marker: agent-governance-live-liveness
  evidence: run 34168524357 passed checkpoint, ownership, policy and catalog validators before failing only live-aware Control Room/liveness while this packet still pointed to the pre-PR head; checkpoint refreshed here
rejected_hypotheses:
  - continuing isolated legacy build patches is sufficient
  - repository-wide COPY is necessary for Platform runtime completeness
  - deploy-runner-only main changes require a Platform/Gateway release
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/reuse-release.Dockerfile
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
validation:
  - command: Build Synology Staging Images / Classify Synology image inputs
    result: PASS
    evidence: run 34168524305 on f87e7574dfe666db498ac88916416cf9f20ea934
  - command: Build Synology Staging Images / Validate Synology deployment package
    result: PASS
    evidence: run 34168524305 passed shell syntax, Synology contract tests, Compose manifests and LAN isolation
  - command: Synology Production Target Preflight / static-validation
    result: PASS
    evidence: run 34168524540 on f87e7574dfe666db498ac88916416cf9f20ea934
blockers:
  - Issue 1313 live staging proof must be reconciled before integration
next_action: qualify the refreshed exact PR head, reconcile Issue 1313 live staging evidence, integrate through protected main, then verify the exact-main Synology staging deployment
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

## Notes

The build replacement deliberately preserves the existing guarded deploy workflow and rollback/recovery boundary rather than redesigning production-sensitive staging state in the same change.
