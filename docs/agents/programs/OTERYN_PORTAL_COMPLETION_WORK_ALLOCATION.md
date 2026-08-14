# Oteryn Portal Completion — Work Allocation Project

```yaml
project_id: OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION
programme: OTERYN_PORTAL_COMPLETION
repository: blakinio/Oteryn-Platform
trusted_base: main
status: ACTIVE
live_state_required: true
selection_authority: docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
architecture_authority: docs/architecture/ARCHITECTURE_AUTHORITY.md
delivery_plan: docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
player_companion: docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
production_authority: false
codex_standing_permission: false
execution_roles_are_model_agnostic: true
```

## Purpose

This document is the execution-allocation companion for the existing `OTERYN_PORTAL_COMPLETION` programme. It maps accepted portal workstreams to model-agnostic execution roles, dependencies, optional Codex suitability and terminal outcomes.

It is **not** a second scheduler, architecture authority or source of current ownership. `OTERYN_PORTAL_COMPLETION.md` selects what work may run next from live state. This companion is consulted only after that selection to decide how the selected work is owned and executed.

Accepted ADRs, focused architecture, contracts, specialized programmes, live Issues, active task records and exact PR/CI state remain authoritative for their scopes. Live repository state always wins over dated board examples below.

## Execution roles

### `ARCHITECTURE_COORDINATOR`

Primary owner for:

- architecture and product decomposition;
- dependency and sequencing reconciliation with the canonical programme;
- source-of-truth reconciliation;
- security/privacy/contract review;
- Platform API, multi-world and cross-module contract decisions;
- production/public-edge evidence interpretation;
- PR review, CI diagnosis and final integration/closeout;
- deciding whether a bounded implementation package is ready for an implementation owner.

This role may also implement a bounded change directly when that is the smallest safe path.

### `IMPLEMENTATION_OWNER`

Owns one bounded implementation slice from accepted scope through focused tests, exact-head self-review, required CI, E2E where applicable, PR closeout and task archival. Typical work includes:

- machine-readable inventories and validators;
- isolated module vertical slices;
- deterministic calculators/parsers;
- route/controller/view/test implementation inside an accepted boundary;
- focused refactors with known contracts;
- reproducible bug fixes with exact evidence.

The role is independent of model or tool. Chat/GitHub, a permitted runner or another permitted execution mode may be used. Codex suitability is recorded separately and never grants authorization.

### `OWNER_DECISION`

The repository owner is required only for product/business choices or authorities that cannot be inferred safely, including:

- explicit launch-scope defer/reject decisions where the accepted programme requires one;
- forum vs Discord if/when a durable owned community product is desired;
- payment/provider/legal activation;
- explicit production/protected-environment authorization;
- external/server-repository access or mutation authority;
- explicit permission for a particular owner-funded Codex/OpenAI invocation.

### `PROTECTED_ENV_OPERATOR`

Executes authorized production/protected-environment evidence collection or mutations that cannot be proven from repository state alone. This role operates only after exact owner/protected-environment authorization and must preserve environment identity, evidence and rollback boundaries.

## Codex suitability and authorization

The `Codex suitability` column is only a technical fit assessment for a bounded task package.

**Suitability is not authorization.** No Codex, OpenAI API or other owner-funded AI invocation is allowed unless the repository owner explicitly approves that exact use/task. Prior approval never becomes standing permission. When permission is absent, the selected task remains valid and must be executed through another permitted mode.

## Current project board

These are **programme workstream maturity/disposition states**, not task-checkpoint statuses and not canonical selector states. The board never emits or persists canonical `READY`; `OTERYN_PORTAL_COMPLETION.md` derives `TERMINAL | OWNED | BLOCKED | DECISION_REQUIRED | READY` from live evidence on every invocation.

Status meanings:

- `DONE` — accepted scope is terminal for this project item;
- `IN_PROGRESS` — a valid live task/PR currently owns the item at the time the row is deliberately refreshed; live ownership must still be rechecked;
- `ARCHITECTURE_READY` — an accepted architecture boundary exists and implementation remains; **this does not mean canonical selector `READY`**;
- `OPEN` — implementation, decomposition or evidence remains and is not currently proven complete;
- `BLOCKED` — a known external/authority dependency prevents the stated outcome; live selector still verifies the exact candidate;
- `DECISION_REQUIRED` — an owner/product decision is required before the stated outcome;
- `DEFERRED` — intentionally not required in the current launch scope;
- `REJECTED` — explicitly excluded by an accepted owner/product decision.

### Strict promotion invariant

A workstream row may be `ARCHITECTURE_READY` while every current implementation candidate is `BLOCKED`, `OWNED` or `DECISION_REQUIRED`. Canonical selector `READY` exists only transiently when one exact Issue/task package satisfies **all** programme eligibility gates: accepted predecessors, exact source authority, Platform-only scope, no conflicting owner, no implied protected/external mutation, bounded completion, rollback and known validation.

The reverse is also true: a documentation/architecture package inside an `OPEN` or `ARCHITECTURE_READY` workstream may be selector-`READY` even though the workstream is not implemented. Completing that package does not promote runtime availability.

| Programme priority | Workstream | Current status | Primary execution role | Codex suitability | Dependencies / next terminal outcome |
|---|---|---|---|---|---|
| P0 | Canonical module catalogue | `DONE` | `ARCHITECTURE_COORDINATOR` | No | Keep `MODULE_CATALOG.md` synchronized when accepted boundaries or implementation availability change. |
| P1 | Production topology and OperationsObservability repository evidence | `ARCHITECTURE_READY` / `BLOCKED` for direct production proof | `ARCHITECTURE_COORDINATOR` + `PROTECTED_ENV_OPERATOR` for live proof | Low | `OPERATIONS_OBSERVABILITY_ARCHITECTURE.md` and the reconciled topology baseline define the current repository/staging evidence contract. Issue #490 retains direct production topology/logging/metrics/alerts/on-call/backup/restore evidence, which requires exact protected-environment authority and identity. Repository or staging evidence must not be promoted to `PRODUCTION_PROVEN`. |
| P1 | PublicEdge protected-environment proof | `BLOCKED` for live proof | `ARCHITECTURE_COORDINATOR` + `PROTECTED_ENV_OPERATOR` | Low | Issue #490. DNS/TLS/HSTS/WAF/private-origin/deployed-identity/smoke evidence requires explicit protected-environment authority. Repository-safe preparation is not blocked. |
| P1/P2 | Core Account Center / Character Portfolio | `ARCHITECTURE_READY` | `ARCHITECTURE_COORDINATOR`, then `IMPLEMENTATION_OWNER` per vertical slice | High after contract/dependency proof | Native delete/restore and rename Issues #317/#319 remain blocked until accepted Oteryn-v2 Character Authority command/result semantics exist; world transfer #320 additionally requires an explicit product decision. Architecture maturity does not bypass either gate. |
| P2 | LiveOps | `ARCHITECTURE_READY` | `ARCHITECTURE_COORDINATOR`, then `IMPLEMENTATION_OWNER` per slice | High when sliced | `LIVEOPS_ARCHITECTURE.md` is canonical and architecture Issue #1046 is terminal. The first runtime package is `WorldStatus + configured Maintenance`, but it becomes selector-`READY` only after the exact authoritative runtime-status source required by the delivered capability is proven from permitted evidence. `ServerSave` is separately blocked until its authoritative source/applicability/time-base/recurrence/freshness contract is proven. Never fabricate stale/unavailable state. `MODULE_CATALOG.md` correctly remains `LiveOps | PLANNED` until executable capability is merged. |
| P2 | PublicPortal Today | `ARCHITECTURE_READY` | `ARCHITECTURE_COORDINATOR`, then `IMPLEMENTATION_OWNER` | High after focused architecture/source readiness | Public Today architecture/documentation may proceed independently when unowned. Runtime implementation must consume source-owned projections, preserve partial/freshness states and keep authenticated/private composition behind the full ADR 0032 owner-private cache/isolation gate. |
| P2 | Federated Search & Discovery | `ARCHITECTURE_READY` | `IMPLEMENTATION_OWNER` after coordinator confirms dependency cleanup | High | Remove Announcements/Events reverse `PublicPortal` dependency first, then implement source-owned providers and PublicPortal orchestration with partial-failure semantics. Before selector promotion, create/reuse one exact bounded Issue/package and prove no overlapping owner. |
| P2 | Client Distribution hardening | `ARCHITECTURE_READY` | `IMPLEMENTATION_OWNER` after architecture closeout | High for Platform-only slice | ADR 0035 and `CLIENT_DISTRIBUTION_ARCHITECTURE.md` accept TUF-based role-separated updater trust with signing keys outside Laravel. Issue #1039 is the explicit Platform implementation handoff and is reachable from the canonical programme order. External updater implementation, private signing infrastructure/operations and production activation remain separate authority/evidence gates and are not required for a truthful Platform-only partial-consumer claim. |
| P2 | Wiki expected-content inventory | `DONE` | `ARCHITECTURE_COORDINATOR` | No | Issue #488 terminal; preserve validators and provenance gates. |
| P2 | Game Catalog authoritative expected inventory | `OPEN` | `ARCHITECTURE_COORDINATOR`, then `IMPLEMENTATION_OWNER` for Platform-only slices | High for Platform-only bounded slices | Route through `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md`. Issue #489 is a shared audit owner; only its GameCatalog/content findings belong here. Server/game producer or authority work requires separate owner authorization before repository access. |
| P2 | Wiki/media 500 + publication-flash investigation | `DONE` | `ARCHITECTURE_COORDINATOR` | No | Issue #365 terminal; do not reopen without new reproduction. |
| P2 | Platform API | `DEFERRED` | `ARCHITECTURE_COORDINATOR` when a named consumer trigger exists | No active handoff | ADR 0036 and `PLATFORM_API_ARCHITECTURE.md` explicitly defer the general API until an approved named consumer/use case exists. Specialized game-auth/internal endpoints remain outside general PlatformAPI classification. Do not create a speculative implementation task; reactivate only through the accepted activation checklist. |
| P2 | Multi-world / rulesets / seasons contract | `OPEN` | `ARCHITECTURE_COORDINATOR` | Low initially | Preserve world/profile/ruleset/season dimensions for URLs, cache keys, projections, events, LiveOps, analytics and PlayerCompanion before broad multi-world rollout. |
| P2/P3 | PlayerCompanion foundation | `DONE` | `ARCHITECTURE_COORDINATOR` | No | PR #1028 merged Hunt Session Analyzer v1 as the first complete PlayerCompanion workflow; exact final-head E2E/CI passed and its task is terminally archived. Follow-up tools remain independent slices. |
| P2/P3 | PlayerCompanion follow-up tools | `OPEN` | `IMPLEMENTATION_OWNER` per independent vertical slice | High | After #1028 use accepted architecture order: Hunt Finder → Equipment Explorer → Character Build Planner → Charm/Perk/Proficiency Planner → Quest/Access Tracker → EXP/Training calculators → validated shareable builds. |
| P3 | PublicGameData / richer community read surfaces | `DEFERRED` by default until promoted | `ARCHITECTURE_COORDINATOR`, then `IMPLEMENTATION_OWNER` per read-only slice | High when promoted | Houses, kill statistics, richer leaderboards, guild wars/activity history are product inventory inputs, not automatic launch requirements. Writes require separate operation contracts. |
| P3 | Forum vs Discord | `DEFERRED` | `OWNER_DECISION` | No | Default direction remains Discord + existing Support; build an owned forum only when durable discussion/moderation need is proven. |
| P3 | Read scaling / projections / dedicated search index | `DEFERRED` | `ARCHITECTURE_COORDINATOR` | Medium after telemetry | Keep the modular-monolith/direct-read-model approach until measured thresholds justify a read DB, projection or search infrastructure. |
| P3 | World Hub / richer community composition | `DEFERRED` | `ARCHITECTURE_COORDINATOR`, then bounded `IMPLEMENTATION_OWNER` slices | Medium | Activate only when multiple worlds/profiles and authoritative LiveOps/community inputs justify it; it never becomes routing/admission authority. |
| P3 | Commerce capability disposition | `DECISION_REQUIRED` | `OWNER_DECISION` + `ARCHITECTURE_COORDINATOR` | Low before decision | Issue #489 also owns commerce findings. Each missing product/payment capability must be delivered, explicitly deferred or rejected; do not treat the GameCatalog inventory row as closing commerce findings. |
| P3 | Commerce production activation | `BLOCKED` by independent product/security/legal/production gates | `OWNER_DECISION` + `ARCHITECTURE_COORDINATOR` + `PROTECTED_ENV_OPERATOR` | Only bounded post-decision tasks | Provider, legal/tax, signed webhooks, reconciliation, refunds/chargebacks, entitlement freshness and production authority must all be independently satisfied. |

