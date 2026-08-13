# Oteryn Portal Completion — Work Allocation Project

```yaml
project_id: OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION
programme: OTERYN_PORTAL_COMPLETION
repository: blakinio/Oteryn-Platform
trusted_base: main
status: ACTIVE
live_state_required: true
architecture_authority: docs/architecture/ARCHITECTURE_AUTHORITY.md
delivery_plan: docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
player_companion: docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
production_authority: false
codex_standing_permission: false
```

## Purpose

This document is the execution allocation for the existing `OTERYN_PORTAL_COMPLETION` programme. It converts the accepted portal architecture, the Tibia/RubinOT/TibiaPal benchmark decisions and the current audit backlog into one ordered project with explicit execution ownership.

It is not a second architecture authority and it does not replace accepted ADRs, focused architecture, contracts, live Issues, active task records or exact PR/CI state. Live repository state always wins over dated status examples below.

## Execution roles

### `CHATGPT_ARCHITECT`

Primary owner for:

- architecture and product decomposition;
- dependency and sequencing decisions;
- source-of-truth reconciliation;
- security/privacy/contract review;
- Platform API and multi-world contract decisions;
- production/public-edge evidence interpretation;
- PR review, CI diagnosis and final integration/closeout;
- deciding whether a proposed Codex task is sufficiently bounded and safe.

This role may also implement work directly when that is the smallest safe path. Delegation is optional, not required.

### `CODEX_ELIGIBLE_WORKER`

Suitable for bounded implementation work with explicit acceptance criteria, owned paths and tests, for example:

- machine-readable inventories and validators;
- isolated module vertical slices;
- deterministic calculators/parsers;
- route/controller/view/test implementation inside an accepted boundary;
- focused refactors with known contracts;
- reproducible bug fixes with exact evidence.

**Eligibility is not authorization.** No Codex/OpenAI API or other owner-funded AI invocation is allowed unless the repository owner explicitly approves that exact use/task. Prior approval never becomes standing permission. When permission is absent, `CHATGPT_ARCHITECT` or another non-owner-funded worker may execute the task instead.

### `OWNER_DECISION`

The repository owner is required only for product/business choices or authorities that cannot be inferred safely, including:

- forum vs Discord if/when a durable community product is desired;
- payment/provider/legal activation;
- explicit production/protected-environment authorization;
- external-repository mutation authority;
- explicit permission for a particular Codex/owner-funded AI invocation.

### `PROTECTED_ENV_OPERATOR`

Executes authorized production/protected-environment checks or mutations that cannot be proven from repository state alone. This role must operate only after exact owner/protected-environment authorization and must preserve environment identity and rollback evidence.

## Current project board

Status meanings:

- `DONE` — accepted scope is terminal for this project item;
- `IN_PROGRESS` — active implementation exists and owns the item;
- `ARCHITECTURE_READY` — boundary/decision exists, implementation remains;
- `OPEN` — implementation or evidence remains and is not currently proven complete;
- `BLOCKED` — exact external/authority dependency prevents progress;
- `DECISION_REQUIRED` — owner/product decision is required before implementation;
- `DEFERRED` — intentionally later, with no current implementation requirement.

