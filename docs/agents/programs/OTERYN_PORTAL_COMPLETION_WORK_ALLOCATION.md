# Oteryn Portal Completion — Work Allocation Project

```yaml
project_id: OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION
programme: OTERYN_PORTAL_COMPLETION
repository: Oteryn/Oteryn-Platform
trusted_base: main
status: ACTIVE
live_state_required: true
selection_authority: docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
completion_scope: docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json
architecture_authority: docs/architecture/ARCHITECTURE_AUTHORITY.md
delivery_plan: docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
player_companion: docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
production_authority: false
codex_standing_permission: false
execution_roles_are_model_agnostic: true
```

## Purpose

This document is the execution-allocation companion for `OTERYN_PORTAL_COMPLETION`. It maps portal capability maturity to model-agnostic execution roles, dependencies, optional Codex suitability and terminal outcomes **after the canonical programme has selected one live candidate**.

It is **not a scheduler, priority queue, architecture authority, completion-scope authority or source of current ownership**. `OTERYN_PORTAL_COMPLETION.md` is the sole live selector. `OTERYN_PORTAL_COMPLETION_SCOPE.json` records non-scheduling launch/completion dispositions. Accepted ADRs, focused architecture, contracts, specialized programmes, live Issues, active task records and exact PR/CI state remain authoritative for their own scopes.

Live repository state always wins over the maturity projection below.

## Control-plane relationship

Use the portal control plane in this order:

1. `ROADMAP.md` — global Platform phases and risk ordering.
2. `PORTAL_COMPLETENESS_ARCHITECTURE.md` — durable portal completion/release boundary.
3. `PORTAL_COMPLETION_DELIVERY_PLAN.md` — portal capability/dependency order.
4. `OTERYN_PORTAL_COMPLETION.md` — sole live selector.
5. **This file** — post-selection execution role/maturity mapping.
6. Selected Issue/task/branch/PR — one exact execution owner.

A delivery band or maturity value in this file cannot promote a candidate to selector `READY`, bypass an earlier canonical candidate, change a launch disposition, or override live ownership.

## Execution roles

### `ARCHITECTURE_COORDINATOR`

Applies and decomposes already accepted architecture for the selected portal capability. Primary responsibilities:

- bounded product/architecture decomposition inside accepted boundaries;
- dependency and sequencing reconciliation with the canonical programme;
- source-of-truth reconciliation;
- security/privacy/contract interpretation;
- implementation-package definition;
- production/public-edge evidence interpretation without protected mutation authority;
- PR review, CI diagnosis and final integration/closeout;
- deciding whether accepted architecture is sufficient for a bounded implementation owner.

This role may implement a bounded change directly when that is the smallest safe path.

**Boundary with `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`:** the coordinator does not create or silently decide a new durable module boundary, major dependency direction, trust boundary or other ADR-level product architecture merely to make a selected slice executable. A genuinely new/superseding durable decision is routed to Architecture Review / the architecture decision backlog first; the affected runtime candidate remains `DECISION_REQUIRED` or `BLOCKED` until accepted authority exists.

### `IMPLEMENTATION_OWNER`

Owns one selected bounded implementation slice from accepted scope through focused tests, exact-head self-review, required CI, E2E where applicable, PR closeout and task archival. Typical work includes:

- machine-readable inventories and validators;
- isolated module vertical slices;
- deterministic calculators/parsers;
- route/controller/view/test implementation inside an accepted boundary;
- focused refactors with known contracts;
- reproducible bug fixes with exact evidence.

The role is independent of model or tool. Chat/GitHub, a permitted runner or another permitted execution mode may be used. Codex suitability is recorded separately and never grants authorization.

### `OWNER_DECISION`

Required only for product/business choices or authorities that cannot be inferred safely, including:

- explicit launch-scope defer/reject/promotion decisions where accepted authority requires one;
- forum vs Discord if/when a durable owned community product is desired;
- payment/provider/legal activation;
- explicit production/protected-environment authorization;
- external/server-repository access or mutation authority;
- explicit permission for a particular owner-funded Codex/OpenAI invocation.

### `PROTECTED_ENV_OPERATOR`

Executes authorized production/protected-environment evidence collection or mutations that cannot be proven from repository state alone. This role operates only after exact owner/protected-environment authorization and must preserve environment identity, evidence and rollback boundaries.

## Codex suitability and authorization

The `Codex suitability` column is only a technical fit assessment for a bounded task package.

