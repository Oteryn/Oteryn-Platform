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
updated_at: 2026-08-12T07:45:00Z
head: 8eef9629110204404b18412fd7fc5c7eb71c8f85
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
proven:
  - branch was created from protected main ab43c4b47173e7208d34851c4091f79051379f7a and remains behind_by=0 at this checkpoint.
  - PR #1003 remains open and owns .github/workflows/deploy-synology-staging.yml; PR #1013 does not modify it.
  - release-state metadata validates exact 40-character application SHA, immutable runtime image identities, schema compatibility identity, accepted schema identities and rollback eligibility.
  - deployment persists candidate identity before migration and writes schema state unknown before invoking migrate; known schema identity is persisted only after migrate succeeds.
  - a pre-migration Platform DB dump is tied to old/candidate release SHAs, old schema identity and SHA-256 evidence.
  - rollback validates actual independent schema identity against last-good application compatibility before old images are started and explicitly does not change schema.
  - recover-schema.sh is explicit staging-only recovery; it verifies managed evidence/digest/transition identity before recreating only the staging Platform DB and never runs automatically.
  - health helper aliases are translated at the shared Docker boundary to repository-pinned alpine/python @sha256 identities while existing health assertions remain unchanged.
  - deterministic tests cover compatible rollback, incompatible schema rejection, missing metadata, stale identity, immutable health helper mapping, migration ambiguity ordering, explicit verified recovery and truthful image rollback.
  - PR #1013 exact-head CI generation for material head 8eef9629110204404b18412fd7fc5c7eb71c8f85 started with Agent Governance, CI, Build Synology Staging Images, Deep System Validation and other applicable workflows.
derived:
  - migration-bearing ambiguous outcomes fail closed because schema identity cannot remain inherited from the prior successful release.
  - successful verified schema recovery and image rollback remain separate operator actions, preventing image rollback from being represented as DB recovery.
unknown:
  - terminal result of exact-head CI after this checkpoint-only commit.
  - fresh independent review result.
conflicts: []
first_failure:
  marker: none-known-on-current-material-head
  evidence: validation generation is running; no terminal failure has yet been observed.
rejected_hypotheses:
  - image rollback implicitly restores schema; it does not.
  - migration reversal is safe by default; no such contract exists.
  - current-release alone is sufficient after a failed migration-bearing deployment; candidate and independent schema state are required for ambiguous outcomes.
changed_paths:
  - deploy/synology/release-contract.env
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/recover-schema.sh
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/rollback.sh
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - tests/ci/test_synology_rollback_contract.py
  - this task record
validation:
  - command: compare main...ops/synology-rollback-schema-safety-1007
    result: PASS
    evidence: 8 changed implementation/test/docs paths, behind_by=0 before PR validation generation.
  - command: exact-head PR workflows
    result: RUNNING
    evidence: PR #1013 workflow generation beginning at material head 8eef9629110204404b18412fd7fc5c7eb71c8f85.
next_action: inspect exact-head CI failures first, repair only verified defects, then perform full diff self-review and obtain fresh independent review before merge.
```
