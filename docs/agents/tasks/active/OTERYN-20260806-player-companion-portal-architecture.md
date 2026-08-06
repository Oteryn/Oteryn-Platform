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

- [x] External benchmark evidence is recorded as dated research and not treated as implementation authority.
- [x] A focused canonical `PlayerCompanion` architecture defines ownership, dependencies, data/versioning, privacy, deterministic calculations, recommendations and delivery priorities.
- [x] An accepted ADR records the durable decision to add `PlayerCompanion` without replacing the modular monolith or duplicating Game Catalog, Wiki, PublicGameData or Game Analytics ownership.
- [x] `MODULE_CATALOG.md` reconciles implemented `PublicPortal`, `Announcements`, `Downloads` and `Events` boundaries and adds planned `PlayerCompanion` and `LiveOps` boundaries.
- [x] System architecture, architecture authority, repository navigation and README consistently reference the new focused portal-completeness and Player Companion architecture.
- [x] The documented launch and post-launch inventory records implement/defer/reject decision points instead of implying every Tibia/RubinOT/TibiaPal feature is mandatory.
- [x] Documentation-only validation and fresh contradiction/diff audit pass on the validated architecture head; the final checkpoint-only head remains subject to the protected exact-head merge gate.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260806-player-companion-portal-architecture.md
  - docs/agents/reports/OTERYN-20260806-portal-player-tools-benchmark.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
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
updated_at: 2026-08-06T08:05:29Z
invocation_started_at: 2026-08-06T07:32:00Z
last_progress_at: 2026-08-06T08:05:29Z
head: 8cd10caab3fce4cdd55503bdcb18e022d0053786
branch: task/OTERYN-20260806-player-companion-portal-architecture
pr: 667
status: ready
phase: merge
session_id: chatgpt-20260806-player-companion-architecture
session_role: architect
execution_mode: github
execution_reason: documentation-only architecture package was authored and validated through the repository connection
context_routes:
  - architecture
  - content
  - public-portal
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260806-player-companion-portal-architecture.md
  - docs/agents/reports/OTERYN-20260806-portal-player-tools-benchmark.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/architecture/adr/0025-player-companion-and-portal-tools-boundary.md
  - docs/architecture/adr/README.md
  - README.md
proven:
  - task-start main was 47c6caa6b35c2d2af08d06322c6911721370860d
  - no open pull request or branch matching Player Companion architecture existed at claim time
  - current code contains separate PublicPortal, Downloads, Events and Announcements route modules
  - the former module table did not list those four boundaries
  - TibiaPal exposes player calculators, planning, hunt-reference and session-analysis capabilities
  - Tibia and RubinOT expose broader portal/community information architectures than the launch core
  - ADR 0025 and focused portal/PlayerCompanion architecture persist the owner-approved direction
  - module catalogue reconciles implemented portal boundaries and planned PlayerCompanion/LiveOps boundaries
  - all changes are documentation-only and do not mutate runtime, database, routes, deployment, production or external repositories
  - exact architecture head 8cd10caab3fce4cdd55503bdcb18e022d0053786 passed all eight emitted workflow runs
  - fresh full-diff and focused-file review found no runtime scope drift, removed canonical boundary or unresolved material contradiction
  - PR #667 has zero unresolved review threads
derived:
  - the modular-monolith foundation should be retained
  - player-useful tools require a dedicated orchestration and personalization boundary rather than being placed in Wiki or CMS
  - complete portal architecture remains an explicit release-scope gate rather than a universal claim
unknown:
  - exact implementation order and product activation of every P0/P1/P2 Player Companion capability remain future task decisions
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - replace the portal with a separate SPA, WordPress or default microservices architecture
  - treat every benchmark capability as mandatory for launch
changed_paths:
  - README.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/reports/OTERYN-20260806-portal-player-tools-benchmark.md
  - docs/agents/tasks/active/OTERYN-20260806-player-companion-portal-architecture.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/adr/0025-player-companion-and-portal-tools-boundary.md
  - docs/architecture/adr/README.md
validation:
  - command: GitHub workflow aggregate on 8cd10caab3fce4cdd55503bdcb18e022d0053786
    result: PASS
    evidence: Agent Governance 31082991740; Edge Security Emulation 31082991375; Native protocol contract 31082991514; Native protocol contract audits 31082991526; Game Auth Ticket Concurrency 31082991505; CI 31082991427; Platform DB Outage Validation 31082991482; Phase 7 Production-Like Validation 31082991418
  - command: full PR changed-file and diff review plus focused canonical document review
    result: PASS
    evidence: exactly 11 declared documentation paths; no executable/runtime/schema/config/deployment or external-repository change; zero unresolved review threads
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only architecture package changes no executable behavior, route, schema, configuration or deployment; broad repository workflows nevertheless passed on the architecture head
ci_checks_for_current_head: 8
ci_check_generation: validated-architecture-head
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 1
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
next_action: require all protected checks on the final checkpoint-only PR head and squash-merge PR #667 without bypass
```

## Notes

External websites are research evidence only. Their code, text, layouts, artwork, icons and proprietary data must not be copied into Oteryn.
