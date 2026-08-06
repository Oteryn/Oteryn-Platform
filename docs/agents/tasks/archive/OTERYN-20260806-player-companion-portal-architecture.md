---
task_id: OTERYN-20260806-player-companion-portal-architecture
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-content
task_kind: architecture
implementation_authorized: true
status: completed
completed_at: 2026-08-06T08:17:13Z
merged_pull_request: 667
merge_commit: 3f25d3951f326d2adffa2eb75855a7817d7cdcc4
---

# OTERYN-20260806-player-companion-portal-architecture

## Goal

Persist the repository owner's complete portal-architecture review: retain the Laravel modular-monolith foundation, reconcile implemented portal modules with canonical architecture, compare Oteryn with Tibia, RubinOT and TibiaPal, and adopt a bounded `PlayerCompanion` architecture for player planning, calculators, hunt guidance, session analysis and progression tracking.

## Terminal result

`DONE`

PR #667 was squash-merged to protected `main` as `3f25d3951f326d2adffa2eb75855a7817d7cdcc4` after all required checks passed on exact head `3810a32855c5c3e66a54f14ca67fa7cc4ce7c819` and zero review threads remained unresolved.

## Delivered architecture

- dated benchmark of Oteryn, Tibia, RubinOT and TibiaPal;
- accepted ADR 0025 for a dedicated `PlayerCompanion` boundary;
- focused `PLAYER_COMPANION_ARCHITECTURE.md` covering calculators, build planning, hunt guidance, private session analysis, progression tracking, recommendations, versioning, privacy and API/client reuse;
- focused `PORTAL_COMPLETENESS_ARCHITECTURE.md` with current assessment, remaining gaps, implement/defer/reject baseline and release completion gate;
- canonical module reconciliation for implemented `PublicPortal`, `Announcements`, `Events` and `Downloads`;
- planned `LiveOps`, `PlayerCompanion` and concrete-consumer-driven `PlatformAPI` boundaries;
- system architecture, architecture authority, repository map, ADR registry and README synchronization.

## Accepted decisions

- retain the Laravel modular monolith and server-rendered Blade baseline;
- do not replace the portal with WordPress, a separate SPA or default microservices;
- keep `PlayerCompanion` separate from `Wiki`, `CMS`, `GameCatalog`, `PublicGameData`, future `GameAnalytics` and `LiveOps` truth ownership;
- classify outputs as `DETERMINISTIC`, `SIMULATION` or `RECOMMENDATION`;
- bind calculations and saved plans to game profile, ruleset, catalogue snapshot, formula version, world/season and effective dates;
- keep saved plans, goals and session analysis private by default;
- implement formulas once in reusable versioned domain services;
- prioritize loot/session analysis, hunt discovery, equipment/build planning, charm/perk/proficiency planning, quest/access tracking and EXP/training tools;
- treat external portals as product research only and copy no code, text, branding, layouts, icons, artwork or proprietary data.

## Nonclaims

- `PlayerCompanion` and `LiveOps` remain `PLANNED` rather than implemented;
- the package changed no runtime, route, schema, deployment, production state, Canary/Otheryn or external repository;
- architecture acceptance does not prove product completeness or production readiness;
- Issues #365, #488, #489 and #490 remain explicit evidence routes for open content, failure-path and environment gaps.

## Validation

```yaml
exact_head: 3810a32855c5c3e66a54f14ca67fa7cc4ce7c819
required_checks:
  Agent Governance: success
  Edge Security Emulation: success
  Native protocol contract: success
  Native protocol contract audits: success
  Game Auth Ticket Concurrency: success
  CI: success
  Platform DB Outage Validation: success
  Phase 7 Production-Like Validation: success
review_threads_unresolved: 0
changed_paths: 11
runtime_e2e:
  result: NOT_APPLICABLE
  reason: documentation-only architecture package with no executable, schema, route, configuration or deployment change
```

## Ownership release

All task-owned paths are released by this archive closeout. Future implementation must use separate bounded tasks and claims for each selected PlayerCompanion, LiveOps, Platform API or portal-completeness slice.

## Final checkpoint

```yaml
checkpoint_version: 1
status: completed
phase: archived
branch: task/OTERYN-20260806-player-companion-portal-architecture
pull_request: 667
merge_commit: 3f25d3951f326d2adffa2eb75855a7817d7cdcc4
blockers: []
conflicts: []
next_action: select one separately authorized PlayerCompanion or portal-completeness vertical slice when implementation is requested
```
