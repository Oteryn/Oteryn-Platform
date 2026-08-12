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
updated_at: 2026-08-12T08:29:00Z
head: 1d6486f7ac1891456ec0be63cc0f983affe4e248
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
  - candidate migration compatibility metadata is read from /var/www/html/deploy/synology/release-contract.env inside the exact Platform image rather than an independently checked-out workflow revision.
  - the image contract parser accepts only the three bounded compatibility keys and requires expand-contract policy plus bounded schema identifiers.
  - pre-existing running Platform/Gateway/Canary images are snapshotted before pull using Docker image IDs resolved immediately to immutable RepoDigests.
  - when no managed current-release exists but a complete running-image snapshot exists, the old Platform/Gateway revision is verified and a synthetic observed-<sha> identity records the exact pre-migration DB/application pairing before backup and migration.
  - an existing non-empty Platform DB without either managed state or a complete running-image baseline fails closed before migration.
  - legacy snapshot parsing occurs in a subshell so candidate image variables are not overwritten.
  - schema state is persisted unknown before destructive recovery and known only after complete restore.
  - optional marketplace-scheduler is stopped and verified stopped before destructive recovery.
  - compatibility tests invoke the implemented compatible-schema subcommand and are wired into Synology CI.
  - release-state values use shell-safe %q serialization and the world-name round trip is covered.
  - rollback validates independent schema identity against last-good application compatibility before old images are started and explicitly does not change schema.
  - recover-schema.sh validates managed evidence, digest and transition identity before recreating only the staging Platform DB.
  - deterministic focused tests include image-bound contract parsing, unexpected contract-key rejection, immutable legacy snapshot ordering, candidate-variable preservation, fail-closed missing baseline, world-name quoting and prior rollback/recovery cases.
derived:
  - the two P1 findings from Codex review at fbbf519471 are structurally addressed by image-bound candidate metadata and backup-capable legacy baseline bootstrap.
unknown:
  - terminal result of the next exact-head CI generation after checkpoint repair.
  - fresh independent review result on the final coherent head.
conflicts: []
first_failure:
  marker: checkpoint-unsupported-keys
  evidence: Agent Governance run 31578389351 rejected unsupported review_findings_repaired and self_review mappings in the checkpoint; implementation tests were not implicated.
rejected_hypotheses:
  - image rollback implicitly restores schema; it does not.
  - migration reversal is safe by default; no such contract exists.
  - workflow checkout metadata is sufficient to identify a historical selected image contract; it is not.
  - mutable running-image tags remain reliable after pull; immutable pre-pull RepoDigest capture is required.
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
  - command: Agent Governance run 31578389351
    result: FAIL
    evidence: checkpoint schema only; unsupported top-level nested mappings were rejected and removed in this repair.
  - command: current exact-head focused and repository CI
    result: NOT_RUN
    evidence: a new validation generation is required after this checkpoint-schema repair commit.
next_action: inspect the new exact-head focused Synology validation and full diff, repair verified defects only, then obtain fresh exact-head Codex review and complete the merge gate.
```
