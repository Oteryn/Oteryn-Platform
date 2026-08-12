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
blockers:
  - none; PR #1003 owns .github/workflows/deploy-synology-staging.yml and temporarily owns deploy/synology/scripts/production-target-preflight.sh to repair the exact release-identity consumer compatibility finding; this task does not edit either path.
cross_repository_tasks: []
```

Ownership coordination at 2026-08-12T18:26:15+02:00: PR #1013 has no diff for `deploy/synology/scripts/production-target-preflight.sh`; that path is explicitly released from this wildcard lease to PR #1003 so the exact-SHA/digest producer and its preflight consumer can be repaired atomically. All other `deploy/synology/**` ownership remains with this task.

The older `OTERYN-20260801-public-domain-repair` record's latest durable checkpoint has `branch: none`, `pr: none`, omits the current implementation paths from active ownership, and explicitly releases its former implementation ownership.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-12T18:26:15+02:00
head: 7bdf6a519f83f367cf442665c09d8729ddd3c405
branch: ops/synology-rollback-schema-safety-1007
pr: 1013
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
  - PR #1013 remains scoped to the dedicated task branch and intentionally excludes PR #1003-owned .github/workflows/deploy-synology-staging.yml.
  - candidate release identity is derived from matching Platform/Gateway OCI revision labels and compatibility metadata is read from the exact Platform image rather than workflow checkout state.
  - release metadata persists exact application SHA, immutable Platform/Gateway/Canary image digests, schema compatibility identity, accepted schema identities and rollback eligibility.
  - legacy first-managed deployment either establishes an immutable running-image baseline plus independent observed schema identity before backup or fails closed before migration for an existing non-empty database.
  - migration-bearing releases read the actual known schema identity from schema-state.env, quiesce Platform database consumers before backup, bind backup evidence to source/candidate releases plus Compose project and Platform database target, and mark schema unknown before migration starts.
  - recover-schema.sh parses an exact evidence allowlist, proves backup digest, release transition, accepted schema and destructive target identity before marking schema unknown or issuing DROP DATABASE, then records known schema only after complete import.
  - the manual Recover Synology Staging Schema workflow reconstructs a secret-bearing ephemeral environment from the protected synology-staging Environment and serializes with the deployment concurrency group; it is repository-delivered only and is not executed by this task.
  - Marketplace reconciliation preserves durable/effective settings and force-recreates both the browser-facing Platform service and scheduler on the selected Platform image before rollback health success or release-state promotion.
  - same-release redeploy requires candidate acceptance of the known schema, skips migration, preserves the existing distinct last-good release and reconciles Marketplace without creating a false migration boundary.
  - health helper aliases remain mapped to repository-pinned immutable Alpine and Python digests without changing probe retry or assertion behavior.
  - rollback validates independent schema identity against last-good application compatibility and explicitly states that image rollback does not restore or change database schema.
  - deterministic focused CI covers rollback compatibility, missing/stale metadata, immutable probes, recovery target binding, guarded recovery workflow, Marketplace runtime reconciliation and same-release last-good preservation.
  - PR #1003's proposed staging workflow resolves release_sha-tagged Platform/Gateway images and Canary input to immutable digests and verifies Platform/Gateway OCI revisions.
  - PR #1013 does not modify deploy/synology/scripts/production-target-preflight.sh; that consumer path is now explicitly excluded from this task and transferred to PR #1003 for the exact-SHA/digest compatibility repair identified by fresh review.
  - protected staging and production execution remain outside this task; no recovery/deploy workflow has been dispatched and no production evidence is claimed.
derived:
  - the four P1 findings from the Codex review of 4085fc5de3 are structurally addressed by the guarded recovery workflow, target-bound backup evidence, complete Marketplace runtime reconciliation and same-release rollback-target preservation.
  - repository contract tests plus exact-head standard CI can prove implementation behavior without performing the protected staging mutation that remains separately gated.
  - this ownership-only checkpoint creates a successor PR head after material implementation head 7bdf6a519f83f367cf442665c09d8729ddd3c405; live PR state is authoritative for that successor identity.
unknown:
  - terminal result of the exact-head CI generation after this checkpoint commit.
  - fresh independent Codex review result on the final coherent head.
conflicts: []
first_failure:
  marker: release-identity-error-not-propagated
  evidence: Build Synology run 31578389324 exposed a masked nested release-revision mismatch; explicit propagation was added and later focused Synology validation passed the corrected path.
rejected_hypotheses:
  - image rollback implicitly restores schema; it does not.
  - migration reversal is safe by default; no such contract exists.
  - workflow checkout metadata is sufficient to identify a historical selected image contract; it is not.
  - mutable running-image tags remain reliable after pull; immutable pre-pull RepoDigest capture is required.
  - application primary schema identity always equals the database schema at backup time; compatible image-only rollback disproves this.
  - recreating only the Marketplace scheduler restores Marketplace runtime; the Platform service must be reconciled with the same durable settings and image.
  - same-release redeploy is a new migration boundary; treating it as one would overwrite the distinct last-good target without a schema transition.
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - .github/workflows/synology-rollback-contract.yml
  - deploy/synology/release-contract.env
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/recover-schema.sh
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/rollback.sh
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - this task record
validation:
  - command: exact-head standard repository CI on predecessor 4085fc5de3cfd8f4a6a1dd4c458856e1ac97d951
    result: PASS
    evidence: CI, Agent Governance, Build Synology, Phase 7 Production-Like, Platform DB Outage, Edge Security and Game Auth Ticket Concurrency were green; fresh review then found four additional P1 edge cases requiring the current repair.
  - command: current repair package
    result: NOT_RUN
    evidence: exact-head validation starts after this checkpoint commit; no protected recovery workflow will be executed.
next_action: inspect aggregate exact-head CI and fresh exact-head Codex review; repair any material finding, otherwise complete full-diff self-review and squash merge.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: 20260812T182615+0200-ownership-coordination
  session_started_at: 2026-08-12T18:26:15+02:00
  checkpointed_at: 2026-08-12T18:26:15+02:00
  last_progress_at: 2026-08-12T18:26:15+02:00
  phase: exact-head validation and independent review
  exact_head: 7bdf6a519f83f367cf442665c09d8729ddd3c405
  pull_request: 1013
  active_operation: ownership coordination plus final exact-head GitHub Actions and Codex review
  external_run_ids: []
  operation_started_at: 2026-08-12T18:26:15+02:00
  wait_deadline_at: 2026-08-12T19:11:15+02:00
  check_generation: ownership-coordination-successor
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: exact-head required checks and fresh review are terminal after PR #1003 releases its two excluded paths
  next_action: After PR #1003 releases .github/workflows/deploy-synology-staging.yml and deploy/synology/scripts/production-target-preflight.sh, refresh against live main and continue exact-head validation without touching those paths beforehand.
```
