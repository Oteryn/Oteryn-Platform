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
- [x] Plan `docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md` with packets P1 to P7, each with owned paths, validation and rollback, and owner/operator inputs.
- [x] Owner answers Q1 to Q6 (2026-10-12) applied consistently to ADR 0046 and the plan: clean-slate bootstrap, rollback only before the first native evidence write, MariaDB volume deleted after a successful cutover smoke test with no dump, stack-CA TLS, NAS-only staging backups with provisional RPO 24 h / RTO 4 h and off-NAS required before production, accent-sensitive search.
- [x] No secret, certificate or credential in the repository; no new legacy-token occurrence; baseline not raised.
- [x] Exact-head CI and Agent Governance gates pass; PR #1561 merged as `dc3edffd` on 2026-10-12.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/architecture/adr/README.md
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
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
updated_at: 2026-10-12T19:00:00Z
head: dc3edffd527e2375bf66c195a529e5b80e3a01ba
branch: claude/platform-postgres-migration-plan-20261012
pr: 1561
status: completed
context_routes:
  - architecture
  - database
owned_paths:
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/architecture/adr/README.md
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-postgres-plan.md
proven:
  - open PR 1558 claims ADR prefix 0045, so this decision takes 0046
  - the Platform runtime DB user holds ALL PRIVILEGES on the schema and migrations run inside the runtime container
  - ADR 0044 allowlists exactly one store identity (driver mysql, host mariadb, port 3306) and needs a new decision to change it
  - the CI workflow budget is full (53 of 53), so packets extend existing workflows
  - the staging deploy already runs migrate, passport:keys, oauth-client:ensure, world:ensure, native-admission publish-trust, native-topology:issue and native-route:publish idempotently
  - owner answers Q1 to Q6 were given on 2026-10-12 and relayed by the lead
derived:
  - a clean-slate bootstrap is safe because the Game node is not deployed and all staging data is test data; the witness directory must be reset together with the database
unknown:
  - none
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - one PostgreSQL cluster shared with Game; PITR is cluster-wide and DUR-02 needs independent restore
  - copying MariaDB data to PostgreSQL as a required step; superseded by owner answer Q1 (clean slate), kept only as an optional fallback
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20261012-platform-postgres-plan.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/architecture/adr/README.md
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
validation:
  - command: python3 tools/validation/adr_registry.py && python3 tools/validation/test_adr_registry.py; python3 tools/validation/legacy_token_ratchet.py; python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint; python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: local run before push
blockers:
  - none
next_action: none for this task. Two review findings on the last head are fixed under OTERYN-20261012-platform-postgres-plan-fixes (PR #1562). P1 starts under its own task.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository docs PR
source_branch_evidence: PR #1561 merged as dc3edffd on 2026-10-12; git ls-remote shows the source branch absent after merge
```

## Notes

The review in PR #1560 recommended staying on MariaDB. The owner chose to migrate. ADR 0046 records the rationale honestly: the benefits are prospective, and the timing (all data is test data) is what decides it. The owner answered the plan's six questions on 2026-10-12; the plan's §6 records the answers.
