---
task_id: OTERYN-20260907-auto-synology-staging
governing_issue: 1313
terminal_pr_policy: archive_pending
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

- [x] Runtime-affecting `main` pushes and Synology workflow/package changes build exact-SHA Platform/Gateway images.
- [x] A successful exact-main image build automatically dispatches the existing `Deploy Synology Staging` workflow and waits for its terminal result.
- [x] Automatic deploy uses exact `github.sha`, the approved immutable Canary digest, LAN game bind `192.168.1.2`, world id `1`, slug `oteryn-staging`, name `Oteryn Staging`, region `LAN`.
- [x] Pull requests never deploy to Synology.
- [x] The privileged deploy-runner image remains non-published on ordinary `main` pushes.
- [x] Existing manual deploy/rollback remains available and production remains untouched.
- [x] The Pythonless runner repair passed exact-head checks and merged through protected Merge Queue.
- [ ] The canonical-origin repair passes exact-head checks and protected Merge Queue.
- [ ] After protected repair integration, the merge SHA is built and automatically deployed successfully to Synology staging.

## Ownership

```yaml
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
modules:
  - Synology staging deployment
  - GitHub Actions deployment orchestration
dependencies:
  - protected main automation delivered by PR #1314
  - Pythonless runtime validation delivered by PR #1317
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
updated_at: 2026-09-07T19:08:00Z
head: 67ba1b4e3bcf91e2b7bbc835bb1c182b8e9ac227
branch: fix/20260907-synology-canonical-app-url
pr: 1320
status: validating
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
proven:
  - PR #1314 merged through protected Merge Queue as main SHA c1e516735613a35240114b23d950b4b30aebcab2 and established automatic exact-main image build plus Synology dispatch.
  - Deploy run 34151852644 exposed the first live blocker: `python3: command not found` on oteryn-synology-platform before runtime mutation.
  - PR #1317 removed the unnecessary Python runtime dependency, passed 9/9 exact-head workflows and Merge Queue, and merged as main SHA 4be6731b4055e0c535618416f642b75ee11eb5a8.
  - Post-merge build run 34153848586 published exact-SHA Platform/Gateway images for 4be6731b4055e0c535618416f642b75ee11eb5a8 and dispatched Deploy Synology Staging run 34153905467.
  - Deploy run 34153905467 passed runner-tool validation and GHCR login, then failed before image resolution/runtime mutation because Environment variable OTERYN_STAGING_APP_URL was stale at http://127.0.0.1:8000 while repository policy requires https://oteryn.molehill.cloud.
  - The repository already treats https://oteryn.molehill.cloud as the canonical public staging origin; the stale Environment variable is redundant configuration drift.
  - PR #1320 is the live protected-main repair surface for branch fix/20260907-synology-canonical-app-url.
  - PR #1320 removes deploy-time dependence on vars.OTERYN_STAGING_APP_URL and writes CANONICAL_PUBLIC_APP_URL directly into the ephemeral staging environment.
derived:
  - The main-to-Synology orchestration, image publication and self-hosted runner routing are functioning; the remaining known blocker is stale environment metadata, not runtime image identity.
unknown:
  - Exact PR #1320 CI result and post-merge automatic Synology deploy/health result are pending.
conflicts: []
first_failure:
  marker: SYNOLOGY_STAGING_APP_URL_DRIFT
  evidence: Deploy Synology Staging run 34153905467 job 101841630509 reported APP_URL_INPUT=http://127.0.0.1:8000 and failed the canonical-origin check before image resolution or runtime mutation.
rejected_hypotheses:
  - Python remains required on the live runner; run 34153905467 passed Validate runner tools after PR #1317.
  - GHCR authentication is broken; run 34153905467 passed Log in to GHCR.
  - Exact-main image publication failed; run 34153848586 successfully built Platform and Gateway before dispatch.
changed_paths:
  - .github/workflows/deploy-synology-staging.yml
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
validation:
  - command: PR #1317 exact-head workflows and Merge Queue
    result: PASS
    evidence: 9/9 exact-head workflows passed; merge-group CI 34153685258 passed; merged as 4be6731b4055e0c535618416f642b75ee11eb5a8
  - command: Deploy Synology Staging run 34153905467 / job 101841630509
    result: FAIL
    evidence: canonical public origin drift in stale OTERYN_STAGING_APP_URL; runtime mutation did not begin
  - command: repository-hosted PR #1320 exact-head checks
    result: NOT_RUN
    evidence: checks start after this checkpoint publication
blockers:
  - none
next_action: Validate PR #1320, integrate through protected Merge Queue, verify exact-main automatic Synology deployment and health checks, then archive this task packet and close Issue #1313.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository protected repair PR path
source_branch_evidence: governing Issue #1313 and PR #1320
```

## Notes

Automatic deployment applies to runtime-affecting changes and Synology deployment/workflow package changes. Documentation-only commits do not rebuild or redeploy an unchanged runtime. Production deployment is excluded.