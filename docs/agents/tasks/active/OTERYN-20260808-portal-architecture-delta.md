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
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
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

Re-evaluate the WWW portal architecture against current Platform `main` and current public MMORPG/player-tool patterns, then persist only material bounded refinements that fit accepted Platform ownership. Do not redesign the Laravel modular-monolith foundation and do not access or modify server/game repositories.

## Acceptance criteria

- [x] Reconcile the current portal-completeness and PlayerCompanion architecture with the 2026-08-08 benchmark delta.
- [x] Distinguish genuinely missing architectural ownership from already-covered or intentionally deferred capabilities.
- [x] Define a bounded first-party `Today`/command-centre composition without making the homepage a new source of truth.
- [x] Clarify player tracking/routine/watch semantics inside the accepted PlayerCompanion boundary while keeping Notifications delivery-only.
- [x] Clarify stable server-specific system definition versus editorial explanation versus live operational state ownership.
- [x] Preserve explicit world/profile/ruleset/season/version/freshness dimensions and avoid irreversible single-world assumptions.
- [x] Do not create a new microservice/module when an accepted owner already exists.
- [x] Record durable allocations that outlive this task in an ADR and reconcile the canonical module catalog.
- [x] Apply the repository remediation risk gate after material review-driven architecture repair.
- [x] Do not expand into Oteryn-v2, Canary, runtime, production, payment or protected-environment work.
- [ ] Complete final exact-head self-review, fresh Codex review, required exact-head CI, zero unresolved threads, squash merge and lifecycle archive closeout.

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
  - ADR 0032
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

The refreshed benchmark strengthened four product patterns without invalidating ADR 0025 or the current module split:

1. `PublicPortal` may compose a `Today`/command-centre view from bounded sources, but owns no underlying runtime/game/editorial truth.
2. Owner-private tracking, routines, watch preferences and derived progress/change signals belong to `PlayerCompanion.ProgressTracker`; `Notifications` remains delivery-only.
3. Structured server-specific system definitions belong under `GameCatalog`, while `Wiki` explains them and `LiveOps` owns current schedule/rotation/runtime state.
4. A future World Hub is a public composition of configured world presentation, `PublicGameData`, `LiveOps` and optional evidence-backed analytics, never routing/admission authority.

Community-submitted hunt evidence remains P2/discovery because provenance, sampling bias, manipulation, privacy and moderation require a separate contract.

ADR 0032 records these durable sub-boundary decisions and explicitly extends rather than supersedes ADR 0025. `MODULE_CATALOG.md` is reconciled so future work can discover the same ownership directly from the canonical module inventory.

## Review findings and repair history

Codex review on exact head `508139a83bef1d00700636d490233ddeccc2ba2c` produced three material findings: missing durable ADR authority, stale self-review gating and canonical module-catalog drift. Repair cycle 1 addressed all three with ADR 0032, ADR registry synchronization, module-catalog reconciliation and exact-head PR-review evidence.

Fresh Codex review on exact head `1b606eaea0cdb102639d87c80c23e6007a6790e3` produced one additional **P1 governance finding**: this review-driven durable-architecture repair requires a version-2 remediation validation gate with `HEIGHTENED` intensity and complete evidence. Repair cycle 2 added that gate.

Exact-head CI on `892893de2303eabd6fc6376a3c5d161e5841fb1f` then found a checkpoint-schema defect introduced while compacting this task record: `Context checkpoint` lacked `owned_paths` and `rejected_hypotheses`. This repair cycle 3 restores those mandatory fields. The downstream Control Room/liveness failures were consequences of the same checkpoint validation failure, not independent product defects.

No repair cycle changes runtime, route, schema, configuration, deployment, payment state or any external repository.

## Remediation validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: implementation owner
  classified_at: 2026-08-09T00:26:00+02:00
  risk: medium
  triggers:
    - durable architecture
    - public module boundaries and data ownership
    - cross-module dependency direction
    - material review-driven repair cycle
  unknown_or_conflict: []
  rationale: >-
    The change is documentation-only and reversible, but it allocates durable Platform
    responsibilities across PublicPortal, PlayerCompanion, Notifications, GameCatalog,
    Wiki and LiveOps. Repository policy therefore requires HEIGHTENED validation even
    though there is no runtime/schema/production mutation.
  self_review:
    result: PASS
    exact_head: none
    evidence:
      - exact final PR head is reviewed out-of-band in an anchored PR review after the final task-record commit
      - all seven changed paths must be included in that exact-head full-diff review
      - acceptance criteria, all current review/CI findings, negative paths, rollback, compatibility and related PR ownership are required review dimensions
      - task-file exact_head is intentionally none because embedding the SHA of the commit containing this field would move the SHA again; the anchored PR review is the authoritative exact-head evidence
