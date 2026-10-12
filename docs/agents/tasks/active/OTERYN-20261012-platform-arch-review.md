---
task_id: OTERYN-20261012-platform-arch-review
governing_issue: 1474
required_reads:
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/operations/INCIDENT_RECOVERY_RUNBOOK.md
search_first:
  - docs/agents/LEGACY_TOKEN_BASELINE.json
optional_reads:
  - deploy/synology/compose.yml
  - deploy/synology/scripts/lib.sh
---

# OTERYN-20261012 Platform architecture and database review

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Write an independent, documentation-only review of the Platform's module boundaries, its security and authority boundaries to the Game node and the Gateway, and its database choices. The database part covers MariaDB versus PostgreSQL, one engine or two, cluster separation, the cross-database Bazaar saga, migrations, backup and restore, Redis, SQLite in tests and single-NAS risk. Findings are ranked P0–P3 with evidence paths. The review ends with an engine recommendation against the Game ADR-0013 §2 criteria and a list of owner decisions. No code, configuration, schema, deployment or secret changes.

## Acceptance criteria

- [x] Review document at `docs/architecture/reviews/OTERYN_PLATFORM_ARCHITECTURE_AND_DATABASE_REVIEW_2026-10-12.md`, with ranked findings and an evidence path for each.
- [x] ADR-0013 §2 criteria for a Platform PostgreSQL migration listed, each with its status.
- [x] Owner decisions listed with options and a recommendation.
- [x] No new legacy-token occurrence; the ratchet baseline is not raised.
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - docs/architecture/reviews/OTERYN_PLATFORM_ARCHITECTURE_AND_DATABASE_REVIEW_2026-10-12.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-arch-review.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-arch-review.md
modules:
  - architecture-docs
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T09:00:00Z
head: UNKNOWN
branch: claude/platform-arch-review-20261012
pr: 1560
status: validating
context_routes:
  - architecture-docs
owned_paths:
  - docs/architecture/reviews/OTERYN_PLATFORM_ARCHITECTURE_AND_DATABASE_REVIEW_2026-10-12.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-arch-review.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-arch-review.md
proven:
  - deploy/synology runs MariaDB 11.8 and Redis 7.4 next to a single-worker built-in PHP server; Redis has no Platform consumer after #1556
  - the only backups are pre-migration dumps on the same NAS; the runbook says no operational backup or restore is proven
  - the only scheduler container exists only when MARKETPLACE_ENABLED=true
  - the default PHPUnit suite runs on in-memory SQLite
derived:
  - no ADR-0013 section 2 criterion for a Platform PostgreSQL migration is met on the reviewed main
unknown:
  - NAS-side state not in the repository (any untracked mTLS terminator, backup jobs, edge rules)
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - a PostgreSQL migration would close the backup gap; it would not, because neither engine has scheduled or PITR backups yet
changed_paths:
  - docs/architecture/reviews/OTERYN_PLATFORM_ARCHITECTURE_AND_DATABASE_REVIEW_2026-10-12.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-arch-review.md
validation:
  - command: python tools/validation/test_legacy_token_ratchet.py && python tools/validation/legacy_token_ratchet.py; python tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint
    result: PASS
    evidence: local run
blockers:
  - none
next_action: Lead review; on merge, archive this record and route owner decisions D-1 to D-7.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository docs PR
source_branch_evidence: pending
```

## Notes

Read-only review. Its recommendation is to keep MariaDB and fix backups, the scheduler and the mTLS deployable first. Owner decisions D-1 to D-7 are in the review's §7.
