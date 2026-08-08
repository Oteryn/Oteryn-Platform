---
task_id: OTERYN-20260808-portal-architecture-delta
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-content
task_kind: architecture
implementation_authorized: true
execution_mode: github
execution_reason: Platform-only architecture/research reconciliation can be completed through repository documentation and GitHub validation
status: validating
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/0025-player-companion-and-portal-tools-boundary.md
search_first:
  - open Platform PRs and active task ownership
  - current Tibia, RubinOT, TibiaPal and player-tool benchmark deltas
optional_reads:
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/agents/reports/OTERYN-20260806-portal-player-tools-benchmark.md
---

# OTERYN-20260808-portal-architecture-delta

## Goal

Re-evaluate the WWW portal architecture against current Platform `main` and current public MMORPG/player-tool patterns, then persist only material bounded refinements that fit already accepted Platform ownership. Do not redesign the sound Laravel modular-monolith foundation and do not access or modify server/game repositories.

## Acceptance criteria

- [x] Reconcile the current portal-completeness and PlayerCompanion architecture with the 2026-08-08 benchmark delta.
- [x] Distinguish genuinely missing architectural ownership from already-covered or intentionally deferred capabilities.
- [x] Define a bounded first-party `Today`/command-centre composition without making the homepage a new source of truth.
- [x] Clarify player tracking/routine/watch semantics inside the accepted PlayerCompanion boundary while keeping Notifications delivery-only.
- [x] Clarify stable server-specific system definition versus editorial explanation versus live operational state ownership.
- [x] Preserve explicit world/profile/ruleset/season/version/freshness dimensions and avoid irreversible single-world assumptions.
- [x] Do not create a new microservice/module when an accepted owner already exists.
- [x] Do not expand into Oteryn-v2, Canary, runtime, production, payment or protected-environment work.
- [ ] Complete exact-head documentation self-review, applicable CI/review hygiene and lifecycle closeout.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-portal-architecture-delta.md
  - docs/agents/reports/OTERYN-20260808-portal-product-delta.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
modules:
  - PublicPortal
  - PlayerCompanion
  - GameCatalog
  - LiveOps
  - PublicGameData
  - Notifications
dependencies:
  - ADR 0025
  - Issue #302
  - Issue #301
  - Issue #489
blockers:
  - none
cross_repository_tasks:
  - none
```

Open PR #338 owns Game Catalog consumer implementation paths and does not overlap these declared architecture paths. Open PR #541 owns only the existing public-domain repair task record and does not overlap this task. `MODULE_CATALOG.md` was reviewed but did not require modification because no module/status boundary changed.

## Delivered architecture delta

The 2026-08-08 evidence strengthened four product patterns without invalidating ADR 0025 or the current module split:

1. `PublicPortal` may compose a `Today`/command-centre view from bounded sources, but owns no underlying runtime/game/editorial truth.
2. Owner-private tracking, routines, watch preferences and derived progress/change signals belong to `PlayerCompanion.ProgressTracker`; `Notifications` remains delivery-only.
3. Structured server-specific system definitions belong under `GameCatalog`, while `Wiki` explains them and `LiveOps` owns current schedule/rotation/runtime state.
4. A future World Hub is a public composition of configured world presentation, `PublicGameData`, `LiveOps` and optional evidence-backed analytics, never routing authority.

Community-submitted hunt evidence remains P2/discovery because provenance, sampling bias, manipulation, privacy and moderation require a separate contract.

No new ADR is required: these are refinements inside the already accepted ADR 0025/PublicPortal/GameCatalog/LiveOps ownership model rather than a new durable deployable boundary.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-08T23:58:00+02:00
invocation_started_at: 2026-08-08T23:48:00+02:00
last_progress_at: 2026-08-08T23:58:00+02:00
head: f1dbe51c7502636f3a930f50d481af8dcfa627ea
branch: docs/OTERYN-20260808-portal-architecture-delta
pr: none
status: validating
phase: validate
session_id: agent-20260808-2348-portal-architecture-delta
session_role: architecture
project_lane: oteryn-platform-content
execution_mode: github
context_routes:
  - architecture
  - web-cms
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-portal-architecture-delta.md
  - docs/agents/reports/OTERYN-20260808-portal-product-delta.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one cohesive documentation-only portal architecture reconciliation with no overlapping runtime ownership
proven:
  - Platform main was 3c6f5192cdcd6bfc999ae0a8c731b88182c65bf4 at task start
  - WWW-only repository scope is mandatory by trusted main governance
  - current architecture retains Laravel modular monolith and Blade baseline
  - PlayerCompanion and LiveOps boundaries are already accepted by ADR 0025
  - architecture decision backlog had zero active records at task start
  - current open PRs do not overlap declared architecture paths
  - benchmark delta fits accepted owners and requires no new top-level module or ADR
derived:
  - Today composition, owner-private tracking/routines, typed server-system ownership and World Hub composition are the only material architecture refinements from the refreshed benchmark
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - portal requires architectural rewrite
  - player tracking requires a new standalone microservice
  - Notifications should own subscription/tracking rules
  - server-specific systems require a generic plugin module
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-portal-architecture-delta.md
  - docs/agents/reports/OTERYN-20260808-portal-product-delta.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
validation:
  - command: focused canonical-document reconciliation
    result: PASS
    evidence: refinements remain inside accepted module/ADR boundaries and do not conflict with open PR ownership
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only architecture package changes no executable route, schema, runtime, configuration or deployment
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
blockers:
  - none
next_action: open the architecture PR, review the exact four-path diff, run required exact-head documentation/governance checks and merge only if review remains clean
```

## Notes

External websites are research evidence only. Do not copy their code, datasets, text, assets, branding or layouts. Architecture decisions remain Oteryn-specific and grounded in accepted Platform ownership.