---
task_id: OTERYN-20260813-portal-program-work-allocation
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
search_first:
  - active portal completion tasks and PRs
optional_reads:
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/agents/programs/GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md
---

# OTERYN-20260813-portal-program-work-allocation

## Goal

Turn the current portal-completion backlog and the accepted Tibia/RubinOT/TibiaPal-derived product decisions into one durable execution-allocation companion with explicit ownership, dependency routing and terminal completion criteria, while keeping `OTERYN_PORTAL_COMPLETION` as the only work-selection authority.

## Acceptance criteria

- [x] The existing `OTERYN_PORTAL_COMPLETION` programme remains the single orchestration and selection authority; no duplicate scheduler/programme is created.
- [x] A canonical work-allocation record classifies the discussed P0-P3 portal areas as `DONE`, `ARCHITECTURE_READY`, `OPEN`, `BLOCKED`, `IN_PROGRESS`, `DEFERRED`, `DECISION_REQUIRED` or `REJECTED`.
- [x] Core Account Center/Character Portfolio and PublicPortal Today are explicitly allocated and cannot disappear from completion accounting.
- [x] Execution roles are model-agnostic: `ARCHITECTURE_COORDINATOR`, `IMPLEMENTATION_OWNER`, `OWNER_DECISION` and `PROTECTED_ENV_OPERATOR`; Codex suitability is a separate non-authorizing attribute.
- [x] Codex suitability never constitutes permission to consume owner-funded Codex/OpenAI quota; every such invocation still requires explicit owner approval for that exact task/use.
- [x] The companion does not define a competing fallback sequence; detailed next-task selection remains in `OTERYN_PORTAL_COMPLETION.md`.
- [x] PlayerCompanion follow-up ordering matches `PLAYER_COMPANION_ARCHITECTURE.md`.
- [x] Game Catalog work reuses `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md` and keeps Platform-only work separate from server/game work requiring separate authorization.
- [x] Issue #489 and #490 are split by their actual concern/authority boundaries rather than treated as one simplified workstream each.
- [x] `project_lane: oteryn-platform-core` prevents broad programme text from being misclassified into the Bazaar lane.
- [x] Existing architecture authority, module boundaries and live-state-first programme rules are preserved.
- [ ] Documentation changes pass exact-head repository governance/CI and are merged through PR #1031.

## Ownership

```yaml
owned_paths:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/active/OTERYN-20260813-portal-program-work-allocation.md
  - docs/agents/tasks/archive/OTERYN-20260813-portal-program-work-allocation.md
modules:
  - PublicPortal
  - Accounts
  - Characters
  - Wiki
  - GameCatalog
  - PublicGameData
  - LiveOps
  - Downloads
  - PlatformAPI
  - PlayerCompanion
  - OperationsObservability
  - PublicEdge
dependencies:
  - OTERYN_PORTAL_COMPLETION
  - GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM for detailed Game Catalog decomposition
  - PR #1028 is independent active PlayerCompanion implementation and must not be modified by this documentation task
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-13T21:36:00+02:00
head: fa77b73f66467b6c8ff5305917d8b129e3b9927c
branch: docs/portal-program-allocation-final
pr: 1031
status: validating
phase: validate
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
execution_mode: chat-github
execution_reason: narrow documentation/programme corrections are safely executable through the GitHub connector without owner-funded AI invocation
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: focused
session_rotation_count: 0
heavy_validation_runs: 1
stale_takeover_count: 0
human_interruptions: 0
context_routes:
  - architecture
  - agent-governance
owned_paths:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/active/OTERYN-20260813-portal-program-work-allocation.md
  - docs/agents/tasks/archive/OTERYN-20260813-portal-program-work-allocation.md
proven:
  - main was synchronized into the task branch at f100334b40181b520a289cf81b28b7f68d26c4ef before review repair continued.
  - OTERYN_PORTAL_COMPLETION exists and remains the single programme selector.
  - Issue #488 and Issue #365 are closed completed; Issue #489 and #490 remain open and span multiple concerns.
  - PR #1028 is an independent active PlayerCompanion Session Analyzer implementation and this task does not modify it.
  - GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM already owns detailed Game Catalog producer/consumer completion sequencing.
  - AGENTS.override.md forbids Platform-invocation server/game repository access without separate owner authorization.
  - PR #1031 review identified three material findings: missing project lane, missing Account Center/Character Portfolio and Today rows, and a competing selection sequence.
  - The repaired allocation adds the missing workstreams, removes the competing scheduler, restores accepted PlayerCompanion order, separates #489/#490 concerns and adds model-agnostic ownership.
  - Exact PR diff at fa77b73f66467b6c8ff5305917d8b129e3b9927c contains only the four declared documentation/programme paths and no runtime changes.
derived:
  - The allocation can remain useful as a role/dependency companion without becoming a second source of task priority.
unknown:
  - Exact required-check results on the next repaired checkpoint head.
conflicts: []
first_failure:
  marker: checkpoint-validation-result-enum
  evidence: CI run 31736230255 job 94568413227 failed because validation item 2 used unsupported result IN_PROGRESS; allowed results are BLOCKED, FAIL, NOT_APPLICABLE, NOT_RUN or PASS.
rejected_hypotheses:
  - Create a second independent portal-completion programme.
  - Let the allocation board choose a later workstream before the canonical programme selects it.
  - Treat Codex suitability as standing permission to invoke Codex.
  - Treat Issue #489 as only Game Catalog or Issue #490 as only public-edge work.
changed_paths:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/active/OTERYN-20260813-portal-program-work-allocation.md
validation:
  - command: synchronization with current main
    result: PASS
    evidence: branch contains merge parent f100334b40181b520a289cf81b28b7f68d26c4ef via sync commit 9a88e393fa574471eb6fda05d04cdb66ec063d71.
  - command: PR review findings content reconciliation
    result: PASS
    evidence: exact diff addresses all three material findings by declaring oteryn-platform-core, adding Account Center/Character Portfolio and Today, and removing the competing scheduler.
  - command: exact PR full-diff self-review
    result: PASS
    evidence: PR #1031 at fa77b73f66467b6c8ff5305917d8b129e3b9927c changes only four intended documentation/programme files; no runtime or external-repository behavior is changed.
  - command: executable runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: this task changes documentation/programme routing only and introduces no executable user or integration journey.
  - command: PR review-thread hygiene
    result: FAIL
    evidence: three repaired review threads are outdated but still reported unresolved; two have repair replies and thread-resolution remains required before merge.
  - command: repository governance/CI on fa77b73f66467b6c8ff5305917d8b129e3b9927c
    result: FAIL
    evidence: CI and Agent Governance failed because of the invalid IN_PROGRESS validation enum; six other returned workflows passed and this checkpoint repair removes that root cause.
blockers:
  - none
next_action: inspect the new exact-head required checks, resolve the three outdated repaired review threads, then merge only when every required gate passes.
```

## Notes

This task changes documentation/programme routing only. It does not invoke Codex, deploy production, mutate protected environments, access or modify a server/game repository, or modify the active PR #1028 implementation.
