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
- [x] Pythonless runner repair merged through protected Merge Queue.
- [x] Canonical public staging origin repair merged through protected Merge Queue.
- [x] Failed exact candidates can be resumed only after the same SHA/images/world identity and completed schema transition are proven.
- [x] Deployment no longer performs a redundant second full Compose pull after exact runtime images were already resolved.
- [x] Gateway readiness diagnostics prove Canary issuer and both internal TLS dependency legs before aggregate `/ready`.
- [ ] PR #1321 passes repository-required exact-head checks and protected Merge Queue.
- [ ] The protected merge SHA is built and automatically deployed successfully to Synology staging with the full health contract passing.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/prepare-fresh-schema-baseline.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
modules:
  - Synology staging deployment
  - deployment recovery and rollback safety
  - Gateway dependency readiness diagnostics
dependencies:
  - protected main automation delivered by PR #1314
  - Pythonless runtime validation delivered by PR #1317
  - canonical public origin delivered by PR #1320
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
updated_at: 2026-09-07T20:18:00Z
head: 07d87c839dea10613c202685db8f5f02cf0bba7a
branch: fix/20260907-synology-deploy-resume-readiness
pr: 1321
status: validating
context_routes:
  - testing
  - execution-resources
owned_paths:
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/prepare-fresh-schema-baseline.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
proven:
  - PR #1314 merged as c1e516735613a35240114b23d950b4b30aebcab2 and established automatic exact-main image build plus Synology dispatch.
  - PR #1317 merged as 4be6731b4055e0c535618416f642b75ee11eb5a8 and removed the unnecessary live-runner Python dependency.
  - PR #1320 passed 8/8 exact-head workflows plus Merge Queue and merged as bbeb084b0a8da2c4640fa6ad7e2e542f039bf047, pinning the canonical public staging origin to https://oteryn.molehill.cloud.
  - Post-merge Build Synology Staging Images run 34155083384 published exact-SHA Platform/Gateway images for bbeb084b0a8da2c4640fa6ad7e2e542f039bf047 and dispatched Deploy Synology Staging run 34155125960.
  - Deploy run 34155125960 first attempt passed runner tools, GHCR login, deployment configuration, immutable runtime digest resolution and staging env creation; it completed the historical Platform migration backlog and reached runtime health validation.
  - That first attempt failed because Gateway /health was available but aggregate Gateway /ready returned HTTP 503 through its full retry window.
  - The retry of the same run failed earlier with `Deployment rejected: unresolved candidate release bbeb084b0a8da2c4640fa6ad7e2e542f039bf047 still owns recovery evidence.` after the prior migration had completed.
  - The retry also proved a second full `docker compose pull` was redundant and consumed material deployment time even with runtime images already resolved.
  - PR #1321 permits candidate resume only for the exact same release SHA, exact immutable Platform/Gateway/Canary identities, exact staging world identity and schema-state=known for that release; recovery evidence remains until full health success.
  - PR #1321 removes the second full Compose pull, force-refreshes TLS bootstrap, and adds explicit Canary issuer plus internal TLS dependency probes before Gateway aggregate readiness.
  - Temporary construction run 34158731681 passed Bash syntax checks plus tests/ci/test_synology_rollback_contract.py and tests/ci/test_synology_auto_staging_deploy.py before the final repair commit was published.
derived:
  - The remaining deploy defect is not main dispatch, image publication, runner tooling, GHCR authentication or canonical URL validation.
  - Aggregate Gateway readiness needs dependency-level evidence; preserving `/ready` as a blocking gate is required because weakening it could expose broken login/session behavior.
unknown:
  - Whether the live failing dependency is Gateway -> Platform internal TLS, Gateway -> Canary session issuer internal TLS, or Gateway aggregate behavior after both dependencies are healthy; PR #1321 makes the next live run distinguish these cases.
  - Exact PR #1321 repository CI and post-merge live deployment result are pending.
conflicts: []
first_failure:
  marker: GATEWAY_AGGREGATE_READINESS_503
  evidence: Deploy Synology Staging run 34155125960 reached full runtime validation and failed because Gateway /ready returned HTTP 503 while Gateway /health responded successfully.
rejected_hypotheses:
  - Main-to-Synology dispatch is broken; the exact main run dispatched the live workflow successfully.
  - Runtime image identity is missing; exact Platform/Gateway/Canary digest resolution passed.
  - Python or stale APP_URL still blocks deployment; both corresponding validation steps passed after PRs #1317 and #1320.
  - Re-running the same failed candidate is safe without state proof; existing recovery evidence correctly prevents blind overwrite, so PR #1321 permits only a fully proven exact-candidate resume.
changed_paths:
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/prepare-fresh-schema-baseline.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
validation:
  - command: Deploy Synology Staging run 34155125960 first attempt
    result: FAIL
    evidence: aggregate Gateway /ready remained HTTP 503 after runtime migration/startup
  - command: Deploy Synology Staging run 34155125960 retry
    result: FAIL
    evidence: exact failed candidate could not resume because surviving recovery evidence was unconditionally rejected
  - command: Temporary Synology Deploy Repair Writer run 34158731681
    result: PASS
    evidence: Bash syntax plus focused rollback and auto-staging contract suites passed before publication
  - command: repository-hosted PR #1321 exact-head checks
    result: NOT_RUN
    evidence: checks start after this checkpoint publication
blockers:
  - none
next_action: Validate PR #1321, integrate through protected Merge Queue, run the automatic exact-main Synology deployment, repair any dependency identified by the new direct readiness probes, and archive this task only after the complete live health contract passes.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository protected repair PR path
source_branch_evidence: governing Issue #1313 and PR #1321
```

## Notes

Automatic deployment applies to runtime-affecting changes and Synology deployment/workflow package changes. Documentation-only commits do not rebuild or redeploy an unchanged runtime. Production deployment is excluded.