| Priority | Workstream | Current status | Primary execution role | Codex suitability | Dependencies / next terminal outcome |
|---|---|---|---|---|---|
| P0 | Canonical module catalogue | `DONE` | `CHATGPT_ARCHITECT` | No | Keep `MODULE_CATALOG.md` synchronized when boundaries change. |
| P0 | Wiki expected-content inventory | `DONE` | `CHATGPT_ARCHITECT` | No | Issue #488 terminal; preserve validators and provenance gates. |
| P0 | Game Catalog authoritative inventory | `OPEN` | `CODEX_ELIGIBLE_WORKER` with `CHATGPT_ARCHITECT` review | **High** | Issue #489. Build versioned inventory for entities, fields, relations, localization, media, profile/ruleset/season applicability, search/detail/admin coverage and exact reconciliation. |
| P0 | Wiki/media 500 + publication-flash investigation | `DONE` | `CHATGPT_ARCHITECT` | No | Issue #365 terminal; preserve evidence, do not reopen without new reproduction. |
| P0/P1 | Production topology / public edge proof | `OPEN` | `CHATGPT_ARCHITECT` + `PROTECTED_ENV_OPERATOR` | Low for live proof | Issue #490. Repository-safe topology/contract work may proceed; production DNS/TLS/HSTS/WAF/private-origin/backup/restore/rollback/metrics/alerts require explicit protected-environment authority. |
| P1 | LiveOps boundary | `ARCHITECTURE_READY` | `CHATGPT_ARCHITECT` decomposition, then `CODEX_ELIGIBLE_WORKER` per slice | **High when sliced** | Implement separately: WorldStatus → Maintenance/ServerSave → RaidSchedule → runtime event/boost projections. Never fabricate stale/unavailable state. |
| P1 | PublicGameData / community expansion | `OPEN` | `CODEX_ELIGIBLE_WORKER` with architecture review | **High** | Existing characters/guilds/highscores/online/status stay source. Add houses, kill statistics, richer leaderboards, guild wars and activity history only as bounded read-model slices. Writes need separate operation contracts. |
| P1 | Client Distribution hardening | `OPEN` | `CHATGPT_ARCHITECT` trust/signature contract, then `CODEX_ELIGIBLE_WORKER` | **High after contract** | Extend Downloads with immutable/signed update manifest, stable/beta channels, minimum supported version, mandatory-update semantics and withdrawal/revocation. Trust root/signature format must be decided before implementation. |
| P1 | Platform API | `ARCHITECTURE_READY` / `DECISION_REQUIRED` for first public surface | `CHATGPT_ARCHITECT` | Medium after decision | Decide first consumers, versioning, authentication, rate limits and compatibility. API remains an adapter over module application services, never a duplicate domain layer. Issue #490 retains audit ownership until implemented/deferred/rejected. |
| P1 | Federated Search & Discovery | `ARCHITECTURE_READY` | `CODEX_ELIGIBLE_WORKER` after dependency cleanup | **High** | First remove Announcements/Events reverse `PublicPortal` dependency; then implement source-owned providers and PublicPortal orchestration. Exclude private modules and preserve partial-failure semantics. |
| P1 | Multi-world / rulesets / seasons contract | `OPEN` | `CHATGPT_ARCHITECT` | Low initially | Make world/profile/ruleset/season dimensions explicit for URLs, cache keys, projections, events, LiveOps, analytics and PlayerCompanion applicability before broad multi-world rollout. |
| P2 | PlayerCompanion foundation | `IN_PROGRESS` | current implementation owner + `CHATGPT_ARCHITECT` integration | No handoff for active slice | PR #1028 owns Hunt Session Analyzer v1. Do not duplicate or hand it to another worker mid-flight. |
| P2 | PlayerCompanion follow-up tools | `OPEN` | `CODEX_ELIGIBLE_WORKER` per independent vertical slice | **High** | Recommended queue after #1028: EXP/training calculator → Equipment Explorer → Hunt Finder → build/proficiency planner → quest/access tracker. Every tool carries version/applicability/privacy semantics. |
| P2 | Forum vs Discord | `DEFERRED` | `OWNER_DECISION` | No | Default product direction remains Discord + existing Support; build an owned forum only when durable discussion/moderation need is proven. |
| P2 | Read scaling / projections / search index | `DEFERRED` | `CHATGPT_ARCHITECT` | Medium after telemetry | Keep modular-monolith direct/read-model approach until measured thresholds justify read DB, projections or dedicated search infrastructure. No premature service split. |
| P3 | World Hub / richer community surfaces | `DEFERRED` | `CHATGPT_ARCHITECT` then bounded workers | Medium | Activate when multiple worlds/profiles and authoritative LiveOps/community inputs justify the composition. It never becomes routing/admission authority. |
| P3 | Commerce activation | `BLOCKED` by independent product/security/legal gates | `OWNER_DECISION` + `CHATGPT_ARCHITECT` | Only bounded post-decision tasks | Provider, legal/tax, signed webhooks, reconciliation, refunds/chargebacks, entitlement freshness and production authority must be decided independently before customer-money activation. |

## Recommended execution sequence

Live state may reorder this queue when an existing valid task/PR owns a higher-priority item. Otherwise use this dependency order:

1. **Finish the active PlayerCompanion Session Analyzer PR #1028** without adding another worker to that branch.
2. **Game Catalog authoritative inventory / Issue #489.** This is the strongest next Codex-eligible candidate because the acceptance surface is machine-checkable and bounded.
3. **Production/public-edge Issue #490 repository-safe closure work.** Separate repository proof from protected-environment evidence; do not claim production state from configuration alone.
4. **Platform API decision package and multi-world/ruleset/season contract.** Resolve durable contracts before broad new consumers depend on them.
5. **LiveOps first slices:** WorldStatus, then Maintenance/ServerSave; add raid/boost/event projections only when an authoritative producer exists.
6. **Federated Search dependency cleanup + implementation.** Remove reverse edges before provider onboarding.
7. **Client Distribution hardening** after signature/trust-root decision.
8. **PublicGameData expansions** as independent read-only vertical slices.
9. **PlayerCompanion follow-up tools** in the order that provides the most repeat player value without duplicating GameCatalog truth.
10. **World Hub, own forum and read-scaling infrastructure** only when telemetry/product evidence justifies them.
11. **Commerce activation** remains independently gated and must not be pulled forward merely to make the portal appear feature-complete.

## Delegation contract

Before assigning an item to a Codex worker, `CHATGPT_ARCHITECT` must produce a bounded task package containing:

1. exact repository and base branch;
2. exact task/Issue and owned paths;
3. accepted architecture/ADR/contracts to obey;
4. explicit in-scope and out-of-scope behavior;
5. required source-of-truth inputs and forbidden assumptions;
6. test/E2E/negative-path/rollback requirements;
7. merge and closeout gate;
8. statement that no external repository, production environment or owner-funded service is authorized beyond the exact owner grant.

Then, and only then, the owner may explicitly authorize a particular Codex invocation. If no authorization is given, the task remains valid but must be executed without consuming owner-funded Codex/OpenAI quota.

## Parallelism rules

Parallel work is allowed only across non-overlapping tasks/branches with distinct `owned_paths`.

Good parallel candidates after the active #1028 slice is terminal:

- Game Catalog inventory;
- repository-safe production/public-edge evidence preparation;
- Platform API decision analysis;
- multi-world contract analysis.

Do not parallelize multiple workers inside the same active PR or give two workers overlapping module/migration/route ownership.

## Definition of project completion

The portal completion project is terminal only when all launch-required rows are either:

- `DONE`, or
- explicitly `DEFERRED`/`REJECTED` by accepted owner/product decision with no launch dependency.

Additionally:

- Issue #489 and Issue #490 have terminal dispositions;
- at least one complete PlayerCompanion workflow is merged and validated end-to-end;
- required LiveOps/public current-state surfaces have authoritative producers and typed stale/unavailable semantics;
- the launcher/client distribution path has the accepted provenance/update contract if it is part of launch;
- release-scope search/community/API items are implemented or explicitly deferred;
- production topology, edge, observability, backup/restore and rollback claims are tied to exact authorized environment evidence;
- no launch-critical security/privacy/audit finding remains unresolved;
- every merged implementation has terminal task/PR/Issue lifecycle and released ownership.

This definition does not require forum, payments, microservices, World Hub or every optional TibiaPal-style tool unless the owner promotes those items into the launch scope.
