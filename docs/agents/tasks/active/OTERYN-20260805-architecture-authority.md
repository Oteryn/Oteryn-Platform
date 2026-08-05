---
task_id: OTERYN-20260805-architecture-authority
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
issue: 548
status: active
agent: ChatGPT
branch: task/OTERYN-20260805-architecture-authority
base_branch: main
exact_base: 3ab77c072dce796b09004c54b649db009a75d524
created: 2026-08-05T16:48:00+02:00
updated: 2026-08-05T16:48:00+02:00
risk: medium
execution_mode: github-only
task_kind: architecture-review
implementation_authorized: false
production_activation_authorized: false
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260805-architecture-authority.md
  - docs/agents/reports/OTERYN-20260805-architecture-authority-review.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
shared_path_lease: []
excluded_overlap:
  - PR 542 native-protocol implementation and contracts
  - PR 541 public-edge checkpoint
  - application, workflow, migration, dependency, deployment and infrastructure paths
---

# Canonical architecture authority review

## Goal

Determine how current architecture truth, accepted decisions, implementation evidence and historical planning documents must be ordered so stale text cannot direct incompatible work.

## Entry evidence

- exact trusted base: `main@3ab77c072dce796b09004c54b649db009a75d524`;
- programme state was `ready` with `decision_backlog: not_reconciled`;
- no existing open Issue matched this bounded decision;
- Issue #548 owns the decision;
- active PR #542 overlaps native-protocol contracts and is excluded;
- merged PR #453 is reused as evidence for module-catalogue drift.

## Acceptance

- [x] inspect live main, programme state, open PRs and overlapping ownership;
- [x] compare current system architecture, roadmap, module catalogue, ADR index and repository map;
- [x] classify proven contradictions and unknowns;
- [x] compare at least three authority-model alternatives;
- [x] create a deduplicated decision Issue;
- [ ] persist the complete review report and implementation handoffs;
- [ ] update programme state with the durable next action;
- [ ] open one documentation-only PR;
- [ ] pass exact-head documentation/governance CI or persist the precise pending blocker;
- [ ] close or archive the task and release ownership after terminal PR state.

## Checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T16:48:00+02:00
status: implementing
phase: decision-package
exact_base: 3ab77c072dce796b09004c54b649db009a75d524
branch: task/OTERYN-20260805-architecture-authority
pull_request: none
issue: 548
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260805-architecture-authority.md
  - docs/agents/reports/OTERYN-20260805-architecture-authority-review.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
proven:
  - SYSTEM_ARCHITECTURE.md contains initial-phase statements contradicted by newer accepted repository state.
  - REPOSITORY_MAP.md references a missing docs/architecture/overview.md path.
  - The ADR directory contains duplicate 0008 numbering while README.md is non-exhaustive.
  - PR 453 already records module-catalogue drift and missing first-class boundaries.
derived:
  - Implicit document precedence can cause an agent to select obsolete constraints or duplicate decisions.
unknown:
  - Whether the owner accepts an authority-index model as the durable canonical solution.
conflicts:
  - Initial target/non-goal text conflicts with current roadmap, module catalogue, contracts and merged implementation evidence.
blockers: []
next_action: Persist the decision matrix and handoff report, update programme state, then open a documentation-only PR.
```

## E2E

`NOT_APPLICABLE`: this task changes architecture-review documentation only and does not alter runtime behavior.