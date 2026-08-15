---
task_id: OTERYN-20260815-client-distribution-platform
mode: implementation
issue: 1039
programme: OTERYN_PORTAL_COMPLETION
status: implementing
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md
  - docs/architecture/adr/0035-client-distribution-updater-trust.md
search_first:
  - live main, active tasks, open PRs and Downloads ownership overlap
---

# OTERYN-20260815-client-distribution-platform

## Goal

Implement the Platform-only portion of Issue #1039 under accepted ADR 0035 without private signing-key custody, external updater implementation, protected signing operations or production activation.

## Acceptance criteria

- [ ] Preserve the existing browser Download Center while adding explicit updater release/channel identity and monotonic sequencing that does not rely on lexical display versions.
- [ ] Add fail-closed exact platform/architecture artifact targeting and updater-policy state, including minimum-supported release, optional/recommended/required mode, withdrawal, release/target revocation and explicit rollback state.
- [ ] Separate browser publication from signed-generation/repository state and updater-active state.
- [ ] Verify and record exact signed-generation public metadata without storing or using private updater signing keys in Laravel.
- [ ] Make import/verification/activation idempotent and ambiguity-safe for stale/replayed generations, channel/target mismatch and revocation/rollback.
- [ ] Keep administrator-supplied SHA-256 presentation truthful and do not upgrade it into publisher verification.
- [ ] Add focused feature/integration/security/negative-path coverage and risk-proportional browser acceptance for changed user-visible surfaces.
- [ ] Pass exact-final-head required CI, whole-diff review and task closeout before merge.
- [ ] Real updater/client E2E, protected signing infrastructure and production activation remain separate gates and are not claimed by this task.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-client-distribution-platform.md
  - docs/agents/tasks/archive/OTERYN-20260815-client-distribution-platform.md
  - app/Downloads/**
  - config/downloads.php
  - database/migrations/*downloads*
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - tests/Unit/Downloads/**
  - tests/Feature/Downloads/**
modules:
  - Downloads
dependencies:
  - ADR 0035 / merged architecture PR #1038
  - terminal immutable artifact-reference repair Issue #948
blockers: []
cross_repository_tasks:
  - external updater implementation and protected signing remain separately authorized and out of scope
```

## Selector proof

Canonical selection was rerun from protected `main@5000f271db49215c93432b78397dd3560b49e7e7`.

- Entry 1: `TERMINAL` — the prior selector-reconciliation task is absent from `tasks/active/**`; PR #1058 and archive PR #1059 are merged.
- Entry 2: `TERMINAL` for the previous selector-reconciliation package — stale convenience text in `ACTIVE_WORK.md` cannot override live task/PR state and does not prevent deterministic selection under programme version 3.
- Entry 3: `TERMINAL` — no open `risk:high` repair Issue was returned; historical #948/#944/#941 are not current queue entries.
- Entry 4: `BLOCKED` — direct production/public-edge proof still requires protected-environment authority and evidence.
- Entry 5: `BLOCKED` / `DECISION_REQUIRED` — #317 and #319 require accepted native Character Authority command/result semantics; #320 additionally requires explicit product decision.
- Entry 6: `BLOCKED` for runtime delivery — LiveOps runtime promotion still requires exact authoritative runtime-status producer evidence; Public Today architecture #1049 is terminal and no independent ready runtime package was proven.
- Entry 7: `OWNED` — Issue #1060 is actively owned by PR #1061.
- Entry 8: `READY` — Issue #1039 is open `agent:ready`, architecture Issue #1037 is terminal through merged PR #1038/ADR 0035, prerequisite #948 is terminal, no open PR owns #1039, and the bounded Platform-only scope does not require external/protected mutations.

Work Allocation maps the selected package to `IMPLEMENTATION_OWNER`; Codex suitability is informational only and no owner-funded Codex/OpenAI invocation is authorized or required.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T08:38:00+02:00
head: 5000f271db49215c93432b78397dd3560b49e7e7
branch: feat/issue-1039-client-distribution-platform
pr: pending
status: implementing
context_routes:
  - downloads
  - architecture
  - security
  - agent-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-client-distribution-platform.md
  - docs/agents/tasks/archive/OTERYN-20260815-client-distribution-platform.md
  - app/Downloads/**
  - config/downloads.php
  - database/migrations/*downloads*
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - tests/Unit/Downloads/**
  - tests/Feature/Downloads/**
proven:
  - Protected main at selection was 5000f271db49215c93432b78397dd3560b49e7e7.
  - Issue #1039 is open and labelled agent:ready with an explicit Platform-only implementation boundary.
  - Architecture Issue #1037 is closed completed through merged PR #1038, which accepted ADR 0035 and created implementation handoff #1039.
  - Immutable artifact-reference prerequisite Issue #948 is closed completed.
  - Issue #1060 is already owned by open PR #1061 and therefore canonical entry 7 is not selectable.
  - No open risk:high repair Issue was returned by the live selector query.
derived:
  - Issue #1039 is the first unowned canonical READY candidate under OTERYN_PORTAL_COMPLETION programme version 3.
  - External updater implementation, private signing operations and production activation are not prerequisites for truthful completion of the bounded Platform-only slice.
unknown:
  - Exact current Downloads persistence/application shape and smallest migration/model boundary must be inspected before implementation edits.
conflicts: []
first_failure:
  marker: none-at-selection
  evidence: live selector found no blocker for the bounded Platform-only Issue #1039 package
rejected_hypotheses:
  - Stale ACTIVE_WORK routing text makes the archived selector task current: rejected because live task/PR state is authoritative.
  - Client Distribution is blocked until external updater/signing infrastructure exists: rejected by ADR 0035 and the canonical programme's explicit Platform-only boundary.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260815-client-distribution-platform.md
validation:
  - command: canonical OTERYN_PORTAL_COMPLETION live selector against main, active task existence, Issues and PR ownership
    result: PASS
    evidence: first unowned READY candidate is Issue #1039 after entries 1-7 were classified from live evidence
  - command: implementation tests
    result: NOT_RUN
    evidence: task was just selected and no runtime implementation change has been made yet
blockers: []
next_action: Inspect ADR 0035, CLIENT_DISTRIBUTION_ARCHITECTURE.md and the current Downloads persistence/application code on this branch, then implement the smallest complete Platform-only #1039 vertical slice without external or protected-environment operations.
```