## Selection contract

This companion must not create a second priority algorithm.

1. `OTERYN_PORTAL_COMPLETION.md` resolves live state and classifies each canonical selection-order entry as `TERMINAL | OWNED | BLOCKED | DECISION_REQUIRED | READY`.
2. It selects the first unowned `READY` candidate and persists exact reasons for every skipped earlier entry.
3. Only after selection is the candidate mapped to the board above to choose a model-agnostic execution role and permitted execution mode.
4. A specialized programme or Issue owner named by the selected row controls its detailed decomposition and acceptance criteria.
5. Board priority labels or maturity states never authorize skipping an earlier canonical `READY` item.
6. `ARCHITECTURE_READY` never promotes itself to selector `READY`; promotion is a live programme decision and is invalidated immediately by ownership, blocker, decision or source-authority drift.
7. If live state or accepted architecture conflicts with a board row, execution stops using that row until this companion is reconciled; the live authoritative state is not overwritten to fit the table.

## Specialized routing and overlap rules

- Audit repair findings use `OTERYN_PLATFORM_REMEDIATION`; this allocation does not create duplicate repair ownership. Historical closed Issues are not candidates.
- Game Catalog implementation must reuse `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md`. Platform-only inventory/consumer work may proceed within Platform authority; server/game producer or authority work is inaccessible from a Platform invocation until the owner separately authorizes that repository access.
- Issue #489 is intentionally split by concern: GameCatalog/content findings route to the Game Catalog row; product/payment findings route to Commerce capability disposition/activation.
- Issue #490 is intentionally split by concern: the PlatformAPI slice is terminally deferred by ADR 0036; OperationsObservability repository/staging applicability is architecture-ready through `OPERATIONS_OBSERVABILITY_ARCHITECTURE.md`; PublicEdge live proof and direct production evidence remain separately blocked on protected-environment authority/evidence.
- Client Distribution Issue #1039 is explicitly routed by canonical selection order; its Platform-only implementation must not be held unreachable behind a permanently blocked external signer/updater gate.
- PR #1028 is merged terminal; its Session Analyzer v1 task is archived and PlayerCompanion follow-up tools remain separately owned work.

## Worker execution contract

Before assigning a selected item to an `IMPLEMENTATION_OWNER`, the `ARCHITECTURE_COORDINATOR` or existing specialized programme must provide a bounded task package containing:

1. exact repository and protected-base identity;
2. exact task/Issue, owned paths and overlap result;
3. accepted architecture/ADR/contracts/programmes to obey;
4. explicit in-scope and out-of-scope behavior;
5. exact source-of-truth inputs and forbidden assumptions, including producer evidence for every delivered current-state fact;
6. test/E2E/negative-path/rollback requirements;
7. merge and closeout gate;
8. explicit external-repository, production and owner-funded-service authority boundaries.

If any prerequisite is missing, the implementation package is not canonical `READY`; classify the exact candidate `BLOCKED` or `DECISION_REQUIRED` and continue canonical selection. If Codex is technically suitable, it may be recorded as an optional execution mode, but it may be invoked only after exact owner permission for that use/task. Without that permission, the same task is executed through another permitted mode.

## Parallelism rules

Parallel work is allowed only across independently selected, non-overlapping tasks/branches with distinct `owned_paths` and no unresolved dependency ordering.

Do not create parallel workers merely because several board rows are `OPEN` or `ARCHITECTURE_READY`, and never run multiple workers inside the same active PR or give two workers overlapping module/migration/route ownership.

## Definition of project completion

The portal completion project is terminal only when every workstream that the canonical programme or accepted owner decision marks launch-required is either:

- `DONE`, or
- explicitly `DEFERRED`/`REJECTED` by an accepted owner/product decision with no remaining launch dependency.

Additionally:

- core Account Center / Character Portfolio launch scope has a terminal implementation/defer/reject disposition;
- required LiveOps and PublicPortal Today launch surfaces have terminal dispositions, with authoritative producers and typed stale/unavailable semantics for every delivered current-state fact;
- Issue #489 has terminal dispositions for both GameCatalog/content and commerce findings rather than being closed by one side alone;
- Issue #490 has terminal dispositions for Platform API, OperationsObservability/repository readiness and PublicEdge evidence, with production claims tied to exact authorized environment identity;
- at least one complete PlayerCompanion workflow is merged and validated end-to-end;
- the launcher/client distribution path has the accepted provenance/update contract if it is part of launch;
- release-scope search/community/API items are implemented or explicitly deferred/rejected;
- no launch-critical security/privacy/audit finding remains unresolved;
- every merged implementation has terminal task/PR/Issue lifecycle and released ownership.

This definition does not require an owned forum, payments, microservices, World Hub, richer optional community reads or every TibiaPal-style tool unless the owner/canonical programme explicitly promotes them into launch scope.
