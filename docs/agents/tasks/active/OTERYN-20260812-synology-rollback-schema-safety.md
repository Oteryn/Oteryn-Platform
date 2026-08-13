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
- [x] Obtain terminal green exact-head CI for material head `45d4bd0d205649c6130eec1343a43f82a88ef4b8`; checkpoint-only recovery successor must revalidate before merge.
- [ ] Complete fresh independent self-review on the final exact head.
- [ ] Squash merge, archive this task and close Issue #1007 only after terminal green final-head validation.

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

PR #1003 is terminal. PR #1013 is the historical implementation/review predecessor; successor PR #1024 is the delivery authority. The current successor material head integrates current `main@0e2351c0b590c24b81d64ed9ec7b2bdea0da09c8` without force update. No production/staging recovery workflow has been dispatched.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T17:42:00+02:00
head: 45d4bd0d205649c6130eec1343a43f82a88ef4b8
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
  - candidate validation fails closed unless the candidate primary schema identity is contained in its accepted-schema identities; the Codex review thread covering this case is resolved on the current implementation.
  - existing databases without a managed/provable baseline fail closed; fresh empty databases get a verified empty backup before first migration.
  - migration-bearing releases quiesce Platform database consumers, record the actual known schema identity, bind evidence to source/candidate plus Compose/database target, and mark schema unknown before migration.
  - recover-schema.sh parses an exact allowlist, verifies backup digest and target identity, never runs implicitly, and marks schema known only after successful bounded restore.
  - surviving candidate-release.env blocks a retry before the migration hook can overwrite candidate identity or reuse the original recovery backup path.
  - Marketplace reconciliation recreates both Platform and scheduler with durable/effective state on the selected Platform image.
  - same-release redeploy preserves the previous distinct last-good target and skips migration after proving schema acceptance.
  - rollback derives GATEWAY_VERSION from persisted last-good RELEASE_SHA and rejects Canary bind/server-IP mismatch with persisted GAME_WORLD_HOST before runtime start.
  - health helper aliases are mapped to immutable Alpine/Python digests without weakening probes.
  - all historical review threads on predecessor PR #1013 and the successor review thread on PR #1024 are resolved.
  - successor material head 45d4bd0d205649c6130eec1343a43f82a88ef4b8 has terminal green Agent Governance, Game Auth Ticket Concurrency, Edge Security Emulation, Platform DB Outage Validation, Synology Rollback Contract, Build Synology Staging Images, Phase 7 Production-Like Validation, CI and Deep System Validation.
  - Deep System Validation run 31714300230 completed the zero-retry browser matrix and all evidence/upload cleanup steps successfully.
  - no recovery/deploy workflow was dispatched and no production evidence is claimed.
derived:
  - the repository implementation covers migration failure before/after application startup, first-deploy partial migration, ambiguous schema state, retry evidence preservation, compatible/incompatible rollback, rollback runtime identity drift and candidate self-schema incompatibility.
unknown:
  - terminal result of exact-head CI on the checkpoint-only recovery successor created by this update.
  - fresh independent final self-review result on that exact final head.
conflicts: []
first_failure:
  marker: stale-base-ci-contract
  evidence: predecessor exact-head CI could not find tests/ci/test_acceptance_lockfile_contract.py because the branch was behind current main; successor was rebuilt from a conflict-free current-main merge tree instead of force-updating the old ref.
rejected_hypotheses:
  - image rollback restores schema; it does not.
  - generic reverse migrations are safe; no such contract exists.
  - workflow checkout metadata identifies historical selected image compatibility; it does not.
  - mutable tags are valid rollback identity; immutable digests and OCI revisions are required.
  - retrying an unresolved candidate is safe; it can destroy recovery evidence and is rejected.
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
  - command: Synology Rollback Contract on material head 45d4bd0d205649c6130eec1343a43f82a88ef4b8
    result: PASS
    evidence: run 31714300257
  - command: Build Synology Staging Images on material head 45d4bd0d205649c6130eec1343a43f82a88ef4b8
    result: PASS
    evidence: run 31714300403
  - command: CI on material head 45d4bd0d205649c6130eec1343a43f82a88ef4b8
    result: PASS
    evidence: run 31714300300
  - command: Agent Governance on material head 45d4bd0d205649c6130eec1343a43f82a88ef4b8
    result: PASS
    evidence: run 31714300242
  - command: Deep System Validation on material head 45d4bd0d205649c6130eec1343a43f82a88ef4b8
    result: PASS
    evidence: run 31714300230
  - command: checkpoint-only recovery successor exact-head validation
    result: NOT_RUN
    evidence: starts after this checkpoint update; no protected recovery or deployment workflow will be executed.
next_action: inspect checkpoint-only successor exact-head CI, perform fresh full-diff self-review, and squash merge only if the final head is terminal green with zero material findings.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 4
  session_id: 20260813T174200+0200-terminal-closeout-recovery
  session_started_at: 2026-08-13T17:42:00+02:00
  checkpointed_at: 2026-08-13T17:42:00+02:00
  last_progress_at: 2026-08-13T17:42:00+02:00
  phase: terminal exact-head revalidation and closeout
  exact_head: 45d4bd0d205649c6130eec1343a43f82a88ef4b8
  pull_request: 1024
  active_operation: checkpoint-only exact-head CI and fresh self-review
  external_run_ids:
    - 31714300242
    - 31714300295
    - 31714300327
    - 31714300682
    - 31714300257
    - 31714300403
    - 31714300330
    - 31714300300
    - 31714300230
  operation_started_at: 2026-08-13T17:42:00+02:00
  wait_deadline_at: 2026-08-13T18:27:00+02:00
  check_generation: terminal-closeout-recovery
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: checkpoint-only successor exact-head applicable checks are terminal and fresh full-diff self-review has zero material findings
  next_action: inspect successor exact-head checks once terminal; do not dispatch staging recovery or production operations.
```

## Safety

Repository-only hardening and recovery logic. No production deployment, protected-environment approval, credential/secret mutation, live data mutation, or owner-funded AI invocation is authorized or performed.
