---
task_id: OTERYN-20261012-pg-p6-design-notes
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
search_first:
  - docs/agents/LEGACY_TOKEN_BASELINE.json
optional_reads:
  - deploy/synology/scripts/generate-native-admission-key.sh
---

# OTERYN-20261012 Platform PostgreSQL P6a and P6b design notes

## Goal

Governing GitHub Issue: #1474 (native-only migration) is the canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Write the two design notes that `PLATFORM_POSTGRES_MIGRATION_PLAN.md` requires before P6 cutover: the P6a trust-safeguards note and the P6b maintenance-gate state-machine note. Documentation only. No code, configuration, schema, deployment, secret or data change.

## Acceptance criteria

- [x] `docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md` states the provenance-only init, the attempt evidence, the invariants and the tests, with no automated missing-profile repair.
- [x] `docs/operations/PLATFORM_POSTGRES_MAINTENANCE_GATE_DESIGN.md` states the marker, lock, ordering, crash recovery and tests per transition; the gate fails closed and only `maintenance-open` reopens the origins.
- [x] No claim beyond the plan except items explicitly marked as proposals.
- [x] No secret in the repository; no new legacy-token occurrence; baseline not raised.
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md
  - docs/operations/PLATFORM_POSTGRES_MAINTENANCE_GATE_DESIGN.md
  - docs/agents/tasks/archive/OTERYN-20261012-pg-p6-design-notes.md
  - docs/agents/tasks/archive/OTERYN-20261012-pg-p6-design-notes.md
modules:
  - operations-docs
dependencies:
  - PR 1562 (merged plan fixes)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T21:00:00Z
head: UNKNOWN
branch: claude/pg-p6-design-notes-20261012
pr: 1567
status: completed
context_routes:
  - operations
owned_paths:
  - docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md
  - docs/operations/PLATFORM_POSTGRES_MAINTENANCE_GATE_DESIGN.md
  - docs/agents/tasks/archive/OTERYN-20261012-pg-p6-design-notes.md
proven:
  - the plan requires both notes accepted before P6 starts and fixes only the two gate claims
  - generate-native-admission-key.sh keeps used.kids in a host key-custody directory outside the database
derived:
  - one PUT changes both canonical rules, so split state can only come from an external edit
unknown:
  - whether the key-custody directory is outside every restore scope on the Synology host
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md
  - docs/operations/PLATFORM_POSTGRES_MAINTENANCE_GATE_DESIGN.md
  - docs/agents/tasks/archive/OTERYN-20261012-pg-p6-design-notes.md
validation:
  - command: python3 tools/validation/legacy_token_ratchet.py; python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint; python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: local run before push
blockers:
  - none
next_action: none; PR #1567 merged as 8aee2a0e7c76d3c2dc07d6a5306c36ed01927443
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: PR #1567 merged as 8aee2a0e7c76d3c2dc07d6a5306c36ed01927443 from claude/pg-p6-design-notes-20261012; git ls-remote --heads origin claude/pg-p6-design-notes-20261012 returns nothing (verified)
```
