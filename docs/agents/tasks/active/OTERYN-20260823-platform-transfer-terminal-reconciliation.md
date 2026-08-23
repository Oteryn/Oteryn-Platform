---
task_id: OTERYN-20260823-platform-transfer-terminal-reconciliation
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
search_first:
  - blakinio/Oteryn-Platform
optional_reads: []
---

# OTERYN-20260823-platform-transfer-terminal-reconciliation

## Goal

Close the remaining mutable repository-coordinate and control-plane evidence gaps for Platform transfer Issue #1155 without rewriting historical provenance or touching production state.

## Acceptance criteria

- [ ] Active authority/configuration surfaces use `Oteryn/Oteryn-Platform` rather than the historical owner coordinate.
- [ ] A deterministic regression rejects reintroduction of the historical coordinate into current authority surfaces.
- [ ] Current owner GHCR publication, runner routing, environments, protection and transfer identity are revalidated without exposing secrets.
- [ ] Historical evidence and immutable migration provenance remain unchanged.
- [ ] Focused tests and exact-head required CI pass; Issue #1155 is terminally reconciled.
## Ownership

```yaml
owned_paths:
  - .github/ISSUE_TEMPLATE/**
  - SECURITY.md
  - docs/operations/**
  - docs/agents/*
  - tools/agents/historical_work_reconciliation.py
  - tests/ci/test_current_repository_coordinate.py
  - tests/ci/test_repository_policy.py
modules:
  - repository-governance
  - migration-closeout
dependencies:
  - Oteryn/Oteryn-Platform#1155
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn#16
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-23T12:46:36Z
head: 0ccbcdc48401e28360f6f814386319cf2c6e7f5d
branch: migration/issue-1155-terminal-coordinate-reconciliation
pr: none
status: validating
context_routes:
  - governance
  - migration
owned_paths:
  - current Platform authority/configuration surfaces only
proven:
  - repository ID 1305155726 resolves at Oteryn/Oteryn-Platform
  - historical blakinio/Oteryn-Platform URL resolves to the same repository ID
  - current-owner GHCR publish run 32625997593 succeeded
  - regression is RED on fourteen current authority files
derived:
  - package metadata API visibility is not required to prove the already-observed current-owner publish identity
unknown:
  - GHCR package settings metadata remains unavailable without read:packages
conflicts: []
first_failure:
  marker: test_current_repository_coordinate
  evidence: fourteen active authority files still name blakinio/Oteryn-Platform
rejected_hypotheses:
  - all remaining legacy-coordinate hits are historical-only
changed_paths:
  - tests/ci/test_current_repository_coordinate.py
  - docs/agents/tasks/active/OTERYN-20260823-platform-transfer-terminal-reconciliation.md
validation:
  - command: python -m pytest tests/ci/test_current_repository_coordinate.py -q
    result: FAIL
    evidence: expected TDD RED; 14 active authority offenders
blockers:
  - none
next_action: commit and push the coherent migration reconciliation candidate, then open the draft PR and observe exact-head CI
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded migration-reconciliation branch is disposable after terminal squash merge
source_branch_evidence: pending
```

## Notes

Historical ADRs, evidence, reports, archived tasks and immutable migration records remain historical and are intentionally not mass-rewritten.
