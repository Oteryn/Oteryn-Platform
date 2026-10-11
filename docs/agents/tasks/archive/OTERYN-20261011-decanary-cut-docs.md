---
task_id: OTERYN-20261011-decanary-cut-docs
governing_issue: 1478
required_reads:
  - docs/agents/programs/DECANARY_REMOVAL_PLAN.md
search_first:
  - tools/validation/legacy_token_ratchet.py
optional_reads: []
---

# OTERYN-20261011-decanary-cut-docs

## Goal

Packets F-1..F-4 of the removal plan in one PR: delete legacy-only contracts and ADRs 0002 and 0021, rewrite the remaining live docs in neutral terms, drop the integration route from context routing, and record the historical-record exclusion list (decision Q-H) in the legacy-token ratchet.

## Acceptance criteria

- [x] Legacy-only contracts and ADRs 0002 and 0021 deleted; ADR README and `adr_registry.py` updated.
- [x] Live docs rewritten by a deterministic script; routing route removed.
- [x] Ratchet exclusions (contents and path names) implemented with tests; docs baseline lowered.
- [x] No archived record changed.

## Ownership

```yaml
owned_paths:
  - docs/contracts
  - docs/architecture
  - docs/operations
  - docs/testing
  - docs/agents
  - README.md
  - THIRD_PARTY_NOTICES.md
  - tools/validation/adr_registry.py
  - tools/validation/legacy_token_ratchet.py
  - tools/validation/test_legacy_token_ratchet.py
modules: []
dependencies: []
blockers:
  - none
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T12:00:00Z
head: 934b7e20b454296a02b96288e23a2840a9da900a
branch: claude/decanary-cut-docs-20261011
pr: 1528
status: validating
context_routes:
  - testing
owned_paths:
  - docs/contracts
  - docs/architecture
  - docs/operations
  - docs/testing
  - docs/agents
  - README.md
  - THIRD_PARTY_NOTICES.md
  - tools/validation/adr_registry.py
  - tools/validation/legacy_token_ratchet.py
  - tools/validation/test_legacy_token_ratchet.py
proven:
  - pytest tools/validation tools/agents tests/ci passes (policy-consistency excluded, needs the META checkout)
derived: []
unknown:
  - machine-checked evidence files that still name tracked code paths keep those paths until their deleting packets land
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs
  - README.md
  - THIRD_PARTY_NOTICES.md
  - tools/validation
validation:
  - command: python -m pytest tools/validation tools/agents tests/ci
    result: PASS
    evidence: 499 passed, policy-consistency deselected
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation and validator change only
blockers:
  - none
next_action: Archive this record once the PR is closed.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR #1528 is terminal once merged and the same-repository branch has no durable purpose.
source_branch_evidence: pending merge of PR #1528.
```

## Notes

Jira synchronization: pending/unknown (no mapped Story).
