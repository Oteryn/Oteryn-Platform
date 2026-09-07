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

Governing GitHub Issue: #1328 — replace the fixed Synology staging image build path with component-aware, proportional build/reuse behavior while preserving protected-main deployment identity, immutable per-component provenance, rollback/recovery safety and the guarded Synology staging deploy boundary.

## Acceptance criteria

- [ ] Pull requests build only runtime/deploy-runner images whose real inputs changed; control-plane changes fail closed to all relevant image builds.
- [ ] Protected-main changes rebuild only changed runtime components; unchanged components are reused only by proven source SHA + immutable digest.
- [ ] Overall protected-main release identity is persisted separately from Platform/Gateway component source identity.
- [ ] Candidate/current/last-good/rollback/recovery state persists and validates per-component source SHA + immutable digest.
- [ ] Platform image construction no longer uses repository-wide `COPY . .` invalidation.
- [ ] Existing Synology migration/schema/rollback safety remains fail closed.
- [ ] Required exact-head CI including `platform-gate` passes and integration follows protected-main policy.
- [ ] The resulting protected `main` is deployed to Synology staging and live health/deployment evidence is verified.

## Ownership

```yaml
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/reuse-release.Dockerfile
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/production-target-preflight.sh
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
  - Platform Issue #1313 live Synology staging deployment proof before integration
blockers:
  - none for implementation; #1313 must be reconciled before integration
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T23:07:30Z
head: e0d322cf4c6557573d3a5a13dcef6f17c58d7795
branch: ci/synology-build-v2-1328
pr: 1331
status: implementing_phase2_provenance
context_routes: []
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/reuse-release.Dockerfile
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
proven:
  - protected main remains ada4d5376817d1c8f439dacba88673b367a69668 through PR 1329 and is an explicit merge parent of this branch
  - exact PR head e0d322cf4c6557573d3a5a13dcef6f17c58d7795 passed all ten triggered workflows including platform-gate, CodeQL, Phase 7, DB outage, governance and Synology contracts
  - classifier separates Platform, Gateway, deploy-runner, runtime-deployment and control-plane changes
  - runner-only main changes do not trigger runtime release or automatic staging deployment
  - Platform image no longer uses repository-wide COPY and retains exact Wiki runtime provenance files used by WikiExpectedContentValidator
  - Issue 1328 explicitly requires component source SHA + immutable digest to propagate through candidate/current/last-good/rollback/recovery state
  - current draft implementation does not yet satisfy that Phase 2 state requirement because it gives reused images a synthetic overall release OCI revision
derived:
  - correct Phase 2 design must preserve org.opencontainers.image.revision as component source SHA, carry overall RELEASE_SHA separately, and deploy unchanged components by their existing immutable digest rather than creating a metadata-only derivative image
unknown:
  - whether Issue 1313 live staging run 34167821638 completes successfully
conflicts: []
first_failure:
  marker: phase2-contract-gap
  evidence: Issue 1328 requires component provenance through release/rollback state and source-SHA OCI validation; the first v2 draft only stored a custom source label while retaining overall release OCI revision
rejected_hypotheses:
  - metadata-only derivative images are sufficient component reuse for Phase 2
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
  - command: exact-head repository and Synology CI on e0d322cf4c6557573d3a5a13dcef6f17c58d7795
    result: PASS_FOR_PHASE1_ONLY
    evidence: all ten triggered workflows succeeded, but manual review against Issue 1328 found an unimplemented Phase 2 state invariant
blockers:
  - Issue 1313 live staging proof must be reconciled before integration
next_action: replace metadata-only derivative reuse with immutable source-image reuse and propagate Platform/Gateway source SHA + digest through deploy, current/last-good state, rollback, recovery and preflight contracts; then requalify exact head
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

## Notes

The guarded staging deployment/migration boundary is preserved. Phase 2 changes only the provenance model needed to make proportional build reuse truthful and rollback-safe.