**Suitability is not authorization.** No Codex, OpenAI API or other owner-funded AI invocation is allowed unless the repository owner explicitly approves that exact use/task. Prior approval never becomes standing permission. When permission is absent, the selected task remains valid and must use another genuinely capable permitted mode or record the exact technical blocker.

## Capability maturity and execution matrix

The matrix contains **non-scheduling maturity projections**. It is not a current task board. `OTERYN_PORTAL_COMPLETION.md` recomputes live selector state from current protected `main`, Issues, tasks, PRs, dependencies and authority on every invocation.

Maturity values:

- `DONE` — accepted scope represented by the row is terminal;
- `IN_PROGRESS` — a deliberately refreshed observation found a live owner; current ownership must still be rechecked;
- `ARCHITECTURE_READY` — accepted architecture exists and implementation/evidence may remain; **never equivalent to selector `READY`**;
- `OPEN` — implementation/decomposition/evidence remains; does not imply eligibility;
- `CONDITIONAL` — cross-cutting or product work participates only when its named activation trigger is proven; not an independent standing queue entry;
- `BLOCKED` — a known dependency/authority/evidence prevents the stated outcome; selector verifies the exact live candidate;
- `DECISION_REQUIRED` — an owner/product/architecture decision is required;
- `DEFERRED` — intentionally outside current launch scope until an accepted trigger reactivates it;
- `REJECTED` — explicitly excluded by accepted authority.

`REQUIRED | CONDITIONAL | DEFERRED | REJECTED` launch/completion dispositions are stored separately in `OTERYN_PORTAL_COMPLETION_SCOPE.json`; they are not maturity states.

