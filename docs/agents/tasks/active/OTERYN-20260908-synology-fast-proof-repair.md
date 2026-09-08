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

- [ ] presentation fast classification ignores explicit repository-only acceptance/test inputs while unknown or mixed runtime inputs remain fail-closed;
- [ ] reused immutable Gateway/Canary images are verified locally first and pulled only when the exact digest is absent;
- [ ] changed/new Platform or Gateway images still require exact immutable digest availability and OCI revision verification;
- [ ] persisted release lineage, source SHA, digest provenance, schema, rollback and protected-main guarantees remain unchanged;
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
  - tests/ci/test_synology_auto_staging_deploy.py
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
updated_at: 2026-09-08T12:27:00Z
head: e242f0cd212abb3cad517d6dcbfde566e604f91d
branch: perf/1341-synology-fast-proof-repair
pr: none
status: implementing
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-fast-proof-repair.md
proven:
  - protected main is e242f0cd212abb3cad517d6dcbfde566e604f91d at task start
  - genuine Portal PR 1348 merged through Merge Queue and Build Synology 34224431667 built only Platform
  - Deploy Synology 34224564123 completed healthy with rollback skipped but selected full because scripts/acceptance/tests/portal-polish-quality.spec.mjs was misclassified as runtime
  - the same run spent 4m13s in Resolve immutable runtime component provenance because the workflow unconditionally pulled Platform, reused Gateway and Canary
  - full Deploy prebuilt images then spent 10m03s
  - deploy-impact.sh already classifies scripts/acceptance/* as non-runtime, proving drift against canonical deploy.sh fast classification
  - exact reused Gateway digest and Canary digest are immutable and already part of the persisted/current deployment identity
unknown:
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
  - docs/agents/tasks/active/OTERYN-20260908-synology-fast-proof-repair.md
validation: []
blockers:
  - none
next_action: repair classifier and provenance preflight, add focused contracts, then open exact-head PR
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: repair implementation is active
source_branch_evidence: Issue #1341 and live Deploy Synology run 34224564123
```

## Notes

No production deployment, secret mutation, direct push to main, force-push, protected-environment bypass, or Merge Queue / `platform-gate` weakening is authorized.
