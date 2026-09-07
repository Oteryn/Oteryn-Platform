---
task_id: OTERYN-20260907-auto-synology-staging
governing_issue: 1313
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - active Synology deployment tasks and open PR ownership
  - existing trusted-main Synology dispatch patterns
optional_reads: []
---

# OTERYN-20260907-auto-synology-staging

## Goal

Governing GitHub Issue: #1313 — make each runtime-affecting protected-main change automatically available on Synology staging after exact-SHA images pass their build/contract checks, while retaining the existing guarded manual deploy/rollback workflow.

## Acceptance criteria

- [ ] Runtime-affecting `main` pushes and Synology workflow/package changes build exact-SHA Platform/Gateway images.
- [ ] A successful exact-main image build automatically dispatches the existing `Deploy Synology Staging` workflow and waits for its terminal result.
- [ ] Automatic deploy uses exact `github.sha`, the approved immutable Canary digest, LAN game bind `192.168.1.2`, world id `1`, slug `oteryn-staging`, name `Oteryn Staging`, region `LAN`.
- [ ] Pull requests never deploy to Synology.
- [ ] The privileged deploy-runner image remains non-published on ordinary `main` pushes.
- [ ] Existing manual deploy/rollback remains available and production remains untouched.
- [ ] Focused Synology workflow contracts and repository-required exact-head checks pass.
- [ ] After protected integration, the merge SHA is built and automatically deployed successfully to Synology staging.

## Ownership

```yaml
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
modules:
  - Synology staging deployment
  - GitHub Actions deployment orchestration
dependencies:
  - existing .github/workflows/deploy-synology-staging.yml
  - approved immutable Canary image digest already used by staging operations
  - synology-staging GitHub Environment and platform-runners self-hosted execution path
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T18:05:29Z
head: 87e584275e492a865bc6c005061126fbec33b7be
branch: ci/20260907-auto-synology-staging
pr: none
status: implementing
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
proven:
  - Protected main was 87e584275e492a865bc6c005061126fbec33b7be at task admission.
  - Existing Build Synology Staging Images publishes exact SHA tags for Platform and Gateway on relevant main pushes but does not deploy them.
  - Existing Deploy Synology Staging is workflow_dispatch-only, serializes through synology-staging-deployment, validates exact release SHA, resolves immutable image digests, runs the guarded deploy/rollback scripts and removes its ephemeral environment file.
  - Historical trusted-main one-shot deployment used actions:write dispatch plus gh run watch successfully and established LAN game endpoint 192.168.1.2:7172 with world id 1.
  - Approved Canary staging image is ghcr.io/blakinio/canary@sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f.
derived:
  - Reusing the existing deploy workflow avoids duplicating secret handling, health checks, rollback and self-hosted cleanup semantics.
unknown:
  - Exact candidate CI and post-merge live staging deployment outcome are pending publication and integration.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - A new permanent one-shot workflow is required; the existing image-build workflow can own the dispatch job without increasing workflow inventory.
changed_paths: []
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation candidate not yet published
blockers:
  - none
next_action: Publish the build-workflow automation and focused contract test as one coherent candidate.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository protected PR path
source_branch_evidence: governing Issue #1313 and owner-requested durable staging automation
```

## Notes

Automatic deployment applies to runtime-affecting changes and Synology deployment/workflow package changes. Documentation-only commits do not rebuild or redeploy an unchanged runtime. Production deployment is excluded.