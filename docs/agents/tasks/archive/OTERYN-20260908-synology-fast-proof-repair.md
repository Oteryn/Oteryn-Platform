---
task_id: OTERYN-20260908-synology-fast-proof-repair
governing_issue: 1341
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - Issue 1341 live proof evidence
  - current Synology impact routing and deploy workflow
optional_reads: []
---

# OTERYN-20260908-synology-fast-proof-repair

## Goal

Repair the two live bottlenecks exposed by the genuine protected-main Portal release `e242f0cd212abb3cad517d6dcbfde566e604f91d`: false full fallback when repository-only acceptance tests are present, and repeated network pulls of already proven reused immutable images.

## Acceptance criteria

- [x] presentation fast classification ignores explicit repository-only acceptance/test inputs while unknown or mixed runtime inputs remain fail-closed;
- [x] immutable runtime images are verified locally first and pulled only when the exact digest is absent or the reference is mutable;
- [x] changed/new Platform or Gateway images still require exact immutable digest availability and OCI revision verification;
- [x] persisted release lineage, source SHA, digest provenance, schema, rollback and protected-main guarantees remain unchanged;
- [x] exact-head Synology contracts, CI and `platform-gate` passed;
- [x] integration used the normal Merge Queue;
- [x] resulting-main control-plane release deliberately fell back to `full`, completed healthy and skipped rollback;
- [x] repair implementation ownership is released while Issue #1341 remains open for a later genuine qualifying `platform-fast` timing proof; no artificial product commit is used as proof.

## Ownership

```yaml
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-fast-proof-repair.md
modules:
  - Synology staging deployment routing
  - Synology staging provenance preflight
dependencies:
  - Issue 1341 remains live proof authority
  - Issue 1345 Platform reconcile implementation already integrated
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T13:19:00Z
head: 2c3c88c30b6da35124ade7ed82dbd20cf8c0f557
branch: perf/1341-synology-fast-proof-repair
pr: 1353
status: completed
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-fast-proof-repair.md
proven:
  - genuine Portal release e242f0cd212abb3cad517d6dcbfde566e604f91d built only Platform but Deploy Synology 34224564123 incorrectly selected full because scripts/acceptance/tests/portal-polish-quality.spec.mjs was classified as runtime
  - that failed fast proof spent 4m13s in immutable runtime provenance resolution and 10m03s in the full deploy body, then completed healthy with rollback skipped
  - canonical fast classification now treats deploy/synology/tests/* and scripts/acceptance/* as repository-only non-runtime while deployment-control and unknown paths still fail closed
  - deploy workflow now checks for each exact immutable @sha256 image locally and pulls only when the exact reference is absent; package ownership, source lineage and OCI revision validation still execute after acquisition
  - PR 1353 exact-head required workflows were green before protected integration, including CI platform-gate job 102068709247 and Build Synology focused contracts
  - PR 1353 integrated through normal Merge Queue and became protected main 2c3c88c30b6da35124ade7ed82dbd20cf8c0f557
  - resulting-main Build Synology Staging Images run 34229397630 completed success
  - resulting-main Deploy Synology Staging run 34229488962 job 102071751147 completed success with rollback skipped and final marker `Oteryn Synology staging deployment is healthy.`
  - resulting-main control-plane deploy correctly selected `Synology staging deploy profile: full` because accumulated impact included .github/workflows/deploy-synology-staging.yml
  - in that control-plane deploy, provenance resolution ran from 13:02:27 UTC to 13:04:13 UTC, about 1m45s; new Platform and Gateway images were pulled while the exact Canary digest was reused locally
  - the source repair branch was configured for automatic deletion after protected integration
derived:
  - the local immutable-image optimization is active in real staging execution: the exact Canary digest was reused without a network pull while new component digests still followed the pull path
  - the control-plane release cannot be used as a fast-path benchmark because its own deployment workflow change must select full by design
  - the next genuine presentation-only protected-main product release can test both repaired non-runtime classification and local reuse of unchanged Gateway/Canary identities without weakening safety
unknown:
  - corrected genuine `platform-fast` wall-time after this repair
conflicts: []
first_failure:
  marker: live-fast-proof-false-full
  evidence: Deploy Synology 34224564123 logged full because scripts/acceptance/tests/portal-polish-quality.spec.mjs was treated as runtime; this repair corrects that classification rather than removing the product test
rejected_hypotheses:
  - counting the earlier healthy full fallback as a fast proof would falsify live evidence
  - removing acceptance tests from product diffs would game the benchmark
  - skipping provenance validation entirely would weaken release safety
changed_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-fast-proof-repair.md
validation:
  - command: PR 1353 exact-head GitHub Actions
    result: PASS
    evidence: required exact-head workflows passed, including Synology contracts and platform-gate job 102068709247
  - command: PR 1353 protected Merge Queue integration
    result: PASS
    evidence: PR 1353 integrated through normal Merge Queue as protected main 2c3c88c30b6da35124ade7ed82dbd20cf8c0f557
  - command: resulting-main Build Synology Staging Images 34229397630
    result: PASS
    evidence: resulting-main deployment package validation/build/dispatch completed successfully
  - command: resulting-main Deploy Synology Staging 34229488962 job 102071751147
    result: PASS
    evidence: control-plane change selected full, exact Canary digest was reused locally, staging ended healthy and rollback was skipped
blockers: []
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1353 is terminal on protected main and the implementation branch has no continuing ownership purpose
source_branch_evidence: PR #1353 merged through normal protected integration as main 2c3c88c30b6da35124ade7ed82dbd20cf8c0f557
```

## Notes

This archive closes only the repair implementation ownership. Issue #1341 intentionally remains open for a later genuine qualifying `platform-fast` timing proof. Create a separate active measurement/closeout packet when such a real product release exists; do not reactivate this implementation task and do not manufacture a no-op product commit. No production deployment authority was granted or exercised.
