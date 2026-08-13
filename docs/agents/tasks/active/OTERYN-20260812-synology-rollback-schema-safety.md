---
task_id: OTERYN-20260812-synology-rollback-schema-safety
mode: implementation
branch: sync/synology-rollback-schema-safety-1007
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
- [x] Add deterministic positive/negative contract tests, including fresh-empty recovery, unresolved transition retries, rollback Gateway identity and Canary bind mismatch.
- [x] Provide a guarded post-failure recovery entry point that reconstructs the ephemeral staging environment from the protected `synology-staging` Environment without executing it in this repository-only task.
- [ ] Obtain terminal green exact-head CI and complete fresh independent review.
- [ ] Squash merge, archive this task and close Issue #1007 only after terminal green exact-head validation.

## Ownership

```yaml
owned_paths:
  - deploy/synology/**
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - .github/workflows/synology-rollback-contract.yml
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - docs/agents/tasks/active/OTERYN-20260812-synology-rollback-schema-safety.md
  - docs/agents/tasks/archive/OTERYN-20260812-synology-rollback-schema-safety.md
excluded_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/production-target-preflight.sh
modules:
  - synology-staging-deployment
blockers: []
cross_repository_tasks: []
```

PR #1003 is terminal and its changes are present in `main@f59227086b9d2a58a37648cd6031e9f653c51b17`. PR #1013 is the historical implementation/review predecessor. The GitHub connector refused a non-force ref update after synchronization, so the exact same reviewed implementation was carried forward through GitHub's conflict-free merge tree onto successor branch `sync/synology-rollback-schema-safety-1007`; temporary synchronization artifacts were removed before successor PR #1024 was opened. No force update was used.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T16:13:00+02:00
head: 9727206dad6d61ff84d2ddc73edaafc5a29df2c9
branch: sync/synology-rollback-schema-safety-1007
pr: 1024
status: validating
context_routes:
  - ci-repair
  - testing
owned_paths:
  - deploy/synology/** except deploy/synology/scripts/production-target-preflight.sh
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - .github/workflows/synology-rollback-contract.yml
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - this task record
blockers: []
proven:
  - candidate release identity is derived from matching Platform/Gateway OCI revision labels and compatibility metadata is read from the exact Platform image.
  - release metadata persists exact application SHA, immutable Platform/Gateway/Canary image digests, schema compatibility identity, accepted schema identities and rollback eligibility.
  - existing databases without a managed/provable baseline fail closed; fresh empty databases get a verified empty backup before first migration.
  - migration-bearing releases quiesce Platform database consumers, record the actual known schema identity, bind evidence to source/candidate plus Compose/database target, and mark schema unknown before migration.
  - recover-schema.sh parses an exact allowlist, verifies backup digest and target identity, never runs implicitly, and marks schema known only after successful bounded restore.
  - surviving candidate-release.env blocks a retry before the migration hook can overwrite candidate identity or reuse the original recovery backup path.
  - Marketplace reconciliation recreates both Platform and scheduler with durable/effective state on the selected Platform image.
  - same-release redeploy preserves the previous distinct last-good target and skips migration after proving schema acceptance.
  - rollback derives GATEWAY_VERSION from persisted last-good RELEASE_SHA and rejects Canary bind/server-IP mismatch with persisted GAME_WORLD_HOST before runtime start.
  - health helper aliases are mapped to immutable Alpine/Python digests without weakening probes.
  - all historical review threads on predecessor PR #1013 are resolved.
  - focused Synology Rollback Contract passed on predecessor exact head 516b4a0e1e200f3f94b4432a8ad3c1e2d9af5459 after the final regression additions.
  - successor branch contains current main f59227086b9d2a58a37648cd6031e9f653c51b17 through a conflict-free merge tree and excludes temporary synchronization artifacts.
  - no recovery/deploy workflow was dispatched and no production evidence is claimed.
derived:
  - the repository implementation now covers migration failure before/after application startup, first-deploy partial migration, ambiguous schema state, retry evidence preservation, compatible/incompatible rollback, and rollback runtime identity drift.
unknown:
  - terminal result of exact-head CI on successor PR #1024 after this checkpoint commit.
  - fresh independent final review result on successor exact head.
conflicts: []
first_failure:
  marker: stale-base-ci-contract
  evidence: predecessor exact-head CI could not find tests/ci/test_acceptance_lockfile_contract.py because the branch was nine commits behind current main; successor was rebuilt from the conflict-free current-main merge tree instead of force-updating the old ref.
rejected_hypotheses:
  - image rollback restores schema; it does not.
  - generic reverse migrations are safe; no such contract exists.
  - workflow checkout metadata identifies historical selected image compatibility; it does not.
  - mutable tags are valid rollback identity; immutable digests and OCI revisions are required.
  - retrying an unresolved candidate is safe; it can destroy recovery evidence and is now rejected.
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - .github/workflows/synology-rollback-contract.yml
  - deploy/synology/release-contract.env
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/prepare-fresh-schema-baseline.sh
  - deploy/synology/scripts/recover-schema.sh
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - this task record
validation:
  - command: Synology Rollback Contract on predecessor exact head 516b4a0e1e200f3f94b4432a8ad3c1e2d9af5459
    result: PASS
    evidence: run 31708424268
  - command: successor exact-head repository validation
    result: NOT_RUN
    evidence: starts after this checkpoint commit; no protected recovery workflow will be executed.
next_action: inspect successor exact-head CI, perform fresh independent full-diff review, repair any material finding, then archive and squash merge only after terminal green validation.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 3
  session_id: 20260813T161300+0200-successor-sync
  session_started_at: 2026-08-13T15:59:00+02:00
  checkpointed_at: 2026-08-13T16:13:00+02:00
  last_progress_at: 2026-08-13T16:13:00+02:00
  phase: exact-head validation and independent review
  exact_head: 9727206dad6d61ff84d2ddc73edaafc5a29df2c9
  pull_request: 1024
  active_operation: successor exact-head GitHub Actions and fresh independent review
  external_run_ids: []
  operation_started_at: 2026-08-13T16:13:00+02:00
  wait_deadline_at: 2026-08-13T17:13:00+02:00
  check_generation: successor-main-sync
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: exact-head applicable checks are terminal and full-diff review has no material findings
  next_action: inspect exact-head workflow generation and continue repository-only validation; do not dispatch staging recovery or production operations.
```
