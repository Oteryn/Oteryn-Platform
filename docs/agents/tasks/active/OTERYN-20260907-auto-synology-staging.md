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

Governing GitHub Issue: #1313 — every runtime-affecting protected-main change is automatically built as exact-SHA images and made available on guarded Synology staging, with fail-closed recovery, rollback, health and identity validation.

## Acceptance criteria

- [x] Protected-main runtime/Synology changes build exact-SHA Platform/Gateway images and dispatch the guarded staging workflow.
- [x] Pull requests never deploy; privileged deploy-runner remains manual-only.
- [x] Pythonless runner validation and canonical public staging origin are integrated.
- [x] Candidate recovery remains fail-closed and requires exact schema/image/world proof.
- [x] Previous candidate can be reconstructed/finalized only after the complete health contract passes.
- [x] Run #23 finalized the historical `bbeb084b...` candidate with full health PASS.
- [x] Run #23 proved both current-release internal TLS dependencies healthy and isolated current Gateway aggregate `/ready` as the remaining failure.
- [x] Runtime Gateway code and Dockerfile are identical between the proven `bbeb084b...` candidate and `c15493a1...`; the defect is deployment orchestration, not application logic.
- [x] PR #1330 makes normal current-release startup use the same proven no-dependency proxy/Gateway recreation order as candidate finalization.
- [x] PR #1330 bounds published Gateway readiness directly through the already-verified loopback binding instead of spawning one helper container per attempt.
- [ ] PR #1330 passes exact-head repository checks and protected Merge Queue.
- [ ] The resulting protected-main SHA resumes/finalizes the current recovery candidate, deploys exact main, and passes the complete live Synology staging health contract.
- [ ] Archive this task after live proof; close Issue #1313 only after archive integration.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_rollback_contract.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
modules:
  - Synology staging deployment
  - staging health contract
  - recovery candidate verification
  - current-runtime startup sequencing
dependencies:
  - PR #1314 protected-main auto-deploy foundation
  - PR #1317 Pythonless runtime validation
  - PR #1320 canonical public origin
  - PR #1321 resumable deployment and dependency diagnostics
  - PR #1322 candidate finalization
  - PR #1325 recovery Gateway reconstruction
  - PR #1326 semantic MFA health validation
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
updated_at: 2026-09-07T22:44:00Z
head: af606d75a3d69d64e23773137578bda6040369fd
branch: fix/20260907-synology-runtime-start-order
pr: 1330
status: validating
context_routes:
  - testing
  - execution-resources
owned_paths:
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_rollback_contract.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
proven:
  - Protected main is c15493a1c4c38762b486893a56b40ee6004f0381 from PR #1326.
  - Deploy run 34165164885 targeted exact main c15493a1c4c38762b486893a56b40ee6004f0381.
  - Run 34165164885 passed runner tools, GHCR, config and immutable image resolution.
  - The previous bbeb084b0a8da2c4640fa6ad7e2e542f039bf047 candidate passed bindings, both internal TLS dependencies, Gateway identity/readiness/login isolation, MFA renderer/assets, canonical URL generation and LAN endpoint, then was explicitly finalized.
  - The current c15493a1 release started and passed both internal TLS dependency probes but Gateway /ready returned HTTP 503 until the 30-minute workflow timeout cancelled the job.
  - Comparing bbeb084b0a8da2c4640fa6ad7e2e542f039bf047..c15493a1c4c38762b486893a56b40ee6004f0381 shows no game-gateway source or Dockerfile changes; only Synology deployment/test files changed.
  - Proven candidate finalization recreates internal-proxy and gateway together with --no-deps; the normal current path separately recreated internal-proxy then ran `up -d gateway`, causing Compose to revisit dependencies after migrations.
  - PR #1330 replaces that dependency-churning sequence with one `up -d --no-deps --force-recreate internal-proxy gateway` command.
  - PR #1330 probes published Gateway health/readiness/version through exact loopback with 12 bounded attempts and preserves direct TLS probes plus aggregate `/ready` as blocking gates.
  - Temporary repair run 34167435972 and alignment run 34167574822 passed shell syntax plus auto-staging, rollback and recovery focused contracts before final publication.
derived:
  - The remaining 503 is caused by second-phase deployment sequencing/runtime state, not a Gateway code delta.
  - Removing post-migration dependency re-walk also removes unnecessary Platform/tls-init churn and materially reduces deployment wall time.
unknown:
  - Final exact-head PR #1330 checks and post-merge live Synology result are pending.
conflicts: []
first_failure:
  marker: CURRENT_RELEASE_GATEWAY_READY_AFTER_DEPENDENCY_CHURN
  evidence: Run 34165164885 finalized the previous candidate successfully, then current release passed both internal TLS probes but `/ready` stayed 503 until the GitHub job timeout.
rejected_hypotheses:
  - MFA health false negative remains; run #23 passed the corrected QR-first MFA asset check.
  - Gateway application code changed; the relevant source and Dockerfile are identical between proven candidate and current release.
  - Either internal TLS leg is broken; both passed for current release before aggregate readiness failed.
  - Raise the global job timeout; rejected in favor of correcting startup order and making readiness fail fast with diagnostics.
changed_paths:
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_rollback_contract.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
validation:
  - command: Deploy Synology Staging run 34165164885
    result: FAIL
    evidence: previous candidate finalized; current release TLS dependencies PASS; aggregate readiness 503 until 30-minute cancellation
  - command: Temporary Synology Runtime Start Repair run 34167435972 attempt 2
    result: PASS
    evidence: shell syntax plus auto-staging, rollback and rollback/recovery contracts passed
  - command: Temporary Synology Rollback Probe Alignment run 34167574822
    result: PASS
    evidence: aligned focused rollback and auto-staging contracts passed
  - command: repository-hosted PR #1330 exact-head checks
    result: NOT_RUN
    evidence: checks start after this checkpoint publication
blockers: []
next_action: Validate PR #1330. After protected integration, verify automatic exact-main Synology deployment reaches full health; then archive this task packet.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository protected repair PR path
source_branch_evidence: governing Issue #1313 and PR #1330
```

## Notes

Automatic deployment applies to runtime-affecting changes and Synology deployment/workflow package changes. Documentation-only commits do not rebuild or redeploy unchanged runtime. Production deployment remains excluded.