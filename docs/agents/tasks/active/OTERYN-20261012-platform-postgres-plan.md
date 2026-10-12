---
task_id: OTERYN-20261012-platform-postgres-plan
governing_issue: 1474
required_reads:
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0044-preproduction-store-native-route-publication.md
search_first:
  - docs/agents/LEGACY_TOKEN_BASELINE.json
optional_reads:
  - deploy/synology/compose.yml
  - config/database.php
---

# OTERYN-20261012 Platform PostgreSQL migration decision and plan

## Goal

Governing GitHub Issue: #1474 (native-only migration) is the canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Record the owner decision of 2026-10-12 to move Platform persistence from MariaDB 11.8 to PostgreSQL. Deliverables:

- ADR 0046. It references Game ADR-0013 §2 and keeps the Game ADR-0004 invariants. It places Platform and Game in separate PostgreSQL instances.
- An ordered migration plan, packets P1 to P7, giving owned paths, validation and rollback for each packet.

This is a documentation-only change. It does not change code, configuration, schema, deployment, secrets or data.

## Acceptance criteria

- [x] ADR 0046 `docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md`, Accepted, with the owner rationale, ADR-0013 §2 reference, ADR-0004 invariants and separate-instance placement; registered in the ADR README inventory.
- [x] Plan `docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md` with packets P1 to P7, each with owned paths, validation and rollback, owner/operator inputs and open owner questions.
- [x] No secret, certificate or credential in the repository; no new legacy-token occurrence; baseline not raised.
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/architecture/adr/README.md
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-postgres-plan.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-postgres-plan.md
modules:
  - architecture-docs
  - operations-docs
dependencies:
  - PR 1560 (architecture and database review, input only)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T12:00:00Z
head: UNKNOWN
branch: claude/platform-postgres-migration-plan-20261012
pr: 1561
status: validating
context_routes:
  - architecture
  - database
owned_paths:
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/architecture/adr/README.md
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-postgres-plan.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-postgres-plan.md
proven:
  - open PR 1558 claims ADR prefix 0045, so this decision takes 0046
  - the Platform runtime DB user holds ALL PRIVILEGES on the schema and migrations run inside the runtime container
  - ADR 0044 allowlists exactly one store identity (driver mysql, host mariadb, port 3306) and needs a new decision to change it
  - the CI workflow budget is full (53 of 53), so packets extend existing workflows
derived:
  - copying data is preferred over re-seeding because Game references AccountIds and operation ids, and the native evidence witness fails closed below its floor
unknown:
  - owner answers Q1 to Q6 in the plan
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - one PostgreSQL cluster shared with Game; PITR is cluster-wide and DUR-02 needs independent restore
changed_paths:
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/architecture/adr/README.md
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-postgres-plan.md
validation:
  - command: python3 tools/validation/adr_registry.py && python3 tools/validation/test_adr_registry.py; python3 tools/validation/legacy_token_ratchet.py; python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint; python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: local run before push
blockers:
  - none
next_action: Lead review; on merge, archive this record and route plan questions Q1 to Q6 to the owner before P1 starts.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository docs PR
source_branch_evidence: pending
```

## Notes

The review in PR #1560 recommended staying on MariaDB. The owner chose to migrate. ADR 0046 records the rationale honestly: the benefits are prospective, and the timing (all data is test data) is what decides it.
