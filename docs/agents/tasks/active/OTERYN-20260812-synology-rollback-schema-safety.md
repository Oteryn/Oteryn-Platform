---
task_id: OTERYN-20260812-synology-rollback-schema-safety
mode: implementation
branch: ops/synology-rollback-schema-safety-1007
status: implementing
project_lane: oteryn-platform-core
---

# OTERYN-20260812 Synology rollback schema-safety

## Goal

Make Synology staging rollback truthful and schema-safe for Issue #1007 without production deployment or protected-environment mutation.

## Acceptance

- [ ] Enforce expand/contract migration compatibility policy and fail closed rollback when compatibility cannot be proven.
- [ ] Persist release SHA, immutable runtime image identities, schema compatibility identity, last-good identity and rollback eligibility.
- [ ] Never represent image rollback as database schema rollback.
- [ ] Provide bounded migration-bearing recovery backed by a pre-migration staging database backup and identity validation.
- [ ] Pin health probe helper images by immutable digest.
- [ ] Add deterministic positive/negative contract tests and exact-head CI evidence.
- [ ] Complete full diff self-review and fresh independent review.
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
  - PR #1003 owns .github/workflows/deploy-synology-staging.yml; this task will not edit that path while ownership remains active.
cross_repository_tasks: []
```

The older `OTERYN-20260801-public-domain-repair` record has a stale historical ownership declaration for `deploy/synology/scripts/health-check.sh`, but its latest durable checkpoint has `branch: none`, `pr: none`, omits this path from `owned_paths`, and explicitly says the former implementation no longer represents active ownership. Under `SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md`, the checkpoint releases that implementation ownership.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-12T07:15:00Z
head: ab43c4b47173e7208d34851c4091f79051379f7a
branch: ops/synology-rollback-schema-safety-1007
pr: none
status: implementing
context_routes:
  - ci-repair
  - testing
owned_paths:
  - deploy/synology/**
  - tests/ci/test_synology_rollback_contract.py
  - docs/operations/SYNOLOGY_ROLLBACK_SCHEMA_SAFETY.md
  - this task record
proven:
  - main was ab43c4b47173e7208d34851c4091f79051379f7a immediately before branch creation.
  - PR #1003 owns .github/workflows/deploy-synology-staging.yml and does not own deploy/synology/**.
  - all other open PRs inspected before mutation do not overlap deploy/synology/**.
  - deploy.sh snapshots runtime image references before running php artisan migrate --force.
  - rollback.sh restores runtime images but does not restore database schema.
  - health-check.sh uses mutable alpine:3.22 and python:3.12-alpine helper tags.
derived:
  - rollback must be compatibility-gated before old application images are started.
unknown:
  - terminal exact-head CI result after implementation.
  - fresh independent review result.
conflicts: []
first_failure:
  marker: schema-rollback-proof-missing
  evidence: current last-good.env contains image references only and cannot prove old application compatibility with post-migration schema.
rejected_hypotheses:
  - image rollback implicitly restores schema; it does not.
  - migration reversal is safe by default; no such contract exists.
changed_paths:
  - this task record
validation: []
next_action: implement release-state, compatibility gate, bounded recovery and immutable-probe contracts on non-overlapping paths.
```
