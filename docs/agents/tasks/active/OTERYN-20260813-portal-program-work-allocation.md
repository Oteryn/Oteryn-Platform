---
task_id: OTERYN-20260813-portal-program-work-allocation
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
---

# OTERYN-20260813-portal-program-work-allocation

## Goal

Turn the current portal-completion backlog and the accepted Tibia/RubinOT/TibiaPal-derived product decisions into one durable execution project with explicit ownership, ordering, delegation rules, dependencies and terminal completion criteria.

## Acceptance criteria

- [x] The existing `OTERYN_PORTAL_COMPLETION` programme remains the single orchestration authority; no duplicate programme is created.
- [x] A canonical work-allocation record classifies every discussed P0-P2/P3 portal area as DONE, ARCHITECTURE_READY, OPEN, BLOCKED, IN_PROGRESS, DEFERRED or DECISION_REQUIRED.
- [x] Each open workstream names the primary execution role: `CHATGPT_ARCHITECT`, `CODEX_ELIGIBLE_WORKER`, `OWNER_DECISION`, or `PROTECTED_ENV_OPERATOR`.
- [x] Codex eligibility never constitutes permission to consume owner-funded Codex/OpenAI quota; every Codex invocation still requires explicit owner approval for that exact task/use.
- [x] Dependencies and recommended sequencing are explicit, including current PlayerCompanion work and open Issues #489/#490.
- [x] Existing architecture authority, module boundaries and live-state-first programme rules are preserved.
- [ ] Documentation changes pass repository governance/CI and are merged through a dedicated PR.

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
  - PR #1028 is independent active implementation and must not be modified by this documentation task
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T20:52:00+02:00
head: 4cfbeda928d28afee283ea96d7e61890ad3fb2a7
branch: docs/portal-program-allocation-final
pr: 1031
status: validating
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
  - main at task start is 638df04f616c93d80e33e1abf3f2cf0198163e7a.
  - OTERYN_PORTAL_COMPLETION already exists and remains the single programme orchestrator.
  - Issue #488 and Issue #365 are closed completed; Issue #489 and #490 remain open.
  - MODULE_CATALOG already contains PublicPortal, Announcements, Events, Downloads, LiveOps, PlayerCompanion, OperationsObservability, PublicEdge and PlatformAPI boundaries.
  - PR #1028 is an independent active PlayerCompanion Session Analyzer implementation and this task does not modify it.
  - AGENTS.md forbids owner-funded Codex/OpenAI quota use without explicit per-use owner permission.
  - The new work-allocation project maps all discussed portal workstreams to roles, dependencies, current status and completion gates.
  - OTERYN_PORTAL_COMPLETION and PORTAL_COMPLETION_DELIVERY_PLAN now link to the work-allocation project.
  - Draft PR #1031 contains only four intended documentation/programme files and no runtime changes.
derived:
  - The project can be executed by ChatGPT directly or selectively delegated to Codex-eligible workers only after explicit per-task owner approval.
unknown:
  - Exact final CI results for the current PR head after this checkpoint update.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Create a second independent portal-completion programme.
  - Treat Codex-eligible classification as standing permission to invoke Codex.
changed_paths:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/active/OTERYN-20260813-portal-program-work-allocation.md
validation:
  - command: exact branch diff versus main
    result: PASS
    evidence: compare main...docs/portal-program-allocation-final reports four intended documentation files only and no runtime changes.
  - command: full PR diff self-review
    result: PASS
    evidence: PR #1031 patch contains only the project allocation, programme linkage, delivery-plan linkage and task record; no architecture invariant or runtime behavior is changed.
  - command: repository governance/CI on PR final head
    result: NOT_RUN
    evidence: exact-head checks must restart after this checkpoint commit.
blockers:
  - none
next_action: inspect exact-head PR #1031 checks; if all required checks pass, mark ready and merge, then archive this task.
```

## Notes

This task changes documentation/programme routing only. It does not invoke Codex, deploy production, mutate protected environments, or modify the active PR #1028 implementation.
