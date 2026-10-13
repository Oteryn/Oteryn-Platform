---
task_id: OTERYN-20261013-platform-pg-p6b-impl
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MAINTENANCE_GATE_DESIGN.md
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
search_first:
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - deploy/synology/scripts/bootstrap-postgres.sh
optional_reads: []
---

# OTERYN-20261013 Platform PostgreSQL P6b implementation and cutover tooling

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Implement the accepted P6b maintenance-gate design (PR #1588): the `maintenance-close` and `maintenance-open` modes of the Cloudflare endpoint workflow, the shared write concurrency group and workflow changes, and `cutover-postgres.sh` with a `cutover` entry in the Synology staging workflow. The destructive MariaDB volume deletion is an explicit entry gate that needs the owner's separate short confirmation. Also record the staging PITR drill evidence (run 38310896412) and archive the P6b design and #1580 task records. Nothing is executed on Synology or Cloudflare; the tooling is tested with fakes only.

## Acceptance criteria

- [x] Gate modes, origin-route precondition, apply refusal, audit fields and evidence artifacts are implemented and tested against the mock.
- [x] `cutover-postgres.sh` enforces step order, forward-only after step 4 and the destructive entry gate, covered by stub tests.
- [x] Cutover evidence template and PITR drill evidence entry exist; the two records are archived with terminal source-branch evidence.
- [ ] Exact-head CI passes, the full Codex review is addressed and no thread is unresolved.

## Ownership

```yaml
owned_paths:
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - .github/workflows/cloudflare-oteryn-endpoint-main-operation.yml
  - tests/operations/cloudflare-oteryn-endpoints/**
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - deploy/synology/scripts/cutover-postgres.sh
  - .github/workflows/deploy-synology-staging.yml
  - tests/ci/test_synology_cutover_postgres.py
  - docs/operations/PLATFORM_POSTGRES_CUTOVER_EVIDENCE.md
  - docs/operations/PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md
  - docs/agents/tasks/active/OTERYN-20261013-platform-pg-p6b-impl.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-pg-p6b.md
  - docs/agents/tasks/archive/OTERYN-20261012-verifier-runtime-connection-allowlist.md
modules:
  - operations
dependencies:
  - PR 1588 (accepted P6b design)
  - P6a tooling (separate session; its files are not touched here)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-13T00:00:00Z
head: UNKNOWN
branch: claude/platform-p6b-cutover-20261013
pr: 1590
status: validating
context_routes:
  - operations
owned_paths:
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - .github/workflows/cloudflare-oteryn-endpoint-main-operation.yml
  - tests/operations/cloudflare-oteryn-endpoints/**
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - deploy/synology/scripts/cutover-postgres.sh
  - .github/workflows/deploy-synology-staging.yml
  - tests/ci/test_synology_cutover_postgres.py
  - docs/operations/PLATFORM_POSTGRES_CUTOVER_EVIDENCE.md
  - docs/operations/PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md
  - docs/agents/tasks/active/OTERYN-20261013-platform-pg-p6b-impl.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-pg-p6b.md
  - docs/agents/tasks/archive/OTERYN-20261012-verifier-runtime-connection-allowlist.md
proven:
  - The owner accepted the P6b residual risks and Q1-4.
  - PITR drill run 38310896412 (head ba971b8) succeeded; its values come from the run API and job log.
derived: []
unknown:
  - Whether the live tunnel configuration API returns a version that increases by one per PUT (design assumption, verified at the first live audit, not here).
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - .github/workflows/cloudflare-oteryn-endpoint-main-operation.yml
  - tests/operations/cloudflare-oteryn-endpoints/**
  - deploy/synology/scripts/cutover-postgres.sh
  - .github/workflows/deploy-synology-staging.yml
  - tests/ci/test_synology_cutover_postgres.py
validation:
  - command: bash tests/operations/cloudflare-oteryn-endpoints/run.sh and check-*.py
    result: PASS
    evidence: mock Cloudflare and fake gh; no real Cloudflare call
  - command: python3 tests/ci/test_synology_cutover_postgres.py; python3 tests/ci/test_synology_postgres_contract.py
    result: PASS
    evidence: 56 cutover tests with stub docker/gh/curl; nothing run on Synology
  - command: python3 tools/validation/legacy_token_ratchet.py; python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: ratchet ok; documentation IA validated; PHP checks run in CI only
blockers:
  - none
next_action: full Codex review, one fixes commit, resolve threads, auto-merge squash
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending merge; verify the source ref disappears after merge
```

## Notes

Nothing in this packet runs on Synology or Cloudflare. The MariaDB deletion step is gated on a separate owner confirmation.
