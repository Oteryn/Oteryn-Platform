---
task_id: OTERYN-20260806-player-companion-portal-architecture
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-content
task_kind: architecture
implementation_authorized: true
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/ROADMAP.md
search_first:
  - overlapping portal, player-tools, PlayerCompanion, TibiaPal or public-site architecture work
  - current module catalogue and route implementation drift
  - current Tibia, RubinOT and TibiaPal public capability evidence
optional_reads:
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260806-player-companion-portal-architecture

## Goal

Persist the repository owner's complete portal-architecture review from this conversation: retain the Laravel modular-monolith foundation, reconcile implemented portal modules with canonical architecture, compare the product surface with Tibia, RubinOT and TibiaPal, and adopt a bounded `PlayerCompanion` architecture for player planning, calculators, hunt guidance, session analysis and progression tracking.

## Delivery classification

```yaml
feature_scope:
  type: documentation
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: false
  e2e_required: false
```

## Acceptance criteria

- [ ] External benchmark evidence is recorded as dated research and not treated as implementation authority.
- [ ] A focused canonical `PlayerCompanion` architecture defines ownership, dependencies, data/versioning, privacy, deterministic calculations, recommendations and delivery priorities.
- [ ] An accepted ADR records the durable decision to add `PlayerCompanion` without replacing the modular monolith or duplicating Game Catalog, Wiki, PublicGameData or Game Analytics ownership.
- [ ] `MODULE_CATALOG.md` reconciles implemented `PublicPortal`, `Announcements`, `Downloads` and `Events` boundaries and adds planned `PlayerCompanion` and `LiveOps` boundaries.
- [ ] System architecture, architecture authority, public-site plan and roadmap consistently reference the new boundary and remaining portal-completeness work.
- [ ] The documented launch and post-launch inventory records implement/defer/reject decision points instead of implying every Tibia/RubinOT feature is mandatory.
- [ ] Documentation-only validation and fresh contradiction/diff audit pass on the exact final head.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260806-player-companion-portal-architecture.md
  - docs/agents/reports/OTERYN-20260806-portal-player-tools-benchmark.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/ROADMAP.md
  - docs/architecture/adr/0025-player-companion-and-portal-tools-boundary.md
  - docs/architecture/adr/README.md
  - README.md
modules:
  - PublicPortal
  - Announcements
  - Downloads
  - Events
  - PlayerCompanion
  - LiveOps
  - GameCatalog
  - Wiki
  - PublicGameData
dependencies:
  - docs/architecture/adr/0001-laravel-modular-monolith.md
  - docs/architecture/adr/0008-oteryn-frontend-information-and-shell-architecture.md
  - docs/architecture/adr/0016-versioned-game-catalog-snapshots.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-06T07:32:00Z
invocation_started_at: 2026-08-06T07:32:00Z
last_progress_at: 2026-08-06T07:32:00Z
head: 47c6caa6b35c2d2af08d06322c6911721370860d
branch: task/OTERYN-20260806-player-companion-portal-architecture
pr: none
status: implementing
phase: design
session_id: chatgpt-20260806-player-companion-architecture
session_role: architect
execution_mode: github
execution_reason: documentation-only architecture package can be completed and validated through the repository connection
context_routes:
  - architecture
  - content
  - public-portal
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260806-player-companion-portal-architecture.md
  - docs/agents/reports/OTERYN-20260806-portal-player-tools-benchmark.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/ROADMAP.md
  - docs/architecture/adr/0025-player-companion-and-portal-tools-boundary.md
  - docs/architecture/adr/README.md
  - README.md
proven:
  - main at task start is 47c6caa6b35c2d2af08d06322c6911721370860d
  - no open pull request or branch matching player-companion architecture was found
  - current code contains separate PublicPortal, Downloads, Events and Announcements route modules
  - current module table does not list those four boundaries
  - TibiaPal currently exposes player calculators, planning, hunt-reference and session-analysis capabilities
  - Tibia and RubinOT expose broader portal/community information architectures than the current launch core
derived:
  - the modular-monolith foundation should be retained
  - player-useful tools require a dedicated orchestration and personalization boundary rather than being placed in Wiki or CMS
unknown:
  - exact implementation order and product activation of every P1/P2 companion capability remain future task decisions
conflicts:
  - canonical module inventory currently lags implemented portal route boundaries
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - replace the portal with a separate SPA, WordPress or default microservices architecture
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260806-player-companion-portal-architecture.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: architecture package is being authored
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one cohesive documentation package with shared architecture authority and no runtime implementation
blockers:
  - none
next_action: create the draft PR, then author the benchmark, ADR and focused canonical architecture before reconciling indexes and roadmap
```

## Notes

External websites are research evidence only. Their code, text, layouts, artwork, icons and proprietary data must not be copied into Oteryn.