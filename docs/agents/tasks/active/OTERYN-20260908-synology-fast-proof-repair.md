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
- [x] persisted release lineage, source SHA, digest provenance, schema, rollback and protected-main guarantees remain unchanged by design;
- [ ] exact-head Synology contracts, CI and `platform-gate` pass;
- [ ] integration uses normal Merge Queue;
- [ ] resulting-main control-plane release deliberately falls back to full and remains healthy before any product benchmark;
- [ ] a later genuine qualifying product release proves the corrected fast profile and records real wall-time.

## Ownership

```yaml
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-fast-proof-repair.md
modules:
  - Synology staging deployment routing
  - Synology staging provenance preflight
dependencies:
  - Issue 1341 implementation already integrated
  - Issue 1345 Platform reconcile implementation already integrated
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T12:48:00Z
head: 651c1d0287ebbb179968ac04f98fb75ce20cb23b
branch: perf/1341-synology-fast-proof-repair
pr: 1353
status: validating
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-fast-proof-repair.md
proven:
  - protected main was e242f0cd212abb3cad517d6dcbfde566e604f91d at task start
  - genuine Portal PR 1348 merged through Merge Queue and Build Synology 34224431667 built only Platform
  - Deploy Synology 34224564123 completed healthy with rollback skipped but selected full because scripts/acceptance/tests/portal-polish-quality.spec.mjs was misclassified as runtime
  - the same run spent 4m13s in Resolve immutable runtime component provenance because the workflow unconditionally pulled Platform, reused Gateway and Canary
  - full Deploy prebuilt images then spent 10m03s
  - deploy-impact.sh already classifies scripts/acceptance/* as non-runtime, proving drift against canonical deploy.sh fast classification
  - canonical fast classifier now explicitly treats deploy/synology/tests/* and scripts/acceptance/* as repository-only while preserving control/unknown fallback
  - provenance preflight now uses an exact @sha256 local image only when docker image inspect proves that exact reference exists; otherwise it pulls normally
  - Platform/Gateway repository-package digest validation and OCI revision checks remain after acquisition
  - PR 1353 is the canonical repair PR
derived:
  - repository-only acceptance and Synology contract tests can be ignored by runtime-impact classification without weakening unknown-path fail-closed behavior because they are not copied into the deployed Platform image and do not alter staging control inputs
  - local reuse of an exact immutable @sha256 image is provenance-equivalent to re-pulling that same digest once repository/package ownership, source lineage and OCI revision checks still run; network pull remains the fallback when the exact local reference is unavailable
  - the genuine e242f0cd release remains a failed fast-path proof even though the deployment itself was healthy, because the selected profile was full
unknown:
  - exact-head CI result for PR 1353 after checkpoint repair
  - corrected resulting-main and later genuine platform-fast wall-times
conflicts: []
first_failure:
  marker: live-fast-proof-false-full
  evidence: Deploy Synology 34224564123 logged `Synology staging deploy profile: full (accumulated runtime impact includes scripts/acceptance/tests/portal-polish-quality.spec.mjs).`
rejected_hypotheses:
  - counting the full fallback as fast-path acceptance would falsify the live evidence
  - removing acceptance tests from the product PR would game the benchmark rather than fix the classifier
  - skipping immutable provenance validation entirely would weaken deployment safety
changed_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-fast-proof-repair.md
validation:
  - command: live protected-main product release e242f0cd212abb3cad517d6dcbfde566e604f91d
    result: FAIL
    evidence: Build 34224431667 was Platform-only and Deploy 34224564123 was healthy, but the release selected full because a scripts/acceptance path was misclassified; provenance took 4m13s and the full deploy step took 10m03s, so this is not accepted as a fast-path proof
blockers:
  - none
next_action: allow exact-head validation to rerun; independently let the existing canonical #1344 closeout worker remove the terminal portal active-task residue, then proceed through normal Merge Queue only when all final checks are green
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: repair PR 1353 is active
source_branch_evidence: Issue #1341; PR #1353; branch perf/1341-synology-fast-proof-repair
```

## Notes

No production deployment, secret mutation, direct push to main, force-push, protected-environment bypass, or Merge Queue / `platform-gate` weakening is authorized.
