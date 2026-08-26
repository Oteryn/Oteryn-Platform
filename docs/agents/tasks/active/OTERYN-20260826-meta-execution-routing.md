---
task_id: OTERYN-20260826-meta-execution-routing
governing_issue: 1271
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20260826-meta-execution-routing

## Goal

Adopt merged META execution routing by reference without changing Platform runtime, runner hosts or production systems.

## Acceptance criteria

- [ ] Root instructions enforce CI/isolated-workspace first and RDC default-deny.
- [ ] Fresh GitHub resume state and parallel-first lane ownership are required.
- [ ] Repository-native governance validation and exact-head CI pass.

## Ownership

```yaml
owned_paths:
  - AGENTS.md
  - docs/agents/tasks/active/OTERYN-20260826-meta-execution-routing.md
modules:
  - agent-governance
dependencies:
  - Oteryn/Oteryn@8fac1d55805fc3372351ea0a55ad7728b3570ebc
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn#90 merged
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-26T16:55:00Z
head: UNKNOWN
branch: governance/meta-execution-routing-1271
pr: none
status: implementing
context_routes:
  - governance
owned_paths:
  - AGENTS.md
  - docs/agents/tasks/active/OTERYN-20260826-meta-execution-routing.md
proven:
  - META policy merged at 8fac1d55805fc3372351ea0a55ad7728b3570ebc
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths: []
validation:
  - command: pending governance validation
    result: NOT_RUN
    evidence: implementation in progress
blockers:
  - none
next_action: validate scoped instructions and open Platform PR
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository governance PR
source_branch_evidence: pending
```
