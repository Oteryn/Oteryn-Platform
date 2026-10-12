---
task_id: OTERYN-20261012-platform-pg-p6b
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
search_first:
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - scripts/operations/cloudflare-oteryn-endpoints.sh
optional_reads: []
---

# OTERYN-20261012 Platform PostgreSQL P6b: maintenance-gate design

## Goal

Governing GitHub Issue: #1474. Complete the P6b maintenance-gate design note that `PLATFORM_POSTGRES_MIGRATION_PLAN.md` requires before P6: resolve the three design questions the merged draft (#1567) left open (closure identity, edge-generation binding, serialization), fix the `maintenance-close` and `maintenance-open` mode contracts, the origin-route precondition, failure and rollback behaviour, the exact files and tests of the later implementation, and the owner approvals. Documentation only. No workflow, script, configuration, secret or data change.

## Acceptance criteria

- [x] The note fixes mechanism, inputs and evidence for `maintenance-close` and `maintenance-open`, and how the origins are closed and reopened.
- [x] The note fixes how the "only two canonical ingress rules" precondition is checked.
- [x] The note fixes failure and rollback behaviour, the implementation files and tests, and the owner approval items.
- [x] No claim beyond the plan except items marked as decisions for owner approval; no secret; no new legacy-token occurrence.
- [ ] Exact-head CI passes, the complete Codex review is addressed and no thread is unresolved (PR #1588, pending).

## Ownership

```yaml
owned_paths:
  - docs/operations/PLATFORM_POSTGRES_MAINTENANCE_GATE_DESIGN.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p6b.md
modules:
  - operations-docs
dependencies:
  - PR 1567 (merged first draft of the note)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T22:00:00Z
head: UNKNOWN
branch: claude/platform-pg-p6b-20261012
pr: 1588
status: waiting
context_routes:
  - operations
owned_paths:
  - docs/operations/PLATFORM_POSTGRES_MAINTENANCE_GATE_DESIGN.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p6b.md
proven:
  - The existing script preserves unrelated rules and its apply rebuilds canonical rules with loopback origins, so apply would reopen a closed gate without a guard; the note restricts apply to the open state.
  - The dispatch concurrency group includes the run id, so today it serializes nothing; the pre-PUT hash check is the only guard.
  - The mock already returns a configuration version; the live API behaviour is an assumption the note says to verify.
derived:
  - Mutating entry points also include the marker pull request, owner comment and push marker workflows, so the apply guard belongs in the script.
unknown:
  - Whether the live tunnel configuration API returns a version that increases by one per PUT.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - A GitHub environment variable for closure identity: GITHUB_TOKEN cannot write it and the plan allows no new credential.
  - A plain concurrency group as a lock: it can replace a pending close.
changed_paths:
  - docs/operations/PLATFORM_POSTGRES_MAINTENANCE_GATE_DESIGN.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p6b.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p5.md (archive_pending transition, not owned)
  - docs/agents/tasks/active/OTERYN-20261012-pg-p6-design-notes.md (gate-note ownership removed, archive_pending transition, not owned)
validation:
  - command: python3 tools/validation/legacy_token_ratchet.py; python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint; python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: ratchet ok; 13 active checkpoints including this record validated; documentation IA validated; source-branch closeout validated
blockers:
  - none
next_action: wait for exact-head CI and the complete Codex review, fix all findings in one commit, then archive this record in the final authoring commit when the coordinator says so
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending merge; verify the source ref disappears after merge
```

## Notes

Design note only; the implementation packet starts after the owner accepts the note.
