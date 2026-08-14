---
task_id: OTERYN-20260814-public-today-architecture
mode: architecture
issue: 1049
status: waiting
programme: OTERYN_PORTAL_COMPLETION
project_lane: oteryn-platform-core
phase: validate
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
---

# OTERYN-20260814-public-today-architecture

## Goal

Define one focused canonical `PublicPortal Today` architecture that turns accepted ADR 0032 composition/privacy rules into an implementation-ready public guest slice without creating a new data authority or leaking owner-private state through shared caching.

## Acceptance criteria

- [x] Current `main`, open PR/task ownership and accepted Today authority are reconciled before implementation.
- [x] Focused `PUBLIC_PORTAL_TODAY_ARCHITECTURE.md` defines source composition, representation classes, partial failure, freshness, applicability, cache, SEO, accessibility, observability and performance boundaries.
- [x] The first implementation slice is explicitly `PUBLIC_GUEST`; owner-private PlayerCompanion composition remains a separate security-sensitive gate unless all ADR 0032 cache-isolation acceptance can be delivered together.
- [x] LiveOps is consumed as a bounded source and unavailable/stale state cannot be fabricated as normal/offline/none.
- [x] `ARCHITECTURE_AUTHORITY.md` routes Today composition to the focused architecture and ADR 0032.
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
updated_at: 2026-08-14T13:20:28Z
head: c5963cfbb46686f3ec07ac3c7e81a7c8e270452c
material_head: c5963cfbb46686f3ec07ac3c7e81a7c8e270452c
branch: docs/OTERYN-20260814-public-today-architecture
pr: 1055
status: waiting
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
  - Issue #1049 owns the package and PR #1055 is open from docs/OTERYN-20260814-public-today-architecture.
  - Material diff at c5963cfbb46686f3ec07ac3c7e81a7c8e270452c contains exactly three declared paths, with one Architecture Authority routing row and no runtime deletion/change.
  - Exact-head review 4937489372 records PASS with no material findings and E2E NOT_APPLICABLE for the architecture-only material head.
  - Security architecture deny-by-default, server-side ownership and no-private-data logging rules are compatible with the focused Today public/private boundary.
derived:
  - A focused Today document can narrow implementation sequencing without creating a new ADR because durable ownership/privacy policy is already accepted.
  - Public guest first materially reduces cross-principal cache risk and allows a complete user-facing slice before private personalization.
unknown:
  - exact future public Today route path
  - exact provider registry/card DTO implementation
  - exact cache technology/TTL and production CDN path
conflicts: []
first_failure:
  marker: exact-head-ci-pending
  evidence: first two aggregate observations of material head c5963cfbb46686f3ec07ac3c7e81a7c8e270452c found the eight emitted workflows queued; no failure conclusion exists
rejected_hypotheses:
  - Today should become a new source-of-truth domain module.
  - A mixed public/private response can inherit public shared-cache eligibility.
  - Source dependency failure can be shown as authoritative empty/normal state.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260814-public-today-architecture.md
  - docs/architecture/PUBLIC_PORTAL_TODAY_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
validation:
  - command: accepted-source reconciliation
    result: PASS
    evidence: ADR 0032, LiveOps architecture, Module Catalog, Security Architecture and work allocation align with public-guest-first focused design
  - command: exact-head architecture self-review
    result: PASS
    evidence: review 4937489372 on c5963cfbb46686f3ec07ac3c7e81a7c8e270452c
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture-only task creates no executable user/system path
  - command: exact-head PR CI
    result: PENDING
    evidence: runs 31804205611, 31804205662, 31804205668, 31804205655, 31804205692, 31804205746, 31804205729 and 31804205736 were queued at the two permitted ordinary observations
blockers: []
next_action: Verify live PR #1055 head after the checkpoint-only commit, perform one exact-head full-diff self-review, then resume bounded final CI/review/merge closeout without changing architecture content unless a material finding appears.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: 20260814T131704Z-public-today-architecture
  session_started_at: 2026-08-14T13:17:04Z
  checkpointed_at: 2026-08-14T13:20:28Z
  last_progress_at: 2026-08-14T13:20:28Z
  phase: final-validation-and-merge
  exact_head: c5963cfbb46686f3ec07ac3c7e81a7c8e270452c
  pull_request: 1055
  active_operation: none
  external_run_ids: [31804205611, 31804205662, 31804205668, 31804205655, 31804205692, 31804205746, 31804205729, 31804205736]
  operation_started_at: null
  wait_deadline_at: null
  check_generation: material-head-c5963cf
  checks_used: 2
  status: waiting
  safe_to_resume: true
  resume_condition: GitHub exposes the live PR #1055 head and its exact-head workflow/review state.
  next_action: Fetch PR #1055, treat the live checkpoint commit as the new exact head, self-review that docs-only delta, then validate its emitted CI and merge/archive only if every gate passes.
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: c5963cfbb46686f3ec07ac3c7e81a7c8e270452c
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - review 4937489372
    - no runtime, API, persistence, frontend, workflow or external-repository change
    - removing the focused document/routing row is documentation-only rollback
```

## Notes

No external/server repository was accessed. No production, protected environment, runtime route or private cache implementation is authorized or performed by this task.
