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
- [ ] Focused Synology workflow contracts and repository-required exact-head checks pass for the Pythonless repair.
- [ ] After protected repair integration, the merge SHA is built and automatically deployed successfully to Synology staging.

## Ownership

```yaml
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/validate-ipv4.sh
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
modules:
  - Synology staging deployment
  - GitHub Actions deployment orchestration
dependencies:
  - protected main automation delivered by PR #1314
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
updated_at: 2026-09-07T18:54:00Z
head: fb90aecc3bbf070d539b0af4e32e2c6b078d13b3
branch: fix/20260907-synology-runner-pythonless
pr: 1317
status: validating
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/validate-ipv4.sh
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
proven:
  - PR #1314 merged through protected Merge Queue as main SHA c1e516735613a35240114b23d950b4b30aebcab2.
  - Main push run 34151778902 built and published exact-SHA Platform and Gateway images and successfully dispatched Deploy Synology Staging.
  - Deploy Synology Staging run 34151852644 targeted exact main SHA c1e516735613a35240114b23d950b4b30aebcab2 on runner oteryn-synology-platform in platform-runners.
  - That live deployment stopped before GHCR login or runtime mutation because Validate runner tools reported `python3: command not found`.
  - The failed deploy still executed its ephemeral environment cleanup and GHCR logout cleanup path.
  - The bounded repair validation run 34153037258 proved shell syntax and the focused Pythonless IPv4 contract suite before publication.
  - PR #1317 is the live protected-main repair surface for branch fix/20260907-synology-runner-pythonless.
  - The repair removes Python only from staging runtime validation and preserves exact loopback plus loopback/RFC1918 game-bind restrictions through a shared Bash validator.
derived:
  - The main-to-Synology orchestration is functioning; the remaining defect is a runtime-tool assumption in the deploy path, not image publication or dispatch.
unknown:
  - Exact PR #1317 CI result and post-merge automatic Synology deploy/health result are pending.
conflicts: []
first_failure:
  marker: SYNOLOGY_RUNNER_PYTHON_MISSING
  evidence: Deploy Synology Staging run 34151852644 job 101835612518 failed in Validate runner tools with `python3: command not found` before runtime mutation.
rejected_hypotheses:
  - The automatic dispatch failed; run 34151778902 successfully created and monitored deploy run 34151852644.
  - Platform or Gateway exact-SHA images were missing; both image publication jobs succeeded before dispatch.
  - Installing Python on the NAS is required; the deployment uses Python only for IPv4 validation and can preserve the same fail-closed policy in Bash.
changed_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/validate-ipv4.sh
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
validation:
  - command: Deploy Synology Staging run 34151852644 / job 101835612518
    result: FAIL
    evidence: live runner lacks python3; no deployment mutation occurred
  - command: Temporary bounded repair validation run 34153037258
    result: PASS
    evidence: Bash syntax and focused Pythonless Synology contract tests passed before publication
  - command: repository-hosted PR #1317 exact-head checks
    result: NOT_RUN
    evidence: checks start after this checkpoint publication
blockers:
  - none
next_action: Validate PR #1317, integrate through protected Merge Queue, verify the automatic exact-main Synology deployment and health path, then archive this task packet.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository protected repair PR path
source_branch_evidence: governing Issue #1313 and PR #1317
```

## Notes

Automatic deployment applies to runtime-affecting changes and Synology deployment/workflow package changes. Documentation-only commits do not rebuild or redeploy an unchanged runtime. Production deployment is excluded.