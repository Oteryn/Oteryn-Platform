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
  - none; PR #1003 still owns .github/workflows/deploy-synology-staging.yml, this task does not edit it, and its proposed immutable release-SHA/digest contract was inspected and is compatible with this implementation.
cross_repository_tasks: []
```

The older `OTERYN-20260801-public-domain-repair` record's latest durable checkpoint has `branch: none`, `pr: none`, omits `health-check.sh` from current `owned_paths`, and explicitly releases its former implementation ownership.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-12T08:34:00Z
head: f568ccac6a316aea97eea1207c24b8e0fa852ceb
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
  - none; PR #1003 owns only the excluded staging workflow path and its proposed release_sha plus immutable digest handoff is compatible with the image-bound contract implemented here.
proven:
  - PR #1013 remains scoped to the dedicated task branch and the compare against main contains only the declared Synology workflow, deployment, test, operator-doc and task-record paths.
  - candidate release identity is derived from matching Platform/Gateway OCI revision labels and rejects disagreement with explicit release identity or GATEWAY_VERSION.
  - failed nested release-identity validation now propagates explicitly through command substitutions with `|| return 1`; the prior focused mismatch test exposed this real fail-open defect.
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
  - PR #1003 deploy workflow patch was inspected: it resolves release_sha-tagged Platform/Gateway images and Canary input to immutable digests and verifies Platform/Gateway OCI revisions match the exact release SHA before writing the staging environment.
  - protected staging and production deployment are deliberately outside task authority; executable validation is deterministic contract coverage plus production-like/deep repository validation and is not claimed as production rollback proof.
derived:
  - the two P1 findings from Codex review at fbbf519471 are structurally addressed by image-bound candidate metadata and backup-capable legacy baseline bootstrap.
  - the proposed #1003 workflow and this implementation compose without path mutation or hidden release-identity assumptions because both use the same verified OCI revision/digest identity.
unknown:
  - terminal result of the exact-head CI generation after the release-identity propagation repair and final documentation/checkpoint commit.
  - fresh independent Codex review result on the final coherent head.
conflicts: []
first_failure:
  marker: release-identity-error-not-propagated
  evidence: Build Synology run 31578389324 reached the mismatch regression and showed _oteryn_release_sha returned success after _oteryn_release_sha_for_images rejected differing OCI revisions; explicit propagation was added in lib.sh.
rejected_hypotheses:
  - image rollback implicitly restores schema; it does not.
  - migration reversal is safe by default; no such contract exists.
  - workflow checkout metadata is sufficient to identify a historical selected image contract; it is not.
  - mutable running-image tags remain reliable after pull; immutable pre-pull RepoDigest capture is required.
  - the OCI mismatch failure was only a test-harness defect; direct shell reproduction proved the nested command-substitution failure could be masked by the caller.
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
    evidence: 14/14 tests, shell syntax, Compose/LAN validation and image builds passed before the two subsequent review findings.
  - command: Agent Governance run 31578389351
    result: FAIL
    evidence: checkpoint schema only; unsupported nested keys were removed in commit 93e653787957bfe2e9754d961fa9c4ea5bd35ca3.
  - command: Build Synology run 31578389324 focused contract test
    result: FAIL
    evidence: release revision mismatch negative test exposed masked nested-function failure; fixed by explicit propagation in commit 5a38ddbef29a5a77e52b1d4d12706914d12877cb.
  - command: final candidate exact-head focused and repository CI
    result: NOT_RUN
    evidence: final generation starts after this documentation/checkpoint commit.
next_action: inspect final-candidate exact-head focused Synology validation and full diff, repair only verified defects, then obtain fresh exact-head Codex review and complete the merge gate.
```
