---
task_id: OTERYN-20260812-synology-rollback-schema-safety
mode: implementation
branch: sync/synology-rollback-schema-safety-1007
status: completed
project_lane: oteryn-platform-core
---

# OTERYN-20260812 Synology rollback schema-safety

## Goal

Make Synology staging rollback truthful and schema-safe for Issue #1007 without production deployment or protected-environment mutation.

## Acceptance

- [x] Enforce expand/contract migration compatibility policy and fail closed rollback when compatibility cannot be proven.
- [x] Persist release SHA, immutable runtime image identities, schema compatibility identity, last-good identity and rollback eligibility.
- [x] Never represent image rollback as database schema rollback.
- [x] Provide bounded migration-bearing recovery backed by verified pre-migration staging database backup evidence and target identity validation.
- [x] Pin health probe helper images by immutable digest at the shared Docker invocation boundary without weakening probes.
- [x] Add deterministic positive/negative contract tests for compatible rollback, incompatible schema rejection, missing metadata, stale last-good identity, immutable probes, fresh-empty recovery, unresolved transition retries, rollback Gateway identity and Canary bind mismatch.
- [x] Provide a guarded post-failure recovery entry point using the protected `synology-staging` Environment without executing it in this repository-only task.
- [x] Obtain terminal green exact-head validation on final PR head `7f296293f284e1cc93061ada5725300b384f630f`.
- [x] Complete fresh independent self-review on the final exact head with zero material findings.
- [x] Squash merge delivery PR #1024.

## Terminal evidence

- Issue authority: #1007.
- Historical implementation/review predecessor: PR #1013.
- Delivery authority: PR #1024.
- Final validated PR head: `7f296293f284e1cc93061ada5725300b384f630f`.
- Squash merge commit on `main`: `db08e90871a546eaa330f3eecd0f1e1d5a0b5254`.
- Exact-head workflows on `7f296293f284e1cc93061ada5725300b384f630f`: all terminal PASS:
  - Agent Governance `31716889390`
  - Synology Rollback Contract `31716889376`
  - Build Synology Staging Images `31716889361`
  - Game Auth Ticket Concurrency `31716889282`
  - Edge Security Emulation `31716889283`
  - Platform DB Outage Validation `31716889284`
  - CI `31716889404`
  - Phase 7 Production-Like Validation `31716889290`
  - Deep System Validation `31716889292`
- Fresh independent self-review verified that the final checkpoint successor from `45d4bd0d205649c6130eec1343a43f82a88ef4b8` to `7f296293f284e1cc93061ada5725300b384f630f` changed only the task record; no deployment/recovery code changed after the previously validated material head.
- All historical review threads on PR #1013 and the successor review thread on PR #1024 were resolved before merge.
- PR #1003 was terminal before Synology path mutation; `.github/workflows/deploy-synology-staging.yml` remained outside this task's mutation scope.

## Delivered safety contract

- Expand/contract is the enforced default migration policy.
- Candidate release metadata persists exact release/application SHA, immutable Platform/Gateway/Canary image identities, schema compatibility identity, accepted schema identities, last-good identity and rollback eligibility.
- Candidate metadata fails closed unless its primary schema identity is included in its own accepted schema set.
- Existing databases without a managed/provable schema baseline fail closed; a truly fresh empty Platform DB gets a verified pre-migration baseline before first application startup/migration.
- Migration-bearing releases quiesce Platform database consumers, persist actual schema identity, bind recovery evidence to source/candidate and database/Compose target, and mark schema state unknown before migration/destructive recovery.
- `recover-schema.sh` verifies backup digest and target identity, uses an allowlisted evidence contract, and marks schema known only after bounded successful recovery.
- Surviving `candidate-release.env` prevents retry from overwriting candidate identity or recovery evidence.
- Same-release redeploy preserves the previous distinct last-good target and proves schema acceptance before skipping migration.
- Rollback derives Gateway version from persisted last-good release SHA and rejects Canary/world-host identity drift before runtime start.
- Image rollback never claims to restore database schema.
- Health-probe helper images are immutable by digest/equivalent repository-pinned identity.
- Deterministic CI regression coverage protects compatible rollback, incompatible rollback rejection, missing metadata, stale identities, recovery evidence, retry safety and immutable probes.

## Boundaries

No production deployment, protected-environment approval, secret mutation, production-data copy, live schema recovery or live data mutation was performed. No owner-funded Codex/API invocation was used for this closeout.

## Closeout

Repository hardening is complete. The separate production go-live/production proof boundary remains outside Issue #1007.