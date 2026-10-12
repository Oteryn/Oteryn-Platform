---
task_id: OTERYN-20261012-platform-postgres-plan-fixes
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
search_first:
  - docs/agents/LEGACY_TOKEN_BASELINE.json
optional_reads:
  - deploy/synology/compose.yml
---

# OTERYN-20261012 Platform PostgreSQL plan review fixes

## Goal

Governing GitHub Issue: #1474 (native-only migration) is the canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Fix two review findings on the merged PostgreSQL migration plan (PR #1561, head `feb62ed`) and the matching ADR 0046 rule:

- After the first native evidence write on PostgreSQL, the PostgreSQL witness directory is never emptied, deleted or lowered, and the database is never emptied under it. Recovery keeps the witness and reconciles forward only. A clean-slate re-run of P3 with a witness reset is allowed only before that write. Retiring the MariaDB-era witness with the MariaDB volume after a successful cutover is kept distinct from the live PostgreSQL witness.
- The P6 smoke tests run against a running application behind a bounded external-access gate (LAN-only bind addresses for the Platform and Gateway), not under `php artisan down`, with no committed bypass secret and a defined rollback.

This is a documentation-only change. It does not change code, configuration, schema, deployment, secrets or data.

## Acceptance criteria

- [x] Plan P3, P5 and P6 and plan §6 state the witness rule and the access gate consistently.
- [x] ADR 0046 §8 and Reversal match the plan.
- [x] No secret in the repository; no new legacy-token occurrence; baseline not raised.
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-postgres-plan-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-postgres-plan-fixes.md
modules:
  - architecture-docs
  - operations-docs
dependencies:
  - PR 1561 (merged plan and ADR 0046)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T18:00:00Z
head: UNKNOWN
branch: claude/platform-postgres-plan-fixes-20261012
pr: 1562
status: validating
context_routes:
  - architecture
  - database
owned_paths:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-postgres-plan-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-postgres-plan-fixes.md
proven:
  - PR 1561 merged before two review findings on its last head were fixed
  - the Platform and Gateway publish host ports through PLATFORM_BIND_ADDRESS and GATEWAY_BIND_ADDRESS in deploy/synology/compose.yml
  - the native evidence witness fails closed when a database binding exists and its directory is empty, or when history is below the retained floor
derived:
  - a network-level access gate keeps the application testable during cutover without a maintenance bypass secret
unknown:
  - none
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Laravel maintenance bypass for smoke tests; it is cookie-based and does not serve machine callers such as health checks and the Gateway
changed_paths:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-postgres-plan-fixes.md
validation:
  - command: python3 tools/validation/adr_registry.py && python3 tools/validation/test_adr_registry.py; python3 tools/validation/legacy_token_ratchet.py; python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint; python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: local run before push
blockers:
  - none
next_action: Lead review; on merge, archive this record.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository docs PR
source_branch_evidence: pending
```
