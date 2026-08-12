---
task_id: OTERYN-20260812-synology-rollback-schema-safety
mode: implementation
branch: ops/synology-rollback-schema-safety-1007
status: validating
project_lane: oteryn-platform-core
---

# OTERYN-20260812 Synology rollback schema-safety

## Goal

Make Synology staging rollback truthful and schema-safe for Issue #1007 without production deployment or protected-environment mutation.

## Acceptance

- [x] Enforce expand/contract migration compatibility policy and fail closed rollback when compatibility cannot be proven.
- [x] Persist release SHA, immutable runtime image identities, schema compatibility identity, last-good identity and rollback eligibility.
- [x] Never represent image rollback as database schema rollback.
- [x] Provide bounded migration-bearing recovery backed by a pre-migration staging database backup and identity validation.
- [x] Pin health probe helper images by immutable digest at the shared Docker invocation boundary without weakening probes.
- [x] Add deterministic positive/negative contract tests.
- [ ] Obtain terminal green exact-head CI and complete fresh independent review.
- [ ] Squash merge, archive this task and close Issue #1007 only after terminal green exact-head validation.

## Ownership

```yaml
owned_paths:
  - deploy/synology/**
  - tests/ci/test_synology_rollback_contract.py
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - docs/agents/tasks/active/OTERYN-20260812-synology-rollback-schema-safety.md
  - docs/agents/tasks/archive/OTERYN-20260812-synology-rollback-schema-safety.md
excluded_paths:
  - .github/workflows/deploy-synology-staging.yml
modules:
  - synology-staging-deployment
blockers:
  - PR #1003 still owns .github/workflows/deploy-synology-staging.yml; this task has not edited that path.
cross_repository_tasks: []
```

The older `OTERYN-20260801-public-domain-repair` record's latest durable checkpoint has `branch: none`, `pr: none`, omits `health-check.sh` from current `owned_paths`, and explicitly releases its former implementation ownership.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-12T08:20:00Z
head: c93baa129de0015b0714461ca2b69549fdc37d21
branch: ops/synology-rollback-schema-safety-1007
pr: 1013
status: validating
context_routes:
  - ci-repair
  - testing
owned_paths:
  - deploy/synology/**
  - tests/ci/test_synology_rollback_contract.py
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - this task record
blockers:
  - PR #1003 owns .github/workflows/deploy-synology-staging.yml; workflow mutation remains deferred.
proven:
  - PR #1013 is mergeable and remains scoped to the dedicated task branch.
  - candidate release identity is derived from matching Platform/Gateway OCI revision labels and rejects disagreement with explicit release identity.
  - candidate migration compatibility metadata is now read from /var/www/html/deploy/synology/release-contract.env inside the exact Platform image, not from an independently checked-out workflow revision.
  - the image contract parser accepts only the three bounded compatibility keys and requires expand-contract policy plus bounded schema identifiers.
  - pre-existing running Platform/Gateway/Canary images are snapshotted before pull using their Docker image IDs resolved immediately to immutable RepoDigests, preventing mutable-tag drift.
  - when no managed current-release exists but a complete running-image snapshot exists, the old Platform/Gateway revision is verified and a synthetic observed-<sha> schema identity records the exact pre-migration DB/application pairing; that DB is backed up before migration.
  - an existing non-empty Platform DB without either managed state or a complete running-image baseline fails closed before migration.
  - legacy snapshot parsing occurs in a subshell so candidate image variables are not overwritten.
  - release-state metadata validates exact application SHA, immutable runtime image identities, schema compatibility identity, accepted schema identities and rollback eligibility.
  - deployment persists candidate identity before migration and writes schema state unknown before invoking migrate; known schema identity is persisted only after migrate succeeds.
  - rollback validates independent schema identity against last-good application compatibility before old images are started and explicitly does not change schema.
  - recover-schema.sh is explicit staging-only recovery and validates managed evidence/digest/transition identity before recreating only the staging Platform DB.
  - deterministic focused tests now include image-bound contract parsing, unexpected contract-key rejection, immutable legacy snapshot ordering, candidate-variable preservation, fail-closed missing baseline, world-name quoting and all prior rollback/recovery cases.
derived:
  - the two P1 findings from Codex review at fbbf519471 are addressed structurally: candidate metadata is image-bound and legacy migration cannot proceed without a backup-capable baseline.
unknown:
  - terminal result of exact-head CI generation for c93baa129de0015b0714461ca2b69549fdc37d21.
  - fresh independent review result on the final coherent head.
conflicts: []
review_findings_repaired:
  - schema-state remained known across failed destructive recovery; fixed by unknown-before-drop/known-after-complete-restore.
  - marketplace scheduler could write during restore; fixed by explicit stop and verification.
  - compatibility test called wrong subcommand; fixed and focused suite wired to Synology CI.
  - release-state world name with spaces was not shell-safe; fixed with %q serialization and round-trip test.
  - first managed migration lacked legacy baseline; fixed with immutable pre-pull running-image snapshot plus observed pre-migration schema identity and fail-closed no-baseline behavior.
  - release contract came from workflow checkout rather than selected image; fixed by reading the contract from the exact Platform image whose OCI revision is verified.
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/release-contract.env
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/recover-schema.sh
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/rollback.sh
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - tests/ci/test_synology_rollback_contract.py
  - this task record
validation:
  - command: prior exact-head focused Synology gate at fbbf519471
    result: PASS
    evidence: 14/14 tests, shell syntax, Compose/LAN validation and image builds passed before the two newest review findings.
  - command: exact-head CI generation c93baa129de0015b0714461ca2b69549fdc37d21
    result: PENDING
    evidence: workflow runs 31578320346/351/353/359/362/365/369/396 created for current head.
self_review:
  result: PENDING
  exact_head: c93baa129de0015b0714461ca2b69549fdc37d21
  acceptance_checked: true
  full_diff_checked: false
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: false
  findings: []
  evidence:
    - image-bound compatibility contract and legacy baseline repairs are present on current head.
next_action: inspect current-head focused Synology validation and full diff; repair only verified defects, then request/complete fresh exact-head Codex review before final CI/merge gate.
```
