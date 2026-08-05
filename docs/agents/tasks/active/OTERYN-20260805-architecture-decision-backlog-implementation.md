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
- [x] Canonical JSON schema version 1 is added with deterministic serialization and explicit non-authority text.
- [x] Initial records contain only unresolved owner decisions for Issues #586, #587 and #588.
- [x] Standard-library validator fails closed for schema, lifecycle, evidence, references, authority, duplication and programme-projection defects.
- [x] Positive, negative and boundary tests cover the validator.
- [x] Existing PHPUnit discovery runs focused tests and repository validation without workflow changes.
- [x] `ARCHITECTURE_AUTHORITY.md` documents routing, transitions and terminal removal.
- [x] Architecture programme stores only compact active IDs and one next action.
- [ ] Final exact-head workflows, fresh audit and review-thread inventory pass.
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
updated_at: 2026-08-05T22:22:00Z
phase: validate
head: 7cfc6892d27fac6105dbe633e20fbafc009b8cd3
branch: repair/issue-642-architecture-decision-backlog
pr: 650
status: validating
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
  - Current repository metadata reports delete_branch_on_merge enabled; Issue 586 remains open because no canonical exception, recovery and cleanup policy was found.
  - Completed ARCH-AUTH history and implementation-only Issues are excluded from the active registry.
  - The JSON registry contains exactly three decision_required records and grants no implementation authority.
  - The programme contains only the exact active ID projection and one next_action.
  - Ten focused validator tests pass in local preflight.
derived:
  - Three initial records are sufficient to prove non-empty schema behavior without importing unrelated backlog.
unknown:
  - Final exact-head workflow conclusions after current-main synchronization.
conflicts: []
first_failure:
  marker: stale-issue-evidence
  evidence: Initial Issue 586 wording said automatic deletion was disabled, but live repository metadata reports delete_branch_on_merge=true; the record was narrowed to the still-unresolved exception and recovery policy.
rejected_hypotheses:
  - Seed completed ARCH-AUTH records.
  - Import all open architecture-labelled Issues.
  - Make ordinary validation depend on live GitHub API access.
  - Duplicate record summaries inside programme state.
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260805-architecture-decision-backlog-implementation.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - tools/validation/architecture_decision_backlog.py
  - tools/validation/phpunit/ArchitectureDecisionBacklogValidationTest.php
  - tools/validation/test_architecture_decision_backlog.py
validation:
  - command: repository, Issue and PR duplicate search
    result: PASS
    evidence: no competing registry implementation owner found
  - command: unresolved decision reconciliation
    result: PASS
    evidence: only Issues 586, 587 and 588 remain initial owner-policy decisions
  - command: python3 tools/validation/test_architecture_decision_backlog.py
    result: PASS
    evidence: ten positive, negative and boundary tests passed in preflight
  - command: python3 tools/validation/architecture_decision_backlog.py
    result: PASS
    evidence: three active records passed schema v1 validation in preflight
blockers: []
next_action: Complete exact-head CI and independent audit, synchronize with current main, merge PR 650 through protection, then archive the task and release ownership.
```
