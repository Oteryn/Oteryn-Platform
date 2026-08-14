---
task_id: OTERYN-20260814-public-today-architecture
mode: architecture
issue: 1049
status: implementing
programme: OTERYN_PORTAL_COMPLETION
project_lane: oteryn-platform-core
phase: implement
execution_mode: github_connector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/LIVEOPS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
search_first:
  - open Today/PublicPortal PRs and tasks, accepted ADR 0032 cache/privacy authority, current LiveOps architecture
---

# OTERYN-20260814-public-today-architecture

## Goal

Define one focused canonical `PublicPortal Today` architecture that turns accepted ADR 0032 composition/privacy rules into an implementation-ready public guest slice without creating a new data authority or leaking owner-private state through shared caching.

## Acceptance criteria

- [x] Current `main`, open PR/task ownership and accepted Today authority are reconciled before implementation.
- [x] Focused `PUBLIC_PORTAL_TODAY_ARCHITECTURE.md` defines source composition, representation classes, partial failure, freshness, applicability, cache, SEO, accessibility, observability and performance boundaries.
- [x] The first implementation slice is explicitly `PUBLIC_GUEST`; owner-private PlayerCompanion composition remains a separate security-sensitive gate unless all ADR 0032 cache-isolation acceptance can be delivered together.
- [x] LiveOps is consumed as a bounded source and unavailable/stale state cannot be fabricated as normal/offline/none.
- [ ] `ARCHITECTURE_AUTHORITY.md` routes Today composition to the focused architecture and ADR 0032.
- [x] `MODULE_CATALOG.md` remains semantically correct: PublicPortal is AVAILABLE as a module while Today itself remains unimplemented.
- [x] Portal work allocation remains `PublicPortal Today | ARCHITECTURE_READY`; no status promotion is justified by architecture-only work.
- [x] ADR allocation is `NOT_APPLICABLE`: ADR 0032 already owns the durable composition/privacy/cache decision.
- [ ] Exact final-head documentation/governance validation and PR hygiene pass.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` because this task adds no executable route, backend, persistence or frontend behavior.
- [ ] PR is merged, Issue #1049 closed, task archived and ownership released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-public-today-architecture.md
  - docs/architecture/PUBLIC_PORTAL_TODAY_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
modules:
  - PublicPortal Today architecture
dependencies:
  - ADR 0032
  - docs/architecture/LIVEOPS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
blockers: []
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T13:17:04Z
head: 166561fe066b12310fb534172542e60b51484c46
branch: docs/OTERYN-20260814-public-today-architecture
pr: none
status: implementing
context_routes:
  - architecture
  - security
  - testing
  - agent-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-public-today-architecture.md
  - docs/architecture/PUBLIC_PORTAL_TODAY_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
proven:
  - Trusted main at selection is 166561fe066b12310fb534172542e60b51484c46.
  - PR #1047 merged focused LiveOps architecture and PR #1048 archived that task before this additional task was selected.
  - ADR 0032 is accepted and defines Today as PublicPortal composition, `PUBLIC_GUEST` / `PRIVATE_PERSONALIZED` / fail-closed representation classes and strict private/shared-cache isolation.
  - Current Module Catalog already states Today is a future PublicPortal composition and forbids source-truth transfer.
  - Portal work allocation already marks PublicPortal Today `ARCHITECTURE_READY` with implementation remaining.
  - Issue #1049 and branch docs/OTERYN-20260814-public-today-architecture were created from trusted main.
derived:
  - A focused Today document can narrow implementation sequencing without creating a new ADR because durable ownership/privacy policy is already accepted.
  - Public guest first materially reduces cross-principal cache risk and allows a complete user-facing slice before private personalization.
unknown:
  - exact future public Today route path
  - exact provider registry/card DTO implementation
  - exact cache technology/TTL and production CDN path
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Today should become a new source-of-truth domain module.
  - A mixed public/private response can inherit public shared-cache eligibility.
  - Source dependency failure can be shown as authoritative empty/normal state.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260814-public-today-architecture.md
  - docs/architecture/PUBLIC_PORTAL_TODAY_ARCHITECTURE.md
validation:
  - command: accepted-source reconciliation
    result: PASS
    evidence: ADR 0032, LiveOps architecture, Module Catalog and work allocation align with public-guest-first focused design
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture-only task creates no executable user/system path
blockers: []
next_action: Route Today in ARCHITECTURE_AUTHORITY.md, open the exact-head architecture PR, self-review the full diff and complete required CI/closeout.
```

## Notes

No external/server repository was accessed. No production, protected environment, runtime route or private cache implementation is authorized or performed by this task.
