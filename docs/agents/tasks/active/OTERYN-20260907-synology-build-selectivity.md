---
task_id: OTERYN-20260907-synology-build-selectivity
governing_issue: 1328
terminal_pr_policy: archive_pending
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - active Synology deployment tasks and open PR ownership
  - current Synology image build and release identity contracts
optional_reads: []
---

# OTERYN-20260907-synology-build-selectivity

## Goal

Make Synology image verification proportional to actual image inputs without weakening protected-main release identity or deployment/recovery safety.

## Phase 1 acceptance

- Platform image no longer copies unrelated repository control/test/docs sources.
- Pull requests build only affected Platform, Gateway and deploy-runner images; deployment-package-only PRs retain package/Compose contract validation without unrelated image builds.
- Build-routing control-plane changes fail closed to all three PR images.
- Ordinary protected-main pushes do not spend a build on the non-published deploy-runner.
- Runner-only protected-main changes are not misrepresented as a new Platform/Gateway runtime release.
- Until Phase 2, any protected-main Platform/Gateway or non-runner Synology deployment-package change continues to build both exact-SHA runtime images and dispatch the guarded staging deploy.
- Full repository exact-head CI and Merge Queue remain mandatory.

## Phase 2 hold

Per-component main reuse is not authorized by Phase 1. It requires explicit persisted component source SHA + immutable digest provenance through deploy, candidate, last-good, rollback and recovery contracts before either Platform or Gateway may be reused across an overall protected-main SHA.

## Ownership

```yaml
owned_paths:
  - .dockerignore
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/docker/platform.Dockerfile
  - scripts/ci/classify_synology_images.py
  - tests/ci/test_synology_image_build_routing.py
modules:
  - Synology image build routing
  - Platform Docker build context
  - CI economy
conflicts: []
```

## Checkpoint

```yaml
checkpoint_version: 1
base: c15493a1c4c38762b486893a56b40ee6004f0381
branch: ci/1328-synology-build-selectivity-phase1
status: implementing
proven:
  - The fixed Synology image matrix currently starts Platform, Gateway and deploy-runner builds for deployment-script-only PRs.
  - Platform Dockerfile currently uses repository-wide COPY . . and there is no root .dockerignore, so unrelated repository paths contaminate Platform image cache/content identity.
  - Current protected-main deploy contract requires both Platform and Gateway OCI revisions to equal one release SHA; Phase 1 therefore cannot safely reuse either runtime image on main.
  - Ordinary main pushes do not publish deploy-runner, but the existing matrix still builds it.
derived:
  - PR image selection can be made component-aware immediately once Docker inputs are truthful.
  - Main Platform/Gateway selectivity requires Phase 2 component provenance rather than a simple conditional skip.
unknown:
  - Post-merge live runner-time reduction until representative PR/main evidence exists.
next_action: Complete Phase 1 exact-head validation and protected integration, then gather routing evidence before Phase 2.
```
