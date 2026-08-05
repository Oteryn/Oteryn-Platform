---
task_id: OTERYN-20260805-architecture-decision-backlog
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
repository: blakinio/Oteryn-Platform
task_kind: discovery/audit/design
implementation_authorized: false
issue: 602
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
search_first:
  - existing architecture decision backlog or registry
  - overlapping Issues and PRs
  - open ADR allocations
optional_reads:
  - tools/validation/adr_registry.py
  - docs/agents/GOVERNANCE_CONTRACT.json
---

# OTERYN-20260805-architecture-decision-backlog

## Goal

Define the authority, schema, lifecycle and deterministic validation boundary for one machine-readable backlog of unresolved architecture decision obligations without duplicating accepted ADR authority, GitHub Issue workflow or programme continuation state.

## Acceptance criteria

- [x] Existing files, Issues and PRs were searched for a duplicate registry or owner.
- [x] Current authority and programme-state boundaries were grounded in ADR 0022 and live repository evidence.
- [x] At least two meaningful alternatives plus status quo were compared.
- [x] One recommendation and exact owner question were recorded.
- [x] A collision-free proposed ADR was allocated from live state.
- [x] No runtime, workflow, migration, deployment, infrastructure, production or external-repository path changed.
- [x] Repository owner accepted Option B.
- [x] ADR 0023 was updated to Accepted and a separate bounded implementation handoff was recorded.
- [x] Exact-head workflows passed with zero unresolved review threads.
- [ ] Protected-main required-check policy permits the documentation-only PR to merge without bypassing protection.
- [ ] PR #604 is merged, this task is archived and ownership is released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260805-architecture-decision-backlog.md
  - docs/agents/reports/OTERYN-20260805-architecture-decision-backlog-review.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  - docs/architecture/adr/README.md
shared_path_lease: []
forbidden_paths:
  - app/**
  - database/**
  - routes/**
  - resources/**
  - services/**
  - .github/workflows/**
  - repository branch-protection settings
  - production and staging systems
  - external repositories
blockers:
  - Issue #552 required-check policy is inconsistent with conditional CI execution for documentation-only changes
after_unblock:
  - merge PR #604 at the validated exact head
  - archive this design task
  - release all five owned paths
  - close Issue #602 as completed
  - start a separate bounded remediation task for the accepted ADR 0023 implementation
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-05T21:03:00Z
phase: validate
session_id: chatgpt-20260805T2231+0200-architecture-decision-backlog
session_role: architecture-adviser
execution_mode: chat
execution_reason: bounded GitHub-only architecture review
lease_expires_at: 2026-08-05T22:30:00Z
context_pressure: low
context_growth: stable
context_score: 4
estimate_confidence: high
decomposition_decision: single
validation_level: documentation
heavy_validation_runs: 1
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 1
head: 155053448e1b090dd47f7b0bf84d7d2ea223109b
branch: task/OTERYN-20260805-architecture-decision-backlog
pr: 604
status: blocked
context_routes:
  - architecture
  - agent-governance
  - ci-repair
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260805-architecture-decision-backlog.md
  - docs/agents/reports/OTERYN-20260805-architecture-decision-backlog-review.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  - docs/architecture/adr/README.md
proven:
  - Current architecture authority is defined by accepted ADR 0022 and ARCHITECTURE_AUTHORITY.md.
  - The programme embeds a compact decision_backlog but no independent schema or validator exists.
  - The ADR registry already has separate fail-closed validation and must not be duplicated.
  - Repository-file, Issue and PR searches found no existing machine-readable architecture decision backlog owner.
  - Issue #602 records the bounded decision and three alternatives.
  - The repository owner accepted Option B in the current invocation and the decision was recorded on Issue #602.
  - ADR 0023 records the accepted dedicated JSON backlog model and separate implementation handoff.
  - PR #604 head 0c3845b0a7a9a30f81fe42fcb2825693aacc20c4 passed all eight emitted workflows and has zero unresolved review threads.
  - CI run 31046599415 reports classify-changes success and test skipped for the documentation-only change set.
  - Main protection requires classify-changes and test; the merge API rejected the validated head with HTTP 405 because both required checks were expected.
  - Issue #552 already owns protected-main and required exact-head check alignment; the new evidence was appended there.
derived:
  - A dedicated JSON backlog provides the clearest boundary between unresolved obligations, accepted ADR authority, Issue workflow and programme continuation.
  - The design package is complete but cannot be merged safely until the required-check policy and conditional CI model agree.
unknown:
  - Issue #552 repair implementation and validation head
  - final PR #604 merge commit and design-task archive commit
conflicts:
  - Protected-main requires a successful test context while the CI workflow intentionally skips that job for documentation-only changes.
first_failure:
  marker: protected-branch-required-check-mismatch
  evidence: merge API HTTP 405 after all emitted workflows passed; CI job test concluded skipped
rejected_hypotheses:
  - The existing ADR registry can also serve as the unresolved-decision backlog.
  - GitHub Issues alone provide reproducible exact-head repository validation.
  - The programme queue can become a permanent registry without changing its authority role.
  - A branch-protection bypass is an acceptable closeout path.
  - Removing test enforcement without an always-running replacement gate preserves runtime safety.
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/reports/OTERYN-20260805-architecture-decision-backlog-review.md
  - docs/agents/tasks/active/OTERYN-20260805-architecture-decision-backlog.md
  - docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  - docs/architecture/adr/README.md
validation:
  - command: repository, Issue and PR duplicate search
    result: PASS
    evidence: no existing machine-readable architecture decision backlog owner found
  - command: ADR allocation inventory and open architecture PR search
    result: PASS
    evidence: highest observed prefix was 0022 and no competing 0023 allocation existed
  - command: owner decision reconciliation
    result: PASS
    evidence: Option B acceptance is recorded on Issue #602 and ADR 0023 is Accepted
  - command: exact-head GitHub Actions at 0c3845b0a7a9a30f81fe42fcb2825693aacc20c4
    result: PASS
    evidence: all eight emitted workflows completed successfully
  - command: PR review-thread inventory
    result: PASS
    evidence: zero unresolved review threads
  - command: protected-main merge gate
    result: BLOCKED
    evidence: required test context is skipped for documentation-only changes and merge API returns HTTP 405
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only architecture decision review changes no executable behavior
blockers:
  - Issue #552 must provide one stable always-emitted merge gate that accepts explicit documentation-only non-applicability while preserving full runtime test enforcement.
next_action: Repair Issue #552 without bypassing protection, then revalidate and merge PR #604, archive this design task and start the separate ADR 0023 implementation package.
```