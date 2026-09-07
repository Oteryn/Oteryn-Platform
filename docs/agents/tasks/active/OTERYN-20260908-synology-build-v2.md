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
updated_at: 2026-09-07T22:44:53Z
head: c15493a1c4c38762b486893a56b40ee6004f0381
branch: ci/synology-build-v2-1328
pr: none
status: implementing
context_routes: []
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/reuse-release.Dockerfile
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
proven:
  - protected main is c15493a1c4c38762b486893a56b40ee6004f0381 and requires platform-gate
  - Issue 1328 is open and directly governs component-aware proportional Synology builds
  - current build workflow uses a fixed platform/game-gateway/deploy-runner matrix
  - current Platform Dockerfile uses repository-wide COPY . .
derived:
  - exact release identity can be preserved without full unchanged-component rebuilds by publishing metadata-only current-release derivatives from immutable component-source images
unknown:
  - whether Issue 1313 live staging proof is complete enough for integration
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - continuing isolated legacy build patches is sufficient
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
validation:
  - command: repository and live Issue/branch ownership inspection
    result: PASS
    evidence: no open PR or active packet found for Issue 1328; dedicated branch created from exact main
blockers:
  - Issue 1313 live staging proof must be reconciled before integration
next_action: implement and contract-test component-aware build classification and immutable component reuse on the dedicated branch
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

## Notes

The build replacement deliberately preserves the existing guarded deploy workflow and rollback/recovery boundary rather than redesigning production-sensitive staging state in the same change.