| Delivery band (non-scheduling) | Workstream | Maturity projection | Primary execution role | Codex suitability | Dependencies / next terminal outcome |
|---|---|---|---|---|---|
| P0 | Canonical module catalogue | `DONE` | `ARCHITECTURE_COORDINATOR` | No | Keep `MODULE_CATALOG.md` synchronized when accepted boundaries or implementation availability change. |
| P1 | Production topology and OperationsObservability repository evidence | `ARCHITECTURE_READY` / `BLOCKED` for direct production proof | `ARCHITECTURE_COORDINATOR` + `PROTECTED_ENV_OPERATOR` for live proof | Low | `OPERATIONS_OBSERVABILITY_ARCHITECTURE.md` owns repository/staging evidence semantics. Direct production topology/logging/metrics/alerts/on-call/backup/restore evidence requires exact protected-environment authority and identity. |
| P1 | PublicEdge protected-environment proof | `BLOCKED` for live proof | `ARCHITECTURE_COORDINATOR` + `PROTECTED_ENV_OPERATOR` | Low | DNS/TLS/HSTS/WAF/private-origin/deployed-identity/smoke evidence requires explicit protected-environment authority. Repository-safe preparation is independent. |
| P1/P2 | Core Account Center / Character Portfolio | `ARCHITECTURE_READY` | `ARCHITECTURE_COORDINATOR`, then `IMPLEMENTATION_OWNER` per vertical slice | High after contract/dependency proof | Native delete/restore and rename remain gated by accepted Character Authority semantics and current external evidence; world transfer additionally requires its accepted product/operation decision. Architecture maturity bypasses no gate. |
| P2 | LiveOps | `ARCHITECTURE_READY` | `ARCHITECTURE_COORDINATOR`, then `IMPLEMENTATION_OWNER` per slice | High when sliced | `LIVEOPS_ARCHITECTURE.md` is canonical. Runtime `WorldStatus + configured Maintenance` requires exact authoritative runtime-status producer evidence; `ServerSave` remains separately gated by producer/applicability/time/freshness semantics. `MODULE_CATALOG.md` remains `LiveOps | PLANNED` until executable capability is merged. |
| P2 | PublicPortal Today | `ARCHITECTURE_READY` | `ARCHITECTURE_COORDINATOR`, then `IMPLEMENTATION_OWNER` | High after source readiness | Public Today consumes source-owned projections and preserves partial/freshness states. Private Today participates only when its completion-scope trigger and ADR 0032 owner-private cache/isolation gates are satisfied. |
| P2 | Federated Search & Discovery | `ARCHITECTURE_READY` | `IMPLEMENTATION_OWNER` after coordinator confirms dependency cleanup | High | Remove Announcements/Events reverse `PublicPortal` dependency first, then implement source-owned providers and PublicPortal orchestration with partial-failure semantics. Live ownership may already exist; always recheck. |
| P2 | Client Distribution hardening | `ARCHITECTURE_READY` | `IMPLEMENTATION_OWNER` after architecture closeout | High for Platform-only slice | ADR 0035 / `CLIENT_DISTRIBUTION_ARCHITECTURE.md` define the Platform TUF boundary. External updater implementation, signing infrastructure/operations and production activation are separate authorities. Live ownership may already exist; always recheck. |
| P2 | Wiki expected-content inventory | `DONE` | `ARCHITECTURE_COORDINATOR` | No | Preserve validators and provenance gates. |
| P2 | Game Catalog authoritative expected inventory | `OPEN` | `ARCHITECTURE_COORDINATOR`, then `IMPLEMENTATION_OWNER` for Platform-only slices | High for bounded Platform slices | Route specialized decomposition through `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md`, but current Platform governance controls external/server repository authority. |
| P2 | Wiki/media failure investigation | `DONE` | `ARCHITECTURE_COORDINATOR` | No | Do not reopen without new exact reproduction/evidence. |
| P2 | Platform API | `DEFERRED` | `ARCHITECTURE_COORDINATOR` when a named consumer trigger exists | No active handoff | ADR 0036 explicitly defers the general API until an approved named consumer/use case satisfies activation criteria. |
| Cross-cutting | Multi-world / rulesets / seasons dimensions | `CONDITIONAL` | `ARCHITECTURE_COORDINATOR`; `OTERYN_PLATFORM_ARCHITECTURE_REVIEW` if a new durable decision is required | Low initially | Not a standing queue item. Activate a bounded decision package only when a selected capability would otherwise introduce an unresolved irreversible world/profile/ruleset/catalog/season assumption. Otherwise preserve dimensions inside that capability. |
| P2/P3 | PlayerCompanion foundation | `DONE` | `ARCHITECTURE_COORDINATOR` | No | Hunt Session Analyzer v1 is the first terminal complete workflow; current terminal state is resolved live, not inferred from this row. |
| P2/P3 | PlayerCompanion follow-up tools | `CONDITIONAL` | `IMPLEMENTATION_OWNER` per promoted independent vertical slice | High | Hunt Finder → Equipment Explorer → Character Build Planner → Charm/Perk/Proficiency Planner → Quest/Access Tracker → EXP/Training calculators → validated shareable builds only when individually promoted and dependency-ready. |
| P3 | PublicGameData / richer community read surfaces | `DEFERRED` | `ARCHITECTURE_COORDINATOR`, then `IMPLEMENTATION_OWNER` when promoted | High when promoted | Houses, kill statistics, richer leaderboards, guild wars/activity history are product inventory inputs, not automatic launch requirements. |
| P3 | Forum vs Discord | `DEFERRED` | `OWNER_DECISION` | No | Default direction remains Discord + existing Support; build an owned forum only when durable discussion/moderation need is proven. |
| P3 | Read scaling / projections / dedicated search index | `DEFERRED` | `ARCHITECTURE_COORDINATOR` | Medium after telemetry | Keep modular-monolith/direct-read-model approach until measured thresholds justify derived infrastructure. |
| P3 | World Hub / richer community composition | `DEFERRED` | `ARCHITECTURE_COORDINATOR`, then bounded `IMPLEMENTATION_OWNER` slices | Medium | Activate only when multiple worlds/profiles and authoritative LiveOps/community inputs justify it; never routing/admission authority. |
| P3 | Commerce capability disposition | `DECISION_REQUIRED` / conditional on product promotion | `OWNER_DECISION` + `ARCHITECTURE_COORDINATOR` | Low before decision | Missing product/payment capabilities require an explicit implement/defer/reject disposition before any promoted commerce scope can claim completion. |
| P3 | Commerce production activation | `DEFERRED` / `BLOCKED` until independent gates | `OWNER_DECISION` + `ARCHITECTURE_COORDINATOR` + `PROTECTED_ENV_OPERATOR` | Only bounded post-decision tasks | Provider, legal/tax, signed webhooks, reconciliation, refunds/chargebacks, entitlement freshness and protected production authority are separate mandatory gates. |

## Selection contract

This companion must not create a second priority algorithm.

1. `OTERYN_PORTAL_COMPLETION.md` resolves completion-scope applicability and live state, then classifies each canonical selection-order entry as `TERMINAL | OWNED | BLOCKED | DECISION_REQUIRED | READY`.
2. It selects the first unowned `READY` candidate and stops traversal; exact reasons for skipped earlier entries are persisted.
3. Only after selection is the candidate mapped here to choose a model-agnostic execution role and permitted execution mode.
4. A specialized programme or Issue owner named by the selected row controls detailed decomposition and acceptance without changing canonical portal order or repository authority.
5. Delivery bands and maturity states never authorize skipping an earlier canonical `READY` item.
6. `ARCHITECTURE_READY`, `OPEN` and `CONDITIONAL` never promote themselves to selector `READY`.
7. If live state or accepted architecture conflicts with a row, stop using that row until this companion is reconciled; never rewrite live truth to fit the matrix.

