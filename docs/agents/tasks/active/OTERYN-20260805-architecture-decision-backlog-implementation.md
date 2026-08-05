---
task_id: OTERYN-20260805-architecture-decision-backlog-implementation
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
issue: 642
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
search_first:
  - duplicate architecture decision backlog implementation
  - overlapping open Issues, PRs and active task ownership
  - unresolved owner decisions suitable for initial seeding
optional_reads:
  - tools/validation/adr_registry.py
  - tools/validation/test_adr_registry.py
  - tools/validation/phpunit/AdrRegistryValidationTest.php
---

# OTERYN-20260805-architecture-decision-backlog-implementation

## Goal

Implement accepted ADR 0023 as one deterministic, offline-validatable inventory of unresolved architecture decision obligations, subordinate to accepted ADR authority and without importing completed programme history or implementation-only work.

## Acceptance criteria

- [x] Duplicate file, Issue, PR and active-owner searches found no competing implementation.
- [x] Initial seed candidates were deduplicated against accepted ADRs and current open Issues.
- [ ] Canonical JSON schema version 1 is added with deterministic serialization and explicit non-authority text.
- [ ] Initial records contain only unresolved owner decisions for Issues #586, #587 and #588.
- [ ] Standard-library validator fails closed for schema, lifecycle, evidence, references, authority, duplication and programme-projection defects.
- [ ] Positive, negative and boundary tests cover the validator.
- [ ] Existing PHPUnit discovery runs focused tests and repository validation without workflow changes.
- [ ] `ARCHITECTURE_AUTHORITY.md` documents routing, transitions and terminal removal.
- [ ] Architecture programme stores only compact active IDs and one next action.
- [ ] Exact-head workflows, fresh audit and review-thread inventory pass.
- [ ] PR merges through protected main, task is archived and ownership is released.

## Ownership

```yaml
owned_paths:
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - tools/validation/architecture_decision_backlog.py
  - tools/validation/test_architecture_decision_backlog.py
  - tools/validation/phpunit/ArchitectureDecisionBacklogValidationTest.php
  - docs/agents/tasks/active/OTERYN-20260805-architecture-decision-backlog-implementation.md
shared_path_lease:
  - path: docs/architecture/ARCHITECTURE_AUTHORITY.md
    holder: OTERYN-20260805-architecture-decision-backlog-implementation
    expires_at: 2026-08-06T00:45:00Z
  - path: docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
    holder: OTERYN-20260805-architecture-decision-backlog-implementation
    expires_at: 2026-08-06T00:45:00Z
forbidden_paths:
  - app/**
  - database/**
  - routes/**
  - resources/**
  - services/**
  - .github/workflows/**
  - repository settings
  - production and staging systems
  - external repositories
blockers: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T22:09:00Z
phase: implement
head: 5264712da5dff535b4612d8f221e148ccef0b6b0
branch: repair/issue-642-architecture-decision-backlog
pr: none
status: implementing
context_routes:
  - architecture
  - testing
  - agent-governance
owned_paths:
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - tools/validation/architecture_decision_backlog.py
  - tools/validation/test_architecture_decision_backlog.py
  - tools/validation/phpunit/ArchitectureDecisionBacklogValidationTest.php
  - docs/agents/tasks/active/OTERYN-20260805-architecture-decision-backlog-implementation.md
proven:
  - ADR 0023 is accepted and authorizes this repository-owned governance implementation.
  - No duplicate registry implementation Issue or open PR exists.
  - Issues 586, 587 and 588 each contain a material unresolved owner decision.
  - Completed ARCH-AUTH history and implementation-only Issues are not active decision obligations.
derived:
  - Three initial records are sufficient to prove non-empty schema behavior without importing unrelated backlog.
unknown:
  - Exact-head validation conclusions after implementation.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Seed completed ARCH-AUTH records.
  - Import all open architecture-labelled Issues.
  - Make ordinary validation depend on live GitHub API access.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260805-architecture-decision-backlog-implementation.md
validation:
  - command: repository, Issue and PR duplicate search
    result: PASS
    evidence: no competing registry implementation owner found
  - command: unresolved decision reconciliation
    result: PASS
    evidence: only Issues 586, 587 and 588 require unresolved owner policy choices in the initial seed
blockers: []
next_action: Add the canonical JSON registry, fail-closed validator, focused tests, PHPUnit bridge, authority routing and compact programme projection.
```
