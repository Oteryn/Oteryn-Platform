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
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0025-player-companion-and-portal-tools-boundary.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
search_first:
  - open Platform PRs and active task ownership
  - current Tibia, RubinOT, TibiaPal and player-tool benchmark deltas
optional_reads:
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/agents/reports/OTERYN-20260806-portal-player-tools-benchmark.md
---

# OTERYN-20260808-portal-architecture-delta

## Goal

Re-evaluate the WWW portal architecture against current Platform `main` and current public MMORPG/player-tool patterns, then persist only material bounded refinements that fit accepted Platform ownership. Do not redesign the sound Laravel modular-monolith foundation and do not access or modify server/game repositories.

## Acceptance criteria

- [x] Reconcile the current portal-completeness and PlayerCompanion architecture with the 2026-08-08 benchmark delta.
- [x] Distinguish genuinely missing architectural ownership from already-covered or intentionally deferred capabilities.
- [x] Define a bounded first-party `Today`/command-centre composition without making the homepage a new source of truth.
- [x] Clarify player tracking/routine/watch semantics inside the accepted PlayerCompanion boundary while keeping Notifications delivery-only.
- [x] Clarify stable server-specific system definition versus editorial explanation versus live operational state ownership.
- [x] Preserve explicit world/profile/ruleset/season/version/freshness dimensions and avoid irreversible single-world assumptions.
- [x] Do not create a new microservice/module when an accepted owner already exists.
- [x] Record durable allocations that outlive this task in an ADR and reconcile the canonical module catalog.
- [x] Do not expand into Oteryn-v2, Canary, runtime, production, payment or protected-environment work.
- [ ] Complete final exact-head self-review, independent Codex review, required CI, review-thread cleanup, squash merge and lifecycle archive closeout.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-portal-architecture-delta.md
  - docs/agents/reports/OTERYN-20260808-portal-product-delta.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
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

Open PR #338 owns Game Catalog consumer implementation paths and does not overlap these architecture/documentation paths. Open PR #541 owns only the existing public-domain repair task record and does not overlap this task.

## Delivered architecture delta

The 2026-08-08 evidence strengthened four product patterns without invalidating ADR 0025 or the current module split:

1. `PublicPortal` may compose a `Today`/command-centre view from bounded sources, but owns no underlying runtime/game/editorial truth.
2. Owner-private tracking, routines, watch preferences and derived progress/change signals belong to `PlayerCompanion.ProgressTracker`; `Notifications` remains delivery-only.
3. Structured server-specific system definitions belong under `GameCatalog`, while `Wiki` explains them and `LiveOps` owns current schedule/rotation/runtime state.
4. A future World Hub is a public composition of configured world presentation, `PublicGameData`, `LiveOps` and optional evidence-backed analytics, never routing/admission authority.

Community-submitted hunt evidence remains P2/discovery because provenance, sampling bias, manipulation, privacy and moderation require a separate contract.

ADR 0032 records these durable sub-boundary decisions and explicitly extends rather than supersedes ADR 0025. `MODULE_CATALOG.md` is reconciled so future work can discover the same ownership directly from the canonical module inventory.

## Review findings and repair

Codex review on exact head `508139a83bef1d00700636d490233ddeccc2ba2c` produced three material findings:

1. **P1 — durable ADR missing:** responsibility allocations intended to outlive one task were not in an ADR.
2. **P1 — self-review gate stale:** the task record allowed merge while its recorded self-review covered an older head.
3. **P2 — module catalog drift:** canonical `MODULE_CATALOG.md` did not expose the new responsibilities.

Repair cycle 1 is complete in content generation `6464cc299a00c13b4a55bee6a37c093caa42d9a3` plus this final checkpoint commit:

- ADR 0032 added and registered;
- `MODULE_CATALOG.md` synchronized across PublicPortal, GameCatalog, PlayerCompanion, Notifications, Wiki and LiveOps responsibilities/invariants;
- benchmark report now references ADR 0032;
- stale task-level self-review/merge assumption removed; final self-review must be a PR review anchored to the live final head.

Historical self-reviews and CI on earlier heads are supporting evidence only and cannot satisfy the final merge gate.

## Final exact-head self-review gate

A task file cannot embed the SHA of the commit that contains its own changed bytes without changing that SHA again. Therefore the final exact-head self-review is recorded as a PR review anchored to the live final PR head after this checkpoint commit. The live PR head plus that anchored PR review are authoritative for this gate.

Before merge the PR review must record:

```yaml
self_review:
  result: PASS
  exact_head: <live final PR head>
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
```

No prior review on `c13d490...`, `508139a...` or the pre-checkpoint content generation may be reused as the final PASS.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-09T00:16:00+02:00
invocation_started_at: 2026-08-09T00:06:00+02:00
last_progress_at: 2026-08-09T00:16:00+02:00
head: UNKNOWN
branch: docs/OTERYN-20260808-portal-architecture-delta
pr: 933
status: validating
phase: exact-head-validation
session_id: agent-20260809-0006-portal-architecture-delta
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
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one cohesive documentation-only architecture repair; no runtime or cross-repository ownership
proven:
  - Platform main was 3c6f5192cdcd6bfc999ae0a8c731b88182c65bf4 at task start
  - WWW-only repository scope is mandatory by trusted main governance
  - current architecture retains Laravel modular monolith and Blade baseline
  - ADR 0025 remains the broad PlayerCompanion/LiveOps decision
  - ADR 0032 is the next available ADR after scanning the main inventory and open PRs
  - Codex exact-head review on 508139a identified the three recorded findings
  - repair content generation 6464cc299a00c13b4a55bee6a37c093caa42d9a3 contains ADR/module/report repairs
  - open PR #338 and #541 do not overlap the repaired architecture paths
derived:
  - final self-review must be anchored out-of-band to the live PR head after this checkpoint commit
unknown:
  - live final PR head produced by this checkpoint commit until re-read
  - final independent Codex verdict on that head
  - final required CI verdict on that head
conflicts: []
first_failure:
  marker: codex-review-508139a
  evidence: P1 durable ADR missing; P1 stale self-review gate; P2 MODULE_CATALOG drift
rejected_hypotheses:
  - portal requires architectural rewrite
  - player tracking requires a new standalone microservice
  - Notifications should own tracking/subscription rules
  - server-specific systems require a generic plugin module
  - focused canonical docs alone are sufficient durable authority for these new allocations
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-portal-architecture-delta.md
  - docs/agents/reports/OTERYN-20260808-portal-product-delta.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
validation:
  - command: focused architecture ownership reconciliation
    result: PASS
    evidence: repair converts all three Codex findings into ADR/module/task authority without changing runtime scope
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only architecture package changes no executable route, schema, runtime, configuration or deployment
ci_checks_for_current_head: 0
ci_check_generation: final_checkpoint
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 0
stall_warnings: 0
blockers:
  - none
next_action: re-read live PR #933 head, perform and record exact-head full-diff self-review, resolve the three addressed review threads, request fresh Codex review, and require exact-head CI before squash merge
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 3
  session_id: agent-20260809-0006-portal-architecture-delta
  session_started_at: 2026-08-09T00:06:00+02:00
  checkpointed_at: 2026-08-09T00:16:00+02:00
  last_progress_at: 2026-08-09T00:16:00+02:00
  phase: exact-head-validation
  exact_head: UNKNOWN
  pull_request: 933
  active_operation: final exact-head self-review, fresh Codex review and required CI
  external_run_ids: []
  operation_started_at: 2026-08-09T00:16:00+02:00
  wait_deadline_at: null
  check_generation: final_checkpoint
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: live PR #933 head is readable and unchanged during exact-head validation
  next_action: re-read PR #933 head and execute the exact-head validation gates without changing repository files
```

## Notes

External websites are research evidence only. Do not copy their code, datasets, text, assets, branding or layouts. Architecture decisions remain Oteryn-specific and grounded in accepted Platform ownership.