```

### Heightened evidence requirements

- **Focused regression / architecture evidence:** ADR 0032 and `MODULE_CATALOG.md` express the same ownership split; focused portal architecture documents remain compatible with them.
- **Negative paths:** `PublicPortal` does not become runtime/game-data/routing authority; `Notifications` does not own tracking rules; `Wiki` does not become deterministic/live truth; `GameCatalog` does not fabricate unsupported systems; stale/unavailable source state does not become fabricated normal state.
- **Rollback:** documentation-only squash revert is bounded; no data/schema/runtime rollback is required.
- **Compatibility:** ADR 0032 extends ADR 0025 without superseding it; no Oteryn-v2/Canary contract or runtime behavior changes.
- **Related PRs:** #338 and #541 have no overlapping owned paths for this package.
- **E2E:** `NOT_APPLICABLE` because there is no executable route, schema, configuration, runtime or deployment change.
- **Final CI:** every required GitHub check must pass on the exact final head after this checkpoint repair.
- **Review hygiene:** zero unresolved material review threads and zero requested changes before merge.

## Exact-head self-review mechanism

The repository requires self-review on the exact final head. A task file cannot embed the SHA of the commit containing that SHA without producing a new head. Therefore the task record contains the full risk/evidence contract, while the implementation owner records PASS as a PR review anchored to the exact live head after the last repository-file commit. No repository file changes are permitted after that review unless the gate is recomputed and exact-head validation is repeated.

Required anchored review shape:

```yaml
self_review:
  result: PASS
  exact_head: <live final PR head>
  acceptance_checked: true
  full_diff_checked: true
  related_prs_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  findings: []
  evidence:
    - seven-path full PR diff
    - ADR 0032 plus ADR inventory
    - canonical MODULE_CATALOG reconciliation
    - all current Codex and CI findings and their repairs
    - checkpoint validator schema and required exact-head CI
```

Historical self-reviews and CI on earlier heads are supporting evidence only and cannot satisfy the final merge gate.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-09T00:31:00+02:00
invocation_started_at: 2026-08-09T00:26:00+02:00
last_progress_at: 2026-08-09T00:31:00+02:00
head: OUT_OF_BAND_FINAL_HEAD_AFTER_THIS_COMMIT
branch: docs/OTERYN-20260808-portal-architecture-delta
pr: 933
status: validating
phase: heightened-exact-head-validation
session_id: agent-20260809-0026-portal-architecture-delta
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
context_pressure: low
context_growth: stable
estimate_confidence: high
decomposition_decision: single
proven:
  - repository scope remains WWW Platform documentation/architecture only
  - ADR 0032 is registered and reconciled with MODULE_CATALOG
  - Codex findings from 508139a were repaired
  - Codex finding from 1b606ea was repaired by HEIGHTENED v2 gate
  - CI failure on 892893de was caused by missing required checkpoint fields owned_paths and rejected_hypotheses
  - runtime E2E is NOT_APPLICABLE for this documentation-only package
derived:
  - final exact-head SHA and PASS evidence must be recorded in the PR review after this commit
unknown: []
conflicts: []
first_failure:
  marker: checkpoint-validator-892893de
  evidence: missing checkpoint fields owned_paths and rejected_hypotheses; downstream Control Room/liveness failures are consequent
rejected_hypotheses:
  - portal requires architectural rewrite
  - player tracking requires a new standalone microservice
  - Notifications should own tracking/subscription rules
  - server-specific systems require a generic plugin module
  - focused canonical docs alone are sufficient durable authority for the new allocations
  - CI failure on 892893de represents a runtime or product defect
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
    evidence: ADR 0032, MODULE_CATALOG and focused architecture documents preserve the same ownership direction
  - command: heightened remediation risk classification
    result: PASS
    evidence: version 2 HEIGHTENED gate recorded with bounded risk/triggers/unknown/conflict/rationale/self-review evidence
  - command: checkpoint validator on 892893de
    result: FAIL_REPAIRED
    evidence: exact missing fields identified in workflow 31281742855 and restored in this task record
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only package changes no executable route, schema, runtime, configuration or deployment
repair_cycles_for_current_gate: 3
blockers:
  - none
next_action: perform exact-head full-diff self-review on the live PR head, request fresh Codex review, verify required exact-head CI and merge only with zero material findings and zero unresolved threads
```

## Notes

External websites are research evidence only. Do not copy their code, datasets, text, assets, branding or layouts. Architecture decisions remain Oteryn-specific and grounded in accepted Platform ownership.