## Specialized routing and overlap rules

- Audit repair findings use `OTERYN_PLATFORM_REMEDIATION`; this allocation does not create duplicate repair ownership. Historical closed Issues are not candidates.
- New durable architecture decisions route through `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`; the Portal coordinator applies accepted decisions but does not silently become a competing architecture-decision programme.
- Game Catalog implementation reuses `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md` for specialized decomposition only. That programme cannot grant server/game repository access; current Platform governance and exact owner authorization remain controlling.
- Historical `RETAIN`/`RECOVERY`, historical branch preservation/deletion and steady-state branch hygiene are repository-governance concerns under ADR 0037/0039, not portal capability workstreams.
- Shared Issue identifiers may contain several concerns; route each concern to its canonical owner rather than allowing one terminal slice to close unrelated findings.
- A live authoritative PR is reused/fixed/rebased when safe; duplicate/superseded attempts are closed only with concrete evidence and preserved provenance.

## Worker execution contract

Before assigning a selected item to an `IMPLEMENTATION_OWNER`, the coordinator or existing specialized programme must provide a bounded package containing:

1. exact repository and protected-base identity;
2. exact task/Issue, owned paths and overlap result;
3. accepted architecture/ADR/contracts/programmes to obey;
4. completion-scope disposition and, for `CONDITIONAL`, the proven activation trigger;
5. explicit in-scope and out-of-scope behavior;
6. exact source-of-truth inputs and forbidden assumptions, including producer evidence for every delivered current-state fact;
7. multi-world/profile/ruleset/catalog/season applicability where relevant;
8. test/E2E/negative-path/rollback requirements;
9. merge and closeout gate;
10. explicit external-repository, production and owner-funded-service authority boundaries.

If any prerequisite is missing, the implementation package is not canonical `READY`; classify the exact candidate `BLOCKED` or `DECISION_REQUIRED` and continue canonical selection. Codex suitability may be recorded as optional technical fit, but invocation still requires exact owner permission for owner-funded use.

## Parallelism rules

Parallelism is global across **already selected/claimed independent tasks**, not inside one selected PR:

- one worker owns one Issue/task/branch/PR;
- the canonical selector chooses at most one new candidate for a worker/invocation entry;
- independent already-owned tasks may run concurrently when `owned_paths` do not overlap and no dependency ordering exists;
- never run multiple workers inside one active PR/worktree or give two owners overlapping module/migration/route paths;
- `OWNED` is a collision-avoidance state, not permission to share another task's branch.

## Definition of project completion

`docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json` is the machine-readable non-scheduling projection of launch/completion dispositions. The portal completion project is globally terminal only when all of the following are proven from current durable evidence for the exact named release scope:

- every `REQUIRED` item has an accepted terminal implement/defer/reject disposition;
- every `CONDITIONAL` item whose activation trigger is proven for current launch scope has a terminal disposition;
- the exact canonical per-capability inventory is resolved from current architecture/owner authority;
- every capability has exactly one owner-approved `IMPLEMENT | DEFER | REJECT` record containing stable `capability_id`, `owner`, `rationale`, `outcome` and `authority_evidence`;
- no broad workstream/family disposition substitutes for the required per-capability records;
- missing, duplicate, conflicting or ambiguous capability-disposition evidence is absent; otherwise the programme remains `DECISION_REQUIRED` and global completion is false;
- inactive conditional and deferred work is not falsely described as implemented;
- no launch-critical security/privacy/audit finding remains unresolved;
- every production/go-live claim is tied to direct authorized evidence for the exact deployed identity;
- the accepted PlayerCompanion foundation workflow remains terminal; follow-up **implementation** is required only for capabilities whose owner-approved outcome is `IMPLEMENT`, while every canonical capability still requires its product disposition record;
- every merged implementation has terminal task/PR/Issue/source-branch lifecycle and released ownership.

`IMPLEMENT` is product disposition only; it does not prove implementation, E2E, CI, production readiness or activation. The scope projection never proves current status, ownership, production evidence, selector `READY` or per-capability disposition evidence; all of those are resolved from their current authorities.
