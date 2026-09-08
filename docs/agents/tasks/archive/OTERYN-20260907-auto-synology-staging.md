---
task_id: OTERYN-20260907-auto-synology-staging
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - issue #1313
  - pull request #1329
  - Deploy Synology Staging run 34167821638
optional_reads: []
---

# OTERYN-20260907 auto Synology staging — archived

## Terminal result

`REPAIR_COMPLETE`

Issue #1313 established and repaired automatic protected-main deployment to Synology staging. The live-proven runtime release is protected `main@ada4d5376817d1c8f439dacba88673b367a69668`, delivered by PR #1329 and verified by automatic Deploy Synology Staging run #24 (`34167821638`).

## Acceptance

- [x] Runtime-affecting protected-main changes build exact-SHA Platform/Gateway images and dispatch guarded Synology staging deployment.
- [x] Pull requests never deploy; ordinary main pushes do not publish the privileged deploy-runner.
- [x] Automatic deploy uses exact protected-main SHA, immutable runtime digests and the approved Canary digest.
- [x] Python is not required on the live Synology deployment runner.
- [x] Public staging origin is pinned to `https://oteryn.molehill.cloud`.
- [x] Recovery candidates remain fail-closed and require exact schema/image/world proof before resume/finalization.
- [x] Gateway readiness verifies both internal TLS dependency legs and aggregate readiness.
- [x] MFA health validation uses stable semantic markers rather than localized presentation copy.
- [x] Internal Nginx upstreams dynamically re-resolve Docker service addresses after container recreation.
- [x] Automatic run #24 finalized the prior candidate and deployed exact `main@ada4d5376817d1c8f439dacba88673b367a69668` successfully.
- [x] Platform, Gateway, Canary, canonical URL, cache-control, isolation, MFA QR, World Registry and LAN game endpoint checks passed live.
- [x] Rollback was not required and ephemeral deployment environment cleanup succeeded.
- [x] Production deployment and production policy remained untouched.
- [x] This closeout releases active task ownership and preserves the completed record here.

## Context checkpoint

```yaml
policy_version: 2
checkpoint_version: 1
updated_at: 2026-09-07T23:12:00+02:00
status: completed
phase: closeout
execution_mode: github_connector
execution_reason: terminal docs-only lifecycle closeout after successful exact-main Synology staging proof
task_kind: repair
implementation_authorized: true
validation_level: full
validation_intensity: HEIGHTENED
validation_risk: low
validation_triggers: staging-deployment, recovery, lifecycle-closeout
self_review_result: PASS
last_completed_step: exact protected-main ada4d5376817d1c8f439dacba88673b367a69668 deployed successfully to Synology staging in run 34167821638
issue: 1313
branch: docs/20260907-archive-auto-synology-staging
head: ada4d5376817d1c8f439dacba88673b367a69668
base_sha: ada4d5376817d1c8f439dacba88673b367a69668
pr: 1329
context_routes:
  - testing
  - execution-resources
  - agent-governance
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260907-auto-synology-staging.md
proven:
  - PR #1314 established automatic exact-main image build and guarded Synology staging dispatch.
  - PR #1317 removed the unnecessary live-runner Python dependency while preserving fail-closed IPv4 policy.
  - PR #1320 pinned the canonical public staging origin and removed environment URL drift.
  - PRs #1321, #1322 and #1325 hardened resumable recovery, exact candidate finalization and recovery Gateway reconstruction.
  - PR #1326 removed a localized-copy false negative from MFA health validation.
  - PR #1329 fixed the final runtime root cause by dynamically resolving Platform and Canary upstreams through Docker DNS after container recreation.
  - Deploy Synology Staging run 34167821638 targeted exact protected main ada4d5376817d1c8f439dacba88673b367a69668 and completed successfully on runner oteryn-synology-platform.
  - Run 34167821638 finalized prior candidate c15493a1c4c38762b486893a56b40ee6004f0381 only after its full staging health contract passed.
  - The same run deployed ada4d5376817d1c8f439dacba88673b367a69668 and verified Platform/Gateway/Canary bindings, both internal TLS dependencies, Gateway identity/readiness, MFA QR, canonical HTTPS origins, LAN endpoint 192.168.1.2:7172 and World Registry route 1 online with login enabled.
  - Terminal live marker was `Oteryn Synology staging deployment is healthy.` and rollback remained skipped.
  - PR #1330 was intentionally closed unmerged because #1329 supplied the more fundamental live-proven root-cause repair.
derived:
  - Runtime-affecting protected-main changes now have a verified automatic path to Synology staging while documentation-only closeout changes do not needlessly rebuild or redeploy unchanged runtime.
unknown: []
conflicts: []
first_failure:
  marker: historical-synology-auto-deploy-chain-failures
  evidence: earlier runs exposed runner Python, URL drift, recovery candidate, health-contract and stale Nginx upstream defects; all are superseded by the live-success evidence in run 34167821638.
rejected_hypotheses:
  - Increase deployment timeout as the primary repair; rejected after identifying stale Nginx upstream addresses as the final runtime root cause.
  - Delete recovery state manually; rejected in favor of fail-closed exact-candidate finalization.
  - Merge PR #1330 after #1329; rejected because #1329 already fixed the root cause and passed the full live staging contract.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
  - docs/agents/tasks/archive/OTERYN-20260907-auto-synology-staging.md
validation:
  - command: Deploy Synology Staging run 34167821638
    result: PASS
    evidence: exact main ada4d5376817d1c8f439dacba88673b367a69668; Deploy prebuilt images succeeded; terminal healthy marker present
  - command: live staging health contract in run 34167821638
    result: PASS
    evidence: both TLS dependencies, Gateway identity/readiness, MFA QR, canonical origins, LAN endpoint and World Registry passed
  - command: rollback path in run 34167821638
    result: PASS
    evidence: rollback correctly skipped because deployment succeeded; cleanup steps succeeded
blockers: []
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: archive-only closeout after live staging proof
source_branch_evidence: Issue #1313; Deploy Synology Staging run 34167821638
```

## Notes

The runtime deployed on Synology staging is `ada4d5376817d1c8f439dacba88673b367a69668`. The archive-only commit that integrates this historical record is intentionally documentation-only and therefore does not rebuild or redeploy unchanged runtime. Production deployment remains excluded.