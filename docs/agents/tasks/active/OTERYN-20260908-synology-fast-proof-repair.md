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
- [x] exact-head Synology contracts, CI and `platform-gate` pass;
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
updated_at: 2026-09-08T12:54:00Z
head: 3f8b1938480487c7e122d1ccde4f35ab8d39a850
branch: perf/1341-synology-fast-proof-repair
pr: 1353
status: ready
terminal_pr_policy: archive_pending
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-fast-proof-repair.md
proven:
  - protected main was e242f0cd212abb3cad517d6dcbfde566e604f91d at task start and later advanced independently through portal lifecycle closeout PR 1352 to 2fcf49feb0ca1b870265673708ac38266c206e44
  - genuine Portal PR 1348 merged through Merge Queue and Build Synology 34224431667 built only Platform
  - Deploy Synology 34224564123 completed healthy with rollback skipped but selected full because scripts/acceptance/tests/portal-polish-quality.spec.mjs was misclassified as runtime
  - the same run spent 4m13s in Resolve immutable runtime component provenance because the workflow unconditionally pulled Platform, reused Gateway and Canary
  - full Deploy prebuilt images then spent 10m03s
  - deploy-impact.sh already classifies scripts/acceptance/* as non-runtime, proving drift against canonical deploy.sh fast classification
  - canonical fast classifier now explicitly treats deploy/synology/tests/* and scripts/acceptance/* as repository-only while preserving control/unknown fallback
  - provenance preflight now uses an exact @sha256 local image only when docker image inspect proves that exact reference exists; otherwise it pulls normally
  - Platform/Gateway repository-package digest validation and OCI revision checks remain after acquisition
  - initial PR head 651c1d0287ebbb179968ac04f98fb75ce20cb23b exposed only governance lifecycle/schema findings; no Synology rollback failure occurred
  - repair packet checkpoint schema and validation result were corrected; independent canonical portal closeout PR 1352 removed the terminal #1348 active-task residue
  - exact head 3f8b1938480487c7e122d1ccde4f35ab8d39a850 has Agent Governance 34228376532 success, Synology Rollback Contract 34228376319 success, CodeQL 34228376392 success, Edge Security Emulation 34228376352 success, Platform DB Outage Validation 34228376415 success, Build Synology Staging Images 34228376398 success, Game Auth Ticket Concurrency 34228376359 success, Phase 7 Production-Like Validation 34228376412 success, and CI 34228376331 success
  - exact-head Build Synology validation job 102068028530 passed shell syntax and all Synology contracts including synology fresh baseline contract PASS with 14 tests
  - exact-head required platform-gate job 102068709247 completed success
  - PR 1353 has zero inline review threads and zero discussion comments
derived:
  - repository-only acceptance and Synology contract tests can be ignored by runtime-impact classification without weakening unknown-path fail-closed behavior because they are not copied into the deployed Platform image and do not alter staging control inputs
  - local reuse of an exact immutable @sha256 image is provenance-equivalent to re-pulling that same digest once repository/package ownership, source lineage and OCI revision checks still run; network pull remains the fallback when the exact local reference is unavailable
  - the genuine e242f0cd release remains a failed fast-path proof even though the deployment itself was healthy, because the selected profile was full
unknown:
  - resulting-main full control-plane deployment result after protected integration of PR 1353
  - corrected later genuine platform-fast wall-time
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
  - command: PR 1353 exact-head GitHub Actions on 3f8b1938480487c7e122d1ccde4f35ab8d39a850
    result: PASS
    evidence: all nine triggered workflows completed success; Build Synology 34228376398 validated shell syntax and Synology contracts, Phase 7 34228376412 passed, CI 34228376331 passed and platform-gate job 102068709247 succeeded
blockers:
  - none
next_action: allow final archive-pending checkpoint head to rerun exact-head checks, then use normal Merge Queue; after protected integration require resulting-main full control-plane deployment to finish healthy before a genuine product benchmark
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: delete the repair source branch only after protected Merge Queue integration
source_branch_evidence: PR #1353 exact-head 3f8b1938480487c7e122d1ccde4f35ab8d39a850 is fully validated and entering terminal archive-pending transition
```

## Notes

No production deployment, secret mutation, direct push to main, force-push, protected-environment bypass, or Merge Queue / `platform-gate` weakening is authorized.
