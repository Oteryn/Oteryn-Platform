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
- [x] Successful exact-main image builds automatically dispatch the guarded Synology staging deployment.
- [x] Automatic deploy uses exact release identity, approved immutable Canary, LAN game bind and canonical staging world identity.
- [x] Pull requests never deploy and ordinary main pushes do not publish the privileged deploy-runner.
- [x] Pythonless runner validation, canonical public URL, safe same-candidate resume, duplicate-pull removal and dependency-level readiness diagnostics are integrated.
- [x] A newer release refuses to overwrite a different unresolved candidate.
- [x] PR #1322 adds fail-closed finalization of a prior migrated candidate only after exact running-image/world/schema identity and the complete staging health contract pass.
- [ ] PR #1322 passes repository-required exact-head checks and protected integration.
- [ ] The resulting protected-main SHA automatically finalizes the old `bbeb084b...` candidate, deploys the newest exact main SHA, and passes the complete Synology staging health contract.
- [ ] This task is archived only after live proof and Issue #1313 is closed completed afterward.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/deploy.sh
  - tests/ci/test_synology_rollback_recovery_contract.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
modules:
  - Synology staging deployment
  - deployment recovery and rollback safety
  - Gateway dependency readiness diagnostics
dependencies:
  - protected-main auto-deploy foundation from PR #1314
  - Pythonless runtime validation from PR #1317
  - canonical public origin from PR #1320
  - resumable/diagnostic deployment from PR #1321
  - approved immutable Canary staging digest
  - synology-staging environment and platform-runners execution path
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T20:40:00Z
head: 5b13770d94ad149c2e30a5a052738cef6080ff82
branch: fix/20260907-synology-finalize-stale-candidate
pr: 1322
status: validating
context_routes:
  - testing
  - execution-resources
owned_paths:
  - deploy/synology/scripts/deploy.sh
  - tests/ci/test_synology_rollback_recovery_contract.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
proven:
  - PR #1321 passed protected integration and is current main SHA febbfde9f9e122fbdf1a835a4ffb563d34b867f1.
  - Post-integration Build Synology Staging Images run 34159592889 published exact-main Platform/Gateway images and dispatched Deploy Synology Staging run 34159643562.
  - Deploy run 34159643562 passed runner tools, GHCR login, configuration validation, immutable image resolution and ephemeral env creation.
  - That live deploy then correctly refused the newer febbfde9 release because migrated-but-unpromoted candidate bbeb084b0a8da2c4640fa6ad7e2e542f039bf047 still owns recovery state.
  - The surviving bbeb candidate originated from run 34155125960, whose migration completed before its post-migration Gateway readiness failure; recovery state was intentionally preserved.
  - PR #1322 never blindly deletes that state. It requires schema-state=known for the exact old candidate, exact accepted schema, exact staging world identity and exact running Platform/Gateway/Canary immutable identities.
  - PR #1322 reconstructs a permission-restricted old-candidate health environment, recreates only internal-proxy/Gateway for that identity, runs the complete current staging health contract, and re-runs idempotent world registration before promotion.
  - Only after full health success may the old candidate become current, prior current become last-good when distinct, and candidate recovery state be removed; normal newest-main deployment then continues.
  - Temporary construction run 34160086400 passed shell syntax, rollback, rollback-recovery, auto-staging and fresh-baseline focused suites before publication.
derived:
  - Automatic deployment needs a safe candidate-finalization transition, not manual recovery-state deletion, because newer main cannot legitimately take ownership while a different candidate remains unresolved.
unknown:
  - Whether the old bbeb candidate now passes direct Canary issuer, internal TLS and aggregate Gateway readiness after the forced TLS refresh and dependency-level diagnostics from PR #1321.
  - Exact PR #1322 repository CI and the subsequent live two-stage deployment result are pending.
conflicts: []
first_failure:
  marker: DIFFERENT_PROVEN_CANDIDATE_BLOCKS_NEW_MAIN
  evidence: Deploy Synology Staging run 34159643562 rejected requested febbfde9f9e122fbdf1a835a4ffb563d34b867f1 because unresolved candidate bbeb084b0a8da2c4640fa6ad7e2e542f039bf047 still owns recovery state.
rejected_hypotheses:
  - Delete candidate-release.env manually; rejected because that would discard rollback/recovery ownership without health proof.
  - Permit a newer SHA to overwrite candidate metadata; rejected because it would destroy exact recovery identity.
  - Weaken Gateway readiness; rejected because the login/session dependency contract must remain blocking.
changed_paths:
  - deploy/synology/scripts/deploy.sh
  - tests/ci/test_synology_rollback_recovery_contract.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
validation:
  - command: Deploy Synology Staging run 34159643562
    result: FAIL
    evidence: fail-closed different-candidate ownership rejection after all pre-deploy identity gates passed
  - command: Temporary Synology Candidate Finalizer Writer run 34160086400
    result: PASS
    evidence: Bash syntax plus rollback, recovery, auto-staging and fresh-baseline focused suites passed
  - command: repository-hosted PR #1322 exact-head checks
    result: NOT_RUN
    evidence: checks start after this checkpoint publication
blockers:
  - none
next_action: Validate PR #1322. After protected integration, verify the automatic Synology deployment finalizes the prior candidate and passes the newest exact-main health contract; then archive this task packet.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository protected repair PR path
source_branch_evidence: governing Issue #1313 and PR #1322
```

## Notes

Automatic deployment applies to runtime-affecting changes and Synology deployment/workflow package changes. Documentation-only commits do not rebuild or redeploy unchanged runtime. Production deployment remains excluded